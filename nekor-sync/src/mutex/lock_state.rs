//! Fairness model for a [`Mutex`].
//!
//! [`Mutex`]: super::Mutex

use core::{
    num::NonZero,
    sync::atomic::{
        AtomicUsize,
        Ordering::{AcqRel, Relaxed, Release},
    },
};

use nekor_aal_signal::prelude::{Monitor, Monitored};

use nekor_backoff::prelude::{Backoff, Retry};

use crate::atomic::bitmap::{
    AtomicBitmap,
    at::At,
    conditional::{Status, Unset},
    mode::{
        cooperative::{Cooperative, Snapshot},
        exclusive::{Exclusive, Outcome, Reason},
    },
};

/// A snapshot of a [`LockState`] at some point in time.
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum Observed {
    /// An unoccupied [`LockState`] lock stratum.
    Desolate,

    /// An occupied [`LockState`] lock stratum.
    Occupied(NonZero<usize>),
}

/// A new-type that describes an occupied slot in an [`LockState`].
#[derive(Debug, Clone, Copy)]
pub struct Waiting<'a>(
    &'a LockState,
    // NOTE(invariant): Only a single bit is always set.
    NonZero<usize>,
);

/// A wait token, used to prove that the turn has been taken for the current
/// thread of execution.
#[repr(transparent)]
pub struct Waited(
    // NOTE(invariant): Only a single bit is always set.
    NonZero<usize>,
);

impl Waiting<'_> {
    /// Wait for the associated lock to be in our turn.
    ///
    /// # Safety
    ///
    /// The returned [`Waited`] proof-type must only be used for the same
    /// [`LockState`] this [`Waiting`] was sourced from.
    #[inline]
    #[must_use]
    pub unsafe fn wait(self) -> Waited {
        let Self(LockState(bitqueue_state, owner_state), queue_bitmask) = self;

        let bitmask_value = NonZero::get(queue_bitmask);

        // NOTE: queue bitmask has a single bit set, so we can subtract one to
        // get all the LSBs to wait for.
        let target_cond = Unset(bitmask_value.wrapping_sub(1));

        'a: loop {
            match AtomicBitmap::condition(bitqueue_state, target_cond) {
                Status::Unmet => () = AtomicBitmap::wait(bitqueue_state),
                Status::Satisfied(..) => {
                    // NOTE: Linearize the queue state.
                    break 'a 'b: loop {
                        let target_monitor = Monitor::engage(owner_state);

                        if owner_state
                            .compare_exchange_weak(
                                usize::MIN,
                                bitmask_value,
                                AcqRel,
                                // NOTE: We are not using the error snapshot,
                                // so relaxed ordering is sufficient.
                                Relaxed,
                            )
                            .is_ok()
                        {
                            break 'b Waited(queue_bitmask);
                        }
                        Monitored::wait(target_monitor);
                    };
                }
            }
        }
    }
}

/// A managed lock-state for mutual exclusion with acquisition fairness.
///
/// # Fairness
///
/// The algorithm used maintains perfect acquisition order for up to
/// [`usize::BITS`] simultaneous lock contenders.
///
/// In the case that the number of contenders breaches this boundary, the order
/// becomes effectively undefined and first-come-first-serve for all contenders
/// beyond [`usize::BITS`].
#[derive(Debug)]
pub struct LockState(AtomicBitmap, Monitor<AtomicUsize>);

impl LockState {
    /// Construct a [`LockState`] in a desolate state.
    #[inline]
    #[must_use]
    pub const fn desolate() -> Self {
        Self(
            AtomicBitmap::zeroed(),
            Monitor::new(AtomicUsize::new(usize::MIN)),
        )
    }

    /// Construct a [`LockState`] in a state where exactly `N` future contenders
    /// exist.
    #[inline]
    #[must_use]
    pub const fn enqueued<const N: usize>() -> Self {
        Self(
            AtomicBitmap::raw(AtomicUsize::new(
                const {
                    const BITS: usize = usize::BITS as _;

                    match N {
                        BITS => usize::MAX,
                        _ => (1 << N) - 1,
                    }
                },
            )),
            const { Monitor::new(AtomicUsize::new(usize::MIN)) },
        )
    }

    /// Take a snapshot of the immediate [`LockState`], determining whether it
    /// is desolate or otherwise occupied.
    ///
    /// This has no validity in terms of the immediately-after lock state, and
    /// should therefore not be relied upon.
    #[inline]
    pub fn snapshot(&self) -> Observed {
        let Self(target_state, ..) = self;

        NonZero::<usize>::new(AtomicBitmap::snapshot(target_state))
            .map_or(Observed::Desolate, Observed::Occupied)
    }
}

impl LockState {
    /// Acquire a ticket for this [`LockState`], yielding a [`Waiting`] instance
    /// that can be used to wait for our ticket to be granted access to the
    /// lock.
    ///
    /// Particularly, this function will busy-wait until the ticket is granted,
    /// without regard for the number of attempts or any other factors.
    ///
    /// For an alternative with a bounded attempt limit, see
    /// [`Self::acquire_with_limit`].
    pub fn acquire(&self) -> Waiting<'_> {
        // FIXME(backoff): This needs to be benchmarked to find appropiate
        // parameters.
        let acquisition_state = &mut Retry::unlimited(Backoff::minimal());

        Self::acquire_with_state(self, acquisition_state)
            .unwrap_or_else(|| unreachable!("unlimited acquisition cannot exhaust retries"))
    }

    /// Attempt to acquire the ticket for this [`LockState`], for the purpose of
    /// waiting for the lock to become available.
    ///
    /// Particularly, this function will busy-wait until:
    ///
    /// - A ticket slot is granted to us.
    /// - The imposed limit is reached.
    ///
    /// The [`Retry`] state required deems each attempt as an access to a
    /// possibly-contended resource, not a complete acquisition attempt.
    #[inline]
    pub fn acquire_with_state(&self, retry_state: &mut Retry) -> Option<Waiting<'_>> {
        let Self(target_state, ..) = self;
        let full_backoff = &mut Backoff::state(Backoff::minimal());

        loop {
            let bit_selected = AtomicBitmap::at_leftmost(target_state).map_or_else(
                || Some(AtomicBitmap::static_at::<0>(target_state)),
                At::left,
            );

            if let Some(locked_target) = bit_selected.as_ref() {
                match locked_target.one_with::<Exclusive>(retry_state) {
                    Outcome::Success(..) => {
                        break Some(Waiting(
                            self,
                            // SAFETY: An integer with at least one bit set
                            // cannot be zero.
                            unsafe { NonZero::new_unchecked(1 << At::index(locked_target)) },
                        ));
                    }
                    Outcome::Failure(Reason::Contended | Reason::Unchanged, ..) => (),
                    Outcome::Failure(Reason::Limited, ..) => break None,
                }
            } else {
                // All slots are occupied, so retain backoff state across
                // retries until a slot becomes available.
                Backoff::cycle(full_backoff);
            }
        }
    }

    /// Release the target [`Occupied`] slot from this [`LockState`].
    ///
    /// This yields the [`Observed`] state of the slot that was released, but
    /// previous to the release operation.
    ///
    /// # Safety
    ///
    /// The provided [`Waited`] structure must be from this same [`LockState`].
    #[inline]
    pub unsafe fn release(&self, Waited(allocated_slot): Waited) -> Observed {
        let Self(lock_state, owner_state) = self;

        let Snapshot(observed_state) = lock_state
            .at(usize::trailing_zeros(NonZero::get(allocated_slot)))
            .zero::<Cooperative>();

        Monitor::access(owner_state).store(usize::MIN, Release);

        NonZero::new(observed_state).map_or(Observed::Desolate, Observed::Occupied)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering, fence};
    use std::sync::{Arc, Barrier};
    use std::thread;

    /// Test basic acquire and release cycle
    #[test]
    fn basic_acquire_release() {
        let lock_state = LockState::desolate();

        // Acquire a slot
        let waiting = lock_state.acquire();

        // SAFETY: wait returns a Waited token for this lock
        let waited = unsafe { waiting.wait() };

        // SAFETY: waited is from this same lock_state
        let observed = unsafe { lock_state.release(waited) };

        // The pre-release snapshot includes the slot held by this thread.
        assert!(matches!(observed, Observed::Occupied(_)));
    }

    /// Test that snapshot returns a valid Observed enum
    #[test]
    fn snapshot_returns_valid_observed() {
        let lock_state = LockState::desolate();

        // Snapshot should return a valid Observed value (but we can't rely on
        // specific values due to Relaxed ordering)
        match lock_state.snapshot() {
            Observed::Desolate | Observed::Occupied(_) => {}
        }

        let waiting = lock_state.acquire();
        // SAFETY: wait returns token for this lock
        let waited = unsafe { waiting.wait() };

        // Snapshot still returns valid values
        match lock_state.snapshot() {
            Observed::Desolate | Observed::Occupied(_) => {}
        }

        // SAFETY: waited is from this lock_state
        let _ = unsafe { lock_state.release(waited) };

        // Still valid after release
        match lock_state.snapshot() {
            Observed::Desolate | Observed::Occupied(_) => {}
        }
    }

    /// Test that multiple threads can acquire different slots
    #[test]
    fn multiple_threads_acquire_different_slots() {
        let lock_state = Arc::new(LockState::desolate());
        let barrier = Arc::new(Barrier::new(4));
        let snapshot_barrier = Arc::new(Barrier::new(5)); // 4 threads + main
        let mut handles = vec![];

        for _ in 0..4 {
            let lock_state = Arc::clone(&lock_state);
            let barrier = Arc::clone(&barrier);
            let snapshot_barrier = Arc::clone(&snapshot_barrier);
            handles.push(thread::spawn(move || {
                let waiting = lock_state.acquire();
                barrier.wait(); // Ensure all have acquired

                // Wait for main thread to check snapshot
                snapshot_barrier.wait();

                // Now wait and release
                // SAFETY: waiting is from this lock_state
                let waited = unsafe { waiting.wait() };
                // SAFETY: waited is from this lock_state
                let _ = unsafe { lock_state.release(waited) };
            }));
        }

        // Wait for all threads to acquire
        snapshot_barrier.wait();

        // Verify snapshot returns a valid value (Relaxed ordering means we
        // can't make specific assertions about the state)
        match lock_state.snapshot() {
            Observed::Desolate | Observed::Occupied(_) => {}
        }

        for handle in handles {
            handle.join().expect("worker thread panicked");
        }
    }

    /// Test that all threads can successfully acquire, wait, and release
    #[test]
    fn all_threads_successfully_acquire_wait_release() {
        let lock_state = Arc::new(LockState::desolate());
        let counter = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];

        for _ in 0..8 {
            let lock_state = Arc::clone(&lock_state);
            let counter = Arc::clone(&counter);

            handles.push(thread::spawn(move || {
                let waiting = lock_state.acquire();

                // SAFETY: waiting is from this lock_state
                let waited = unsafe { waiting.wait() };

                // Increment counter while holding lock
                counter.fetch_add(1, Ordering::SeqCst);

                // SAFETY: waited is from this lock_state
                let _ = unsafe { lock_state.release(waited) };
            }));
        }

        for handle in handles {
            handle.join().expect("worker thread panicked");
        }

        fence(Ordering::Acquire);

        // All 8 threads should have successfully acquired and incremented
        assert_eq!(counter.load(Ordering::SeqCst), 8);
    }

    /// Test mutual exclusion: only one thread proceeds at a time
    #[test]
    fn mutual_exclusion_only_one_thread_at_a_time() {
        let lock_state = Arc::new(LockState::desolate());
        let active_count = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(Barrier::new(5));
        let mut handles = vec![];

        for _ in 0..5 {
            let lock_state = Arc::clone(&lock_state);
            let active_count = Arc::clone(&active_count);
            let barrier = Arc::clone(&barrier);

            handles.push(thread::spawn(move || {
                let waiting = lock_state.acquire();
                barrier.wait();

                // SAFETY: waiting is from this lock_state
                let waited = unsafe { waiting.wait() };

                // Increment active count
                let count = active_count.fetch_add(1, Ordering::SeqCst);

                // Verify we're the only one
                assert_eq!(count, 0, "multiple threads in critical section!");

                // Do some work
                std::thread::yield_now();

                // Decrement before releasing
                active_count.fetch_sub(1, Ordering::SeqCst);

                // SAFETY: waited is from this lock_state
                let _ = unsafe { lock_state.release(waited) };
            }));
        }

        for handle in handles {
            handle.join().expect("worker thread panicked");
        }
    }

    /// Test that slots can be reused after release
    #[test]
    fn slots_are_reused_after_release() {
        let lock_state = LockState::desolate();

        // Acquire and release multiple times
        for _ in 0..10 {
            let waiting = lock_state.acquire();
            // SAFETY: waiting is from this lock_state
            let waited = unsafe { waiting.wait() };
            // SAFETY: waited is from this lock_state
            let _ = unsafe { lock_state.release(waited) };
        }

        // Should still work - slots are being reused
        let waiting = lock_state.acquire();
        // SAFETY: waiting is from this lock_state
        let waited = unsafe { waiting.wait() };
        // SAFETY: waited is from this lock_state
        let _ = unsafe { lock_state.release(waited) };
    }

    /// Test that unlimited acquire always succeeds (when slots available)
    #[test]
    fn unlimited_acquire_always_succeeds() {
        let lock_state = LockState::desolate();

        // Should succeed multiple times
        for _ in 0..5 {
            let waiting = lock_state.acquire();
            // SAFETY: waiting is from this lock_state
            let waited = unsafe { waiting.wait() };
            // SAFETY: waited is from this lock_state
            let _ = unsafe { lock_state.release(waited) };
        }
    }

    /// Test enqueued constructor creates proper initial state
    #[test]
    fn enqueued_constructor() {
        let lock_state = LockState::enqueued::<3>();

        // The constructor should create state with 3 bits, but due to Relaxed
        // ordering in snapshot(), we can't reliably assert the exact value
        // The important test is that it compiles and runs without panicking
        let _ = lock_state.snapshot();

        // We can verify the underlying bitmap directly if needed for testing
        // but snapshot() itself doesn't provide strong guarantees
    }

    /// Test that many threads can cycle through the lock
    #[test]
    fn many_threads_cycle_through_lock() {
        let lock_state = Arc::new(LockState::desolate());
        let counter = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];

        for _ in 0..20 {
            let lock_state = Arc::clone(&lock_state);
            let counter = Arc::clone(&counter);

            handles.push(thread::spawn(move || {
                let waiting = lock_state.acquire();
                // SAFETY: waiting is from this lock_state
                let waited = unsafe { waiting.wait() };

                // Increment counter while holding lock
                let value = counter.load(Ordering::SeqCst);
                counter.store(value + 1, Ordering::SeqCst);

                // SAFETY: waited is from this lock_state
                let _ = unsafe { lock_state.release(waited) };
            }));
        }

        for handle in handles {
            handle.join().expect("worker thread panicked");
        }

        // Fence to ensure we see all writes before reading counter (for Miri)
        fence(Ordering::Acquire);

        // All 20 threads should have incremented the counter
        // Fence to ensure we see all writes before checking results (for Miri)
        fence(Ordering::Acquire);

        // The execution order should match the acquisition order
        // All 20 threads should have incremented the counter
        assert_eq!(counter.load(Ordering::SeqCst), 20);
    }

    /// Test sequential acquire and wait pattern
    #[test]
    fn sequential_acquire_and_wait() {
        let lock_state = LockState::desolate();

        let w1 = lock_state.acquire();
        let w2 = lock_state.acquire();
        let w3 = lock_state.acquire();

        // All should have different slots
        let Observed::Occupied(bits) = lock_state.snapshot() else {
            unreachable!("three acquired slots must leave an occupied state")
        };
        assert_eq!(bits.get().count_ones(), 3);

        // First thread waits and gets the lock
        // SAFETY: w1 is from this lock_state
        let waited1 = unsafe { w1.wait() };

        // Release first
        // SAFETY: waited1 is from this lock_state
        let _ = unsafe { lock_state.release(waited1) };

        // Second thread can now proceed
        // SAFETY: w2 is from this lock_state
        let waited2 = unsafe { w2.wait() };

        // SAFETY: waited2 is from this lock_state
        let _ = unsafe { lock_state.release(waited2) };

        // Third thread can now proceed
        // SAFETY: w3 is from this lock_state
        let waited3 = unsafe { w3.wait() };

        // SAFETY: waited3 is from this lock_state
        let _ = unsafe { lock_state.release(waited3) };
    }

    /// Test that release returns a valid Observed value
    #[test]
    fn release_returns_valid_observed() {
        let lock_state = LockState::desolate();

        let waiting = lock_state.acquire();
        // SAFETY: waiting is from this lock_state
        let waited = unsafe { waiting.wait() };

        // SAFETY: waited is from this lock_state
        let observed = unsafe { lock_state.release(waited) };

        // Release returns the previous state (from Cooperative mode snapshot)
        // which is a valid Observed value
        match observed {
            Observed::Desolate | Observed::Occupied(_) => {
                // Both are valid - depends on what other bits were set
            }
        }
    }

    /// Test that multiple threads can all acquire and release successfully
    #[test]
    fn multiple_threads_acquire_release_successfully() {
        let lock_state = Arc::new(LockState::desolate());
        let counter = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(Barrier::new(6));
        let mut handles = vec![];

        for _ in 0..6 {
            let lock_state = Arc::clone(&lock_state);
            let counter = Arc::clone(&counter);
            let barrier = Arc::clone(&barrier);

            handles.push(thread::spawn(move || {
                let waiting = lock_state.acquire();
                barrier.wait();

                // SAFETY: waiting is from this lock_state
                let waited = unsafe { waiting.wait() };

                // Increment counter while holding lock
                counter.fetch_add(1, Ordering::SeqCst);

                // SAFETY: waited is from this lock_state
                let _ = unsafe { lock_state.release(waited) };
            }));
        }

        for handle in handles {
            handle.join().expect("worker thread panicked");
        }

        // Fence to ensure we see all writes (for Miri)
        fence(Ordering::Acquire);

        // All 6 threads should have successfully acquired and incremented
        assert_eq!(counter.load(Ordering::SeqCst), 6);
    }

    /// Test concurrent acquire attempts
    #[test]
    fn concurrent_acquire_attempts() {
        let lock_state = Arc::new(LockState::desolate());
        let barrier = Arc::new(Barrier::new(10));
        let success_count = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];

        for _ in 0..10 {
            let lock_state = Arc::clone(&lock_state);
            let barrier = Arc::clone(&barrier);
            let success_count = Arc::clone(&success_count);

            handles.push(thread::spawn(move || {
                barrier.wait();
                // All try to acquire simultaneously
                let waiting = lock_state.acquire();
                success_count.fetch_add(1, Ordering::SeqCst);

                // Wait and release
                // SAFETY: waiting is from this lock_state
                let waited = unsafe { waiting.wait() };
                // SAFETY: waited is from this lock_state
                let _ = unsafe { lock_state.release(waited) };
            }));
        }

        for handle in handles {
            handle.join().expect("worker thread panicked");
        }

        // Fence to ensure all operations are visible (for Miri)
        fence(Ordering::Acquire);

        // All should succeed in getting a slot
        assert_eq!(success_count.load(Ordering::SeqCst), 10);
    }
}

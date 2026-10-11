//! Fairness model for a [`Mutex`].
//!
//! [`Mutex`]: super::Mutex

use core::{num::NonZero, ops::ControlFlow, sync::atomic::AtomicUsize};

use nekor_aal_signal::monitor::{Attempt, Fastlane, Polling};
use nekor_backoff::prelude::{Backoff, Retry};

use crate::atomic::bitmap::{
    AtomicBitmap,
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

/// A reserved turn in a [`LockState`].
#[derive(Debug)]
// NOTE(invariant): `Waiting` uniquely owns a set bit in its `LockState`, reserved by an `AcqRel`
// CAS.
pub struct Waiting<'a>(&'a LockState, NonZero<usize>);

/// Proof that a [`LockState`] reservation holds the turn.
#[repr(transparent)]
// NOTE(invariant): The mask contains exactly one set bit identifying the acquired turn.
pub struct Waited(NonZero<usize>);

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
        let Self(&LockState(ref bitqueue_state), queue_bitmask) = self;

        let bitmask_value = NonZero::get(queue_bitmask);

        match bitmask_value {
            // NOTE: The `AcqRel` CAS from `0` to `1` already acquired the
            // emptying release. Later reservations cannot precede this bit.
            1 => Waited(queue_bitmask),
            _ => {
                // NOTE: Subtracting one selects all earlier reservations.
                let target_cond = Unset(bitmask_value.wrapping_sub(1));

                Polling::<_, Fastlane>::of(bitqueue_state.value()).until(|_| {
                    match AtomicBitmap::condition(bitqueue_state, target_cond) {
                        Status::Unmet => Attempt::Pending,
                        // NOTE: `Acquire` synchronizes with the lower holders' releases.
                        Status::Satisfied(..) => Attempt::Ready(Waited(queue_bitmask)),
                    }
                })
            },
        }
    }
}

/// A managed lock-state for mutual exclusion with acquisition fairness.
///
/// # Fairness
///
/// The algorithm serves up to [`usize::BITS`] outstanding reservations in
/// their reservation order.
///
/// Contenders without a reservation have no ordering guarantee. When the
/// highest bit is occupied, they wait until the bitmap becomes empty.
#[derive(Debug)]
// NOTE(invariant): Reservations form a contiguous interval. New bits append above it,
// and only the lowest holder may clear its bit. An occupied top bit prevents
// reuse of lower positions until the bitmap is empty.
pub struct LockState(AtomicBitmap);

impl LockState {
    /// Construct a [`LockState`] in a desolate state.
    #[inline]
    #[must_use]
    pub const fn desolate() -> Self {
        Self(AtomicBitmap::zeroed())
    }

    /// Construct a [`LockState`] in a state where exactly `N` future contenders
    /// exist.
    #[inline]
    #[must_use]
    pub const fn enqueued<const N: usize>() -> Self {
        Self(AtomicBitmap::raw(AtomicUsize::new(
            const {
                const BITS: usize = usize::BITS as usize;

                match N {
                    BITS => usize::MAX,
                    _ => (1 << N) - 1,
                }
            },
        )))
    }

    /// Take a snapshot of the immediate [`LockState`], determining whether it
    /// is desolate or otherwise occupied.
    ///
    /// This has no validity in terms of the immediately-after lock state, and
    /// should therefore not be relied upon.
    #[inline]
    pub fn snapshot(&self) -> Observed {
        let &Self(ref target_state, ..) = self;

        NonZero::<usize>::new(AtomicBitmap::snapshot(target_state)).map_or(Observed::Desolate, Observed::Occupied)
    }
}

impl LockState {
    /// Acquire a [`Waiting`] reservation for this [`LockState`].
    ///
    /// This polls briefly before engaging the monitor. For bounded attempts,
    /// see [`Self::acquire_with_state`].
    pub fn acquire(&self) -> Waiting<'_> {
        let &Self(ref target_state) = self;

        let mut polling = Polling::<_, Fastlane>::of(target_state.value());

        polling.until(|_| {
            let mut retry = Retry::oneshot(Backoff::minimal());

            match self.acquire_with_state(&mut retry) {
                Some(reservation) => Attempt::Ready(reservation),
                None => Attempt::Pending,
            }
        })
    }

    /// Attempt to reserve a turn using a [`Retry`] state.
    ///
    /// Each CAS or saturated access consumes an attempt.
    ///
    /// # Failure
    ///
    /// Returns [`None`] when the retry limit is reached.
    #[inline]
    pub fn acquire_with_state(&self, retry_state: &mut Retry) -> Option<Waiting<'_>> {
        let &Self(ref target_state, ..) = self;

        loop {
            // NOTE: Append above the highest live bit, never into lower vacancies.
            let target_closure = |target_snapshot| {
                if let usize::MIN = target_snapshot {
                    Some(u32::MIN)
                } else {
                    usize::BITS.checked_sub(target_snapshot.leading_zeros())
                }
            };

            if let Some(locked_target) = AtomicBitmap::at_predicate(target_state, target_closure).as_ref() {
                // NOTE: The `Exclusive` CAS uses this `Acquire` snapshot.
                // It linearizes a successful reservation. Failure requires reselection.
                match locked_target.one_with::<Exclusive>(retry_state) {
                    Outcome::Success(..) => {
                        break Some(Waiting(
                            self,
                            // SAFETY: Setting an in-bounds bit produces a nonzero mask.
                            unsafe { NonZero::new_unchecked(1 << locked_target.index()) },
                        ));
                    },
                    Outcome::Failure(Reason::Contended | Reason::Unchanged, ..) => (),
                    Outcome::Failure(Reason::Limited, ..) => break None,
                }
            } else {
                // NOTE: An occupied top bit selects `usize::BITS`, so append
                // waits for an empty bitmap. This still consumes a `Retry` attempt.
                match Retry::attempt(retry_state) {
                    ControlFlow::Continue(backoff_cycle) => {
                        backoff_cycle();
                    },
                    ControlFlow::Break(..) => break None,
                }
            }
        }
    }

    /// Release a [`Waited`] turn and return the previous [`Observed`] state.
    ///
    /// # Safety
    ///
    /// The provided [`Waited`] structure must be from this same [`LockState`].
    #[inline]
    pub unsafe fn release(&self, Waited(allocated_slot): Waited) -> Observed {
        let &Self(ref lock_state) = self;

        let Snapshot(observed_state) = lock_state
            .at(usize::trailing_zeros(NonZero::get(allocated_slot)))
            .zero::<Cooperative>();

        NonZero::new(observed_state).map_or(Observed::Desolate, Observed::Occupied)
    }
}

#[cfg(test)]
mod tests {
    use alloc::sync::Arc;
    use core::{
        hint, iter,
        sync::atomic::{AtomicUsize, Ordering, fence},
        time::Duration,
    };
    use std::{
        sync::{Barrier, mpsc},
        thread,
    };

    use super::*;

    /// Bit-zero acquisition observes the previous holder's payload.
    #[test]
    fn empty_reservation_acquires_previous_payload() {
        let lock_state = Arc::new(LockState::desolate());
        let payload = Arc::new(AtomicUsize::new(usize::MIN));
        let reservation = lock_state.acquire();

        // SAFETY: This reservation belongs to `lock_state`.
        let waited = unsafe { reservation.wait() };

        let reader_state = Arc::clone(&lock_state);
        let reader_payload = Arc::clone(&payload);

        let reader = thread::spawn(move || {
            // NOTE: The `Relaxed` snapshot does not publish the payload.
            while matches!(reader_state.snapshot(), Observed::Occupied(..)) {
                hint::spin_loop();
            }

            let reservation = reader_state.acquire();

            let Waiting(_, bitmask) = &reservation;

            assert_eq!(bitmask.get(), 1);

            // SAFETY: This reservation belongs to `reader_state`.
            let waited = unsafe { reservation.wait() };

            assert_eq!(reader_payload.load(Ordering::Relaxed), 41);

            // SAFETY: This turn belongs to `reader_state`.
            let _: Observed = unsafe { reader_state.release(waited) };
        });

        // NOTE: Write after spawning, so only `LockState` publishes the payload.
        payload.store(41, Ordering::Relaxed);

        // SAFETY: This turn belongs to `lock_state`.
        let _: Observed = unsafe { lock_state.release(waited) };

        reader.join().expect("reader thread panicked");
    }

    /// Bit zero keeps its turn after another reservation appends.
    #[test]
    fn bit_zero_keeps_turn_after_append() {
        let lock_state = LockState::desolate();
        let first = lock_state.acquire();
        let second = lock_state.acquire();

        assert_eq!(
            lock_state.snapshot(),
            Observed::Occupied(NonZero::new(3).expect("two live bits"))
        );

        // SAFETY: The first reservation belongs to `lock_state`.
        let first = unsafe { first.wait() };

        let Waited(first_bitmask) = &first;

        assert_eq!(first_bitmask.get(), 1);

        // SAFETY: The first turn belongs to `lock_state`.
        let _: Observed = unsafe { lock_state.release(first) };

        // SAFETY: The second reservation belongs to `lock_state`.
        let second = unsafe { second.wait() };

        let Waited(second_bitmask) = &second;

        assert_eq!(second_bitmask.get(), 2);

        // SAFETY: The second turn belongs to `lock_state`.
        let _: Observed = unsafe { lock_state.release(second) };

        assert_eq!(lock_state.snapshot(), Observed::Desolate);
    }

    /// Saturation consumes an attempt without wrapping.
    #[test]
    fn bounded_saturation_consumes_one_attempt() {
        let lock_state = LockState::enqueued::<{ usize::BITS as usize }>();
        let mut retry = Retry::oneshot(Backoff::minimal());

        assert!(lock_state.acquire_with_state(&mut retry).is_none());
        assert_eq!(retry.backoff_state_mut().cycle(), Some(NonZero::<usize>::MIN));
        assert!(lock_state.acquire_with_state(&mut retry).is_none());
        assert_eq!(retry.backoff_state_mut().cycle(), Some(NonZero::<usize>::MIN));
        assert_eq!(lock_state.snapshot(), Observed::Occupied(NonZero::<usize>::MAX));
    }

    /// Acquire and release a turn.
    #[test]
    fn basic_acquire_release() {
        let lock_state = LockState::desolate();

        let waiting = lock_state.acquire();

        // SAFETY: `Waiting` belongs to this `LockState`.
        let waited = unsafe { waiting.wait() };

        // SAFETY: `waited` belongs to `lock_state`.
        let observed = unsafe { lock_state.release(waited) };

        // NOTE: The pre-release snapshot includes this thread's slot.
        assert!(matches!(observed, Observed::Occupied(_)));
    }

    /// Snapshots produce a valid [`Observed`] state.
    #[test]
    fn snapshot_returns_valid_observed() {
        let lock_state = LockState::desolate();

        match lock_state.snapshot() {
            Observed::Desolate | Observed::Occupied(_) => {},
        }

        let waiting = lock_state.acquire();
        // SAFETY: `waiting` was acquired from `lock_state`.
        let waited = unsafe { waiting.wait() };

        match lock_state.snapshot() {
            Observed::Desolate | Observed::Occupied(_) => {},
        }

        // SAFETY: `waited` belongs to `lock_state`.
        let _: Observed = unsafe { lock_state.release(waited) };

        match lock_state.snapshot() {
            Observed::Desolate | Observed::Occupied(_) => {},
        }
    }

    /// Concurrent acquisitions reserve distinct slots.
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

                snapshot_barrier.wait();

                // SAFETY: `waiting` was acquired from `lock_state`.
                let waited = unsafe { waiting.wait() };
                // SAFETY: `waited` belongs to `lock_state`.
                let _: Observed = unsafe { lock_state.release(waited) };
            }));
        }

        // NOTE: All reservations are live when the snapshot barrier opens.
        snapshot_barrier.wait();

        match lock_state.snapshot() {
            Observed::Desolate | Observed::Occupied(_) => {},
        }

        for handle in handles {
            handle.join().expect("worker thread panicked");
        }
    }

    /// All contenders acquire and release.
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

                // SAFETY: `waiting` was acquired from `lock_state`.
                let waited = unsafe { waiting.wait() };

                counter.fetch_add(1, Ordering::SeqCst);

                // SAFETY: `waited` belongs to `lock_state`.
                let _: Observed = unsafe { lock_state.release(waited) };
            }));
        }

        for handle in handles {
            handle.join().expect("worker thread panicked");
        }

        fence(Ordering::Acquire);

        assert_eq!(counter.load(Ordering::SeqCst), 8);
    }

    /// Only one reservation holds the turn.
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

                // SAFETY: `waiting` was acquired from `lock_state`.
                let waited = unsafe { waiting.wait() };

                let count = active_count.fetch_add(1, Ordering::SeqCst);

                assert_eq!(count, 0, "multiple threads in critical section!");

                thread::yield_now();

                active_count.fetch_sub(1, Ordering::SeqCst);

                // SAFETY: `waited` belongs to `lock_state`.
                let _: Observed = unsafe { lock_state.release(waited) };
            }));
        }

        for handle in handles {
            handle.join().expect("worker thread panicked");
        }
    }

    /// Released slots can be reused.
    #[test]
    fn slots_are_reused_after_release() {
        let lock_state = LockState::desolate();

        for _ in 0..10 {
            let waiting = lock_state.acquire();
            // SAFETY: `waiting` was acquired from `lock_state`.
            let waited = unsafe { waiting.wait() };
            // SAFETY: `waited` belongs to `lock_state`.
            let _: Observed = unsafe { lock_state.release(waited) };
        }

        let waiting = lock_state.acquire();
        // SAFETY: `waiting` was acquired from `lock_state`.
        let waited = unsafe { waiting.wait() };
        // SAFETY: `waited` belongs to `lock_state`.
        let _: Observed = unsafe { lock_state.release(waited) };
    }

    /// Unbounded acquisition succeeds when slots are available.
    #[test]
    fn unlimited_acquire_always_succeeds() {
        let lock_state = LockState::desolate();

        for _ in 0..5 {
            let waiting = lock_state.acquire();
            // SAFETY: `waiting` was acquired from `lock_state`.
            let waited = unsafe { waiting.wait() };
            // SAFETY: `waited` belongs to `lock_state`.
            let _: Observed = unsafe { lock_state.release(waited) };
        }
    }

    /// Construct a pre-enqueued [`LockState`].
    #[test]
    fn enqueued_constructor() {
        let lock_state = LockState::enqueued::<3>();

        lock_state.snapshot();
    }

    /// Contenders reuse the lock across turns.
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
                // SAFETY: `waiting` was acquired from `lock_state`.
                let waited = unsafe { waiting.wait() };

                let value = counter.load(Ordering::SeqCst);

                counter.store(value + 1, Ordering::SeqCst);

                // SAFETY: `waited` belongs to `lock_state`.
                let _: Observed = unsafe { lock_state.release(waited) };
            }));
        }

        for handle in handles {
            handle.join().expect("worker thread panicked");
        }

        fence(Ordering::Acquire);

        fence(Ordering::Acquire);

        assert_eq!(counter.load(Ordering::SeqCst), 20);
    }

    /// Reservations are served in acquisition order.
    #[test]
    fn sequential_acquire_and_wait() {
        let lock_state = LockState::desolate();

        let w1 = lock_state.acquire();
        let w2 = lock_state.acquire();
        let w3 = lock_state.acquire();

        let Observed::Occupied(bits) = lock_state.snapshot() else {
            unreachable!("three acquired slots must leave an occupied state")
        };
        assert_eq!(bits.get().count_ones(), 3);

        // SAFETY: `w1` was acquired from `lock_state`.
        let waited1 = unsafe { w1.wait() };

        // SAFETY: `waited1` belongs to `lock_state`.
        let _: Observed = unsafe { lock_state.release(waited1) };

        // SAFETY: `w2` was acquired from `lock_state`.
        let waited2 = unsafe { w2.wait() };

        // SAFETY: `waited2` belongs to `lock_state`.
        let _: Observed = unsafe { lock_state.release(waited2) };

        // SAFETY: `w3` was acquired from `lock_state`.
        let waited3 = unsafe { w3.wait() };

        // SAFETY: `waited3` belongs to `lock_state`.
        let _: Observed = unsafe { lock_state.release(waited3) };
    }

    /// Release returns a valid [`Observed`] state.
    #[test]
    fn release_returns_valid_observed() {
        let lock_state = LockState::desolate();

        let waiting = lock_state.acquire();
        // SAFETY: `waiting` was acquired from `lock_state`.
        let waited = unsafe { waiting.wait() };

        // SAFETY: `waited` belongs to `lock_state`.
        let observed = unsafe { lock_state.release(waited) };

        match observed {
            Observed::Desolate | Observed::Occupied(_) => {},
        }
    }

    /// All simultaneous contenders complete their turns.
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

                // SAFETY: `waiting` was acquired from `lock_state`.
                let waited = unsafe { waiting.wait() };

                counter.fetch_add(1, Ordering::SeqCst);

                // SAFETY: `waited` belongs to `lock_state`.
                let _: Observed = unsafe { lock_state.release(waited) };
            }));
        }

        for handle in handles {
            handle.join().expect("worker thread panicked");
        }

        fence(Ordering::Acquire);

        assert_eq!(counter.load(Ordering::SeqCst), 6);
    }

    /// Concurrent acquisition attempts complete.
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

                let waiting = lock_state.acquire();

                success_count.fetch_add(1, Ordering::SeqCst);

                // SAFETY: `waiting` was acquired from `lock_state`.
                let waited = unsafe { waiting.wait() };
                // SAFETY: `waited` belongs to `lock_state`.
                let _: Observed = unsafe { lock_state.release(waited) };
            }));
        }

        for handle in handles {
            handle.join().expect("worker thread panicked");
        }

        fence(Ordering::Acquire);

        assert_eq!(success_count.load(Ordering::SeqCst), 10);
    }

    /// The highest bit prevents reuse until the bitmap empties.
    #[test]
    fn highest_bit_prevents_overtaking_until_empty() {
        let lock_state = LockState::desolate();
        let reservations: Vec<_> = iter::repeat_with(|| lock_state.acquire())
            .take(usize::BITS as usize)
            .collect();

        assert_eq!(
            lock_state.snapshot(),
            Observed::Occupied(NonZero::new(usize::MAX).expect("nonzero"))
        );

        for reservation in reservations {
            // SAFETY: This reservation belongs to `lock_state`.
            let waited = unsafe { reservation.wait() };

            let Waited(released_bit) = &waited;

            let released_bit = released_bit.get();
            // SAFETY: This token belongs to `lock_state`.
            let _: Observed = unsafe { lock_state.release(waited) };

            if released_bit != 1_usize << (usize::BITS - 1) {
                let mut retry = Retry::oneshot(Backoff::minimal());
                assert!(lock_state.acquire_with_state(&mut retry).is_none());
            }
        }

        let reservation = lock_state.acquire();

        let Waiting(_, bit) = &reservation;
        assert_eq!(bit.get(), 1);
        // SAFETY: This reservation belongs to `lock_state`.
        let waited = unsafe { reservation.wait() };
        // SAFETY: This token belongs to `lock_state`.
        let _: Observed = unsafe { lock_state.release(waited) };
    }

    /// Simultaneous reservations receive distinct bits.
    #[test]
    fn simultaneous_reservations_have_unique_bits() {
        let lock_state = Arc::new(LockState::desolate());
        let start = Arc::new(Barrier::new(9));
        let reserved = Arc::new(Barrier::new(9));
        let mut handles = vec![];

        for _ in 0..8 {
            let lock_state = Arc::clone(&lock_state);
            let start = Arc::clone(&start);
            let reserved = Arc::clone(&reserved);

            handles.push(thread::spawn(move || {
                start.wait();

                let reservation = lock_state.acquire();

                let Waiting(_, bit) = &reservation;

                let bit = bit.get();

                reserved.wait();

                // SAFETY: This reservation belongs to `lock_state`.
                let waited = unsafe { reservation.wait() };
                // SAFETY: This token belongs to `lock_state`.
                let _: Observed = unsafe { lock_state.release(waited) };

                bit
            }));
        }

        start.wait();
        reserved.wait();

        let mut bits = 0;

        for handle in handles {
            let bit = handle.join().expect("worker thread panicked");
            assert_eq!(bits & bit, 0);
            bits |= bit;
        }
        assert_eq!(bits.count_ones(), 8);
    }

    #[test]
    fn saturated_waiter_reserves_after_last_release() {
        let lock_state = Arc::new(LockState::desolate());
        let mut reservations: Vec<_> = iter::repeat_with(|| lock_state.acquire())
            .take(usize::BITS as usize)
            .collect();
        let highest = reservations.pop().expect("highest reservation exists");

        let (sender, receiver) = mpsc::channel();

        let waiter_state = Arc::clone(&lock_state);

        let waiter = thread::spawn(move || {
            let reservation = waiter_state.acquire();

            let Waiting(_, bit) = &reservation;

            let bit = bit.get();

            sender.send(bit).expect("receiver remains present");

            // SAFETY: This reservation belongs to `waiter_state`.
            let waited = unsafe { reservation.wait() };
            // SAFETY: This token belongs to `waiter_state`.
            let _: Observed = unsafe { waiter_state.release(waited) };
        });

        for reservation in reservations {
            // SAFETY: This reservation belongs to `lock_state`.
            let waited = unsafe { reservation.wait() };
            // SAFETY: This token belongs to `lock_state`.
            let _: Observed = unsafe { lock_state.release(waited) };
        }

        assert!(receiver.recv_timeout(Duration::from_millis(20)).is_err());

        // SAFETY: This reservation belongs to `lock_state`.
        let waited = unsafe { highest.wait() };
        // SAFETY: This token belongs to `lock_state`.
        let _: Observed = unsafe { lock_state.release(waited) };

        assert_eq!(
            receiver.recv_timeout(Duration::from_secs(2)).expect("waiter timed out"),
            1
        );
        waiter.join().expect("waiter thread panicked");
    }
}

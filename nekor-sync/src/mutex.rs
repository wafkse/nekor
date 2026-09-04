//! A lock-free, fair mutual exclusion mechanism for critical sections.

pub mod guard;

pub mod lock_state;

use core::cell::UnsafeCell;

use nekor_aal_cache::padded::CachePadded;
#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
use nekor_aal_cache::padded::CachePadded;

use crate::mutex::{
    guard::{Guard, RawMutexGuard},
    lock_state::{LockState, Waiting},
};

/// A blocking mutual exclusion primitive for shared program state.
///
/// # Fairness
///
/// The algorithm used maintains perfect acquisition order for up to
/// [`usize::BITS`] simultaneous lock contenders.
///
/// In the case that the number of contenders breaches this boundary, the order
/// becomes effectively undefined and first-come-first-serve for all consecutive
/// contenders beyond [`usize::BITS`].
///
/// # Non-poisoning
///
/// If a holding thread panics, it could either:
///
/// - Remain locked away forever.
/// - Be eventually unlocked, but with the ability to present itself or the underlying `T` with
///   broken invariants.
#[derive(Debug)]
pub struct Mutex<T> {
    /// The monitored [`usize`]-sized bitset used to manage the lock state.
    lock_state: CachePadded<LockState>,

    /// The value that is granted exclusive access to.
    target_value: UnsafeCell<T>,
}

impl<T> Mutex<T> {
    /// Construct a new [`Mutex`] from the target value.
    #[inline]
    pub const fn new(target_value: T) -> Self {
        let target_value = UnsafeCell::new(target_value);

        let lock_state = CachePadded::new(LockState::desolate());

        Self {
            lock_state,
            target_value,
        }
    }

    /// Access the underlying value through a mutable reference.
    #[inline]
    pub const fn get_mut(&mut self) -> &mut T {
        let &mut Self {
            ref mut target_value, ..
        } = self;

        target_value.get_mut()
    }

    /// Consume this [`Mutex`], yielding the underlying data.
    #[inline]
    pub fn into_inner(self) -> T {
        let Self { target_value, .. } = self;

        target_value.into_inner()
    }

    /// Lock the [`Mutex`], blocking as-needed.
    #[inline]
    pub fn lock(&self) -> Guard<'_, T> {
        let &Self { ref lock_state, .. } = self;

        // SAFETY: The `Waited` token is used with the same `LockState`.
        let waited_state = unsafe { Waiting::wait(LockState::acquire(lock_state)) };

        // SAFETY: The `Occupied` was acquired from the same `Mutex`.
        unsafe {
            let target_value = RawMutexGuard::new(self, waited_state);

            Guard::new(target_value)
        }
    }
}

/// Require `T` to be [`Send`] for [`Mutex`] to be [`Send`], as `T` can be
/// destructured from [`Mutex`].
// SAFETY: Moving a `Mutex<T>` between threads moves its contained `T`, which
// requires `T: Send`.
unsafe impl<T: Send> Send for Mutex<T> {}

/// [`Mutex`] provides mutable access to `T` to one thread at a time.
///
/// However, this requires that `T` be [`Send`] for it to be safe.
// SAFETY: The lock state grants mutable access to the `UnsafeCell<T>` to one
// thread at a time, and sharing the mutex therefore requires only `T: Send`.
unsafe impl<T: Send> Sync for Mutex<T> {}

#[cfg(test)]
mod tests {
    use alloc::sync::Arc;
    use core::sync::atomic::{AtomicUsize, Ordering, fence};
    use std::{sync::Barrier, thread};

    use super::*;

    /// Test basic lock and unlock cycle
    #[test]
    fn basic_lock_unlock() {
        let mutex = Mutex::new(42);
        let guard = mutex.lock();
        assert_eq!(*guard, 42);
        drop(guard);

        // Should be able to lock again
        let guard = mutex.lock();
        assert_eq!(*guard, 42);
    }

    /// Test that we can modify the protected value
    #[test]
    fn modify_protected_value() {
        let mutex = Mutex::new(0);
        {
            let mut guard = mutex.lock();
            *guard = 100;
        }
        assert_eq!(*mutex.lock(), 100);
    }

    /// Test `get_mut` provides mutable access without locking
    #[test]
    fn get_mut_access() {
        let mut mutex = Mutex::new(10);
        *mutex.get_mut() = 20;
        assert_eq!(*mutex.lock(), 20);
    }

    /// Test `into_inner` consumes mutex and returns value
    #[test]
    fn into_inner_consumes() {
        let mutex = Mutex::new(String::from("test"));
        let value = mutex.into_inner();
        assert_eq!(value, "test");
    }

    /// Test THE CRITICAL GUARANTEE: mutual exclusion
    /// Only one thread can hold the lock at a time
    #[test]
    fn mutual_exclusion_guarantee() {
        let mutex = Arc::new(Mutex::new(()));
        let in_critical_section = Arc::new(AtomicUsize::new(0));
        let violations = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];

        for _ in 0..10 {
            let mutex = Arc::clone(&mutex);
            let in_critical_section = Arc::clone(&in_critical_section);
            let violations = Arc::clone(&violations);

            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    let _guard = mutex.lock();

                    // Enter critical section
                    let count = in_critical_section.fetch_add(1, Ordering::SeqCst);

                    // If count was non-zero, another thread was in critical section
                    if count != 0 {
                        violations.fetch_add(1, Ordering::SeqCst);
                    }

                    // Simulate work
                    thread::yield_now();

                    // Exit critical section
                    in_critical_section.fetch_sub(1, Ordering::SeqCst);
                }
            }));
        }

        for handle in handles {
            handle.join().expect("worker thread panicked");
        }

        fence(Ordering::Acquire);
        assert_eq!(
            violations.load(Ordering::SeqCst),
            0,
            "mutual exclusion violated - multiple threads in critical section"
        );
    }

    /// Test that concurrent increments are properly synchronized
    #[test]
    fn concurrent_increments_synchronized() {
        let mutex = Arc::new(Mutex::new(0));
        let mut handles = vec![];

        for _ in 0..10 {
            let mutex = Arc::clone(&mutex);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    let mut guard = mutex.lock();
                    *guard += 1;
                }
            }));
        }

        for handle in handles {
            handle.join().expect("worker thread panicked");
        }

        fence(Ordering::Acquire);
        assert_eq!(*mutex.lock(), 1000, "all increments should be visible");
    }

    /// Test that no data races occur with Vec operations
    #[test]
    fn no_data_races_with_vec() {
        let mutex = Arc::new(Mutex::new(vec![0_u8; 100]));
        let mut handles = vec![];

        for thread_id in 0..8 {
            let mutex = Arc::clone(&mutex);
            handles.push(thread::spawn(move || {
                for _ in 0..50 {
                    let mut guard = mutex.lock();

                    // Write thread_id to all elements
                    for elem in guard.iter_mut() {
                        *elem = thread_id;
                    }

                    // Verify consistency - all elements should be the same
                    let first = guard[0];
                    for elem in guard.iter() {
                        assert_eq!(*elem, first, "data race detected - inconsistent array state");
                    }
                }
            }));
        }

        for handle in handles {
            handle.join().expect("worker thread panicked");
        }
    }

    /// Test that all threads can successfully acquire and use the lock
    #[test]
    fn all_threads_acquire_successfully() {
        let mutex = Arc::new(Mutex::new(Vec::new()));
        let barrier = Arc::new(Barrier::new(8));
        let mut handles = vec![];

        for thread_id in 0..8 {
            let mutex = Arc::clone(&mutex);
            let barrier = Arc::clone(&barrier);

            handles.push(thread::spawn(move || {
                barrier.wait();
                let mut guard = mutex.lock();
                guard.push(thread_id);
            }));
        }

        for handle in handles {
            handle.join().expect("worker thread panicked");
        }

        fence(Ordering::Acquire);
        let guard = mutex.lock();

        // All 8 threads should have successfully acquired the lock
        assert_eq!(guard.len(), 8, "all threads should acquire lock");
    }

    /// Test that guard properly releases lock on drop
    #[test]
    fn guard_releases_on_drop() {
        let mutex = Mutex::new(0);

        {
            let mut guard = mutex.lock();
            *guard = 42;
            // Guard drops here
        }

        // If lock wasn't released, this would deadlock
        let guard = mutex.lock();
        assert_eq!(*guard, 42);
    }

    /// Test concurrent mixed increment/decrement operations
    #[test]
    fn concurrent_mixed_operations() {
        let mutex = Arc::new(Mutex::new(0_i32));
        let mut handles = vec![];

        // 5 threads incrementing
        for _ in 0..5 {
            let mutex = Arc::clone(&mutex);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    let mut guard = mutex.lock();
                    *guard += 1;
                }
            }));
        }

        // 5 threads decrementing
        for _ in 0..5 {
            let mutex = Arc::clone(&mutex);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    let mut guard = mutex.lock();
                    *guard -= 1;
                }
            }));
        }

        for handle in handles {
            handle.join().expect("worker thread panicked");
        }

        fence(Ordering::Acquire);
        assert_eq!(*mutex.lock(), 0, "balanced operations should result in 0");
    }

    /// Test high contention with many threads
    #[test]
    fn high_contention_many_threads() {
        let mutex = Arc::new(Mutex::new(0));
        let num_threads = 20;
        let iterations = 50;
        let mut handles = vec![];

        for _ in 0..num_threads {
            let mutex = Arc::clone(&mutex);
            handles.push(thread::spawn(move || {
                for _ in 0..iterations {
                    let mut guard = mutex.lock();
                    *guard += 1;
                }
            }));
        }

        for handle in handles {
            handle.join().expect("worker thread panicked");
        }

        fence(Ordering::Acquire);
        assert_eq!(
            *mutex.lock(),
            num_threads * iterations,
            "all increments must be visible under high contention"
        );
    }

    /// Test that very short critical sections work correctly
    #[test]
    fn short_critical_sections() {
        let mutex = Arc::new(Mutex::new(0));
        let mut handles = vec![];

        for _ in 0..50 {
            let mutex = Arc::clone(&mutex);
            handles.push(thread::spawn(move || {
                for _ in 0..20 {
                    // Very short - just increment
                    *mutex.lock() += 1;
                }
            }));
        }

        for handle in handles {
            handle.join().expect("worker thread panicked");
        }

        fence(Ordering::Acquire);
        assert_eq!(*mutex.lock(), 1000);
    }

    /// Test with complex types (Vec operations)
    #[test]
    fn complex_type_vec_operations() {
        let mutex = Mutex::new(Vec::new());

        {
            let mut guard = mutex.lock();
            guard.push(1);
            guard.push(2);
            guard.extend_from_slice(&[3, 4, 5]);
        }

        let guard = mutex.lock();
        assert_eq!(&*guard, &[1, 2, 3, 4, 5]);
    }

    /// Test with Option type
    #[test]
    fn option_type_operations() {
        let mutex = Mutex::new(Some(42));

        {
            let mut guard = mutex.lock();
            let value = guard.take();
            assert_eq!(value, Some(42));
        }

        let guard = mutex.lock();
        assert_eq!(*guard, None);
    }

    /// Test sequential lock/unlock cycles
    #[test]
    fn sequential_lock_unlock_cycles() {
        let mutex = Mutex::new(0);

        for i in 0..100 {
            let mut guard = mutex.lock();
            *guard = i;
            drop(guard);

            let guard = mutex.lock();
            assert_eq!(*guard, i);
        }
    }

    /// Test that exceeding `usize::BITS` contenders still works
    #[test]
    fn exceeding_bit_limit_contenders() {
        let mutex = Arc::new(Mutex::new(0));
        let num_threads = (usize::BITS as usize) + 10;
        let mut handles = vec![];

        for _ in 0..num_threads {
            let mutex = Arc::clone(&mutex);
            handles.push(thread::spawn(move || {
                let mut guard = mutex.lock();
                *guard += 1;
            }));
        }

        for handle in handles {
            handle.join().expect("worker thread panicked");
        }

        fence(Ordering::Acquire);
        assert_eq!(
            *mutex.lock(),
            num_threads,
            "should handle more than usize::BITS contenders"
        );
    }

    /// Test that panicking while holding lock doesn't cause UB
    /// (Note: the lock may remain locked, which is documented behavior)
    #[test]
    #[should_panic(expected = "intentional panic")]
    fn panic_while_holding_lock() {
        use core::hint::black_box;

        let mutex = Mutex::new(0);
        let _guard = mutex.lock();
        assert!(black_box(false), "intentional panic");
    }

    /// Test Send + Sync bounds are correct
    #[test]
    fn send_sync_bounds() {
        fn assert_send<T>()
        where
            T: Send,
        {
        }
        fn assert_sync<T>()
        where
            T: Sync,
        {
        }

        assert_send::<Mutex<i32>>();
        assert_sync::<Mutex<i32>>();
    }

    /// Test Debug implementation exists
    #[test]
    fn debug_implementation() {
        let mutex = Mutex::new(42);
        let debug_str = format!("{mutex:?}");
        assert!(debug_str.contains("Mutex"));
    }

    /// Test with zero-sized type
    #[test]
    fn zero_sized_type() {
        struct ZeroSized;
        let mutex = Mutex::new(ZeroSized);
        let _guard = mutex.lock();
    }

    /// Test guard deref operations
    #[test]
    fn guard_deref_operations() {
        let mutex = Mutex::new(String::from("hello"));

        let guard = mutex.lock();
        assert_eq!(guard.len(), 5);
        assert_eq!(&*guard, "hello");
    }

    /// Test guard `deref_mut` operations
    #[test]
    fn guard_deref_mut_operations() {
        let mutex = Mutex::new(String::from("hello"));

        {
            let mut guard = mutex.lock();
            guard.push_str(" world");
        }

        let guard = mutex.lock();
        assert_eq!(&*guard, "hello world");
    }

    /// Test that concurrent writes are properly serialized
    #[test]
    fn concurrent_writes_serialized() {
        let mutex = Arc::new(Mutex::new(Vec::new()));
        let mut handles = vec![];

        for thread_id in 0..10 {
            let mutex = Arc::clone(&mutex);
            handles.push(thread::spawn(move || {
                for i in 0..10 {
                    let mut guard = mutex.lock();
                    guard.push((thread_id, i));
                }
            }));
        }

        for handle in handles {
            handle.join().expect("worker thread panicked");
        }

        fence(Ordering::Acquire);
        let guard = mutex.lock();
        assert_eq!(guard.len(), 100, "all writes should be visible");
    }

    /// Test alternating read/write patterns
    #[test]
    fn alternating_read_write_patterns() {
        let mutex = Arc::new(Mutex::new(0));
        let mut handles = vec![];

        for i in 0..10 {
            let mutex = Arc::clone(&mutex);
            handles.push(thread::spawn(move || {
                for _ in 0..20 {
                    let mut guard = mutex.lock();
                    if i % 2 == 0 {
                        *guard += 1;
                    } else {
                        let _: usize = *guard; // Just read
                    }
                }
            }));
        }

        for handle in handles {
            handle.join().expect("worker thread panicked");
        }

        fence(Ordering::Acquire);
        // 5 threads writing 20 times each
        assert_eq!(*mutex.lock(), 100);
    }

    /// Test that guard `value()` method works
    #[test]
    fn guard_value_method() {
        let mutex = Mutex::new(String::from("test"));
        let guard = mutex.lock();
        let value_ref: &String = guard.value();
        assert_eq!(value_ref, "test");
    }

    /// Test that guard `value_mut()` method works
    #[test]
    fn guard_value_mut_method() {
        let mutex = Mutex::new(String::from("test"));
        let mut guard = mutex.lock();
        let value_ref = guard.value_mut();
        value_ref.push_str("ing");
        assert_eq!(value_ref, "testing");
    }

    /// Test concurrent access with `HashMap`
    #[test]
    fn concurrent_hashmap_operations() {
        use std::collections::HashMap;

        let mutex = Arc::new(Mutex::new(HashMap::new()));
        let mut handles = vec![];

        for i in 0..10 {
            let mutex = Arc::clone(&mutex);
            handles.push(thread::spawn(move || {
                let mut guard = mutex.lock();
                guard.insert(i, i * 10);
            }));
        }

        for handle in handles {
            handle.join().expect("worker thread panicked");
        }

        fence(Ordering::Acquire);
        let guard = mutex.lock();
        assert_eq!(guard.len(), 10);
        assert_eq!(guard.get(&5), Some(&50));
    }
}

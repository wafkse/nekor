//! The *cooperative* [`AtomicBitmap`] engagement [`Mode`].
//!
//! [`AtomicBitmap`]: crate::atomic::bitmap::AtomicBitmap
//! [`Mode`]: crate::atomic::bitmap::mode::Mode

use core::{
    fmt::{self, Display},
    ops::Not,
    sync::atomic::Ordering::AcqRel,
};

use crate::atomic::bitmap::{
    at::{At, Disengage},
    mode::{InMode, Mode},
};

/// A snapshot of the shared [`AtomicUsize`].
///
/// This is simply informative and does not provide any guarantees about the
/// state of the bitmap.
///
/// [`AtomicUsize`]: core::sync::atomic::AtomicUsize
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct Snapshot(pub usize);

impl fmt::Debug for Snapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let &Self(target_value) = self;

        f.write_fmt(format_args!("Snapshot({target_value:b})"))
    }
}

/// A cooperative engagement [`Mode`].
// NOTE: Auto-derive all the following traits to allow types with generic
// [`Mode`] parameters to derive them too without relaxing generic bounds.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Cooperative {}

// SAFETY: The (cooperative) guarantees of this engagement mode are known and
// well-documented, furthermore, the implementation of each finalizer is atomic
// and thread-safe.
unsafe impl Mode for Cooperative {
    type Signal = Snapshot;
    type State = ();

    #[inline]
    fn zero_with(at: At<'_>, _target_state: &mut Self::State, in_mode: &InMode) -> Self::Signal {
        let Disengage(target_value, _, bit_index) = At::disengage(at, in_mode);

        let snapshot_value = target_value.fetch_and(1_usize.wrapping_shl(bit_index).not(), AcqRel);

        Snapshot(snapshot_value)
    }

    #[inline]
    fn one_with(at: At<'_>, _target_state: &mut Self::State, in_mode: &InMode) -> Self::Signal {
        let Disengage(target_value, _, bit_index) = At::disengage(at, in_mode);

        let snapshot_value = target_value.fetch_or(1_usize.wrapping_shl(bit_index), AcqRel);

        Snapshot(snapshot_value)
    }
}

impl Display for Cooperative {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        // NOTE: `Cooperative` is uninhabited
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use core::sync::atomic::{Ordering, fence};

    use super::*;
    use crate::atomic::bitmap::AtomicBitmap;

    /// Test that `one()` actually sets a bit from 0 to 1
    #[test]
    fn one_sets_bit() {
        let bitmap = AtomicBitmap::zeroed();

        let _: Snapshot = bitmap.at(5).one::<Cooperative>();

        assert_eq!(bitmap.snapshot() & (1 << 5), 1 << 5, "bit 5 should be set");
    }

    /// Test that `zero()` actually clears a bit from 1 to 0
    #[test]
    fn zero_clears_bit() {
        let bitmap = AtomicBitmap::zeroed();
        let _: Snapshot = bitmap.at(7).one::<Cooperative>();

        let _: Snapshot = bitmap.at(7).zero::<Cooperative>();

        assert_eq!(bitmap.snapshot() & (1 << 7), 0, "bit 7 should be cleared");
    }

    /// Test that the snapshot reflects the state BEFORE the operation
    #[test]
    fn snapshot_returns_previous_state() {
        let bitmap = AtomicBitmap::zeroed();

        // Set bit 3 - should return previous state (all zeros)
        let Snapshot(before_set) = bitmap.at(3).one::<Cooperative>();
        assert_eq!(before_set & (1 << 3), 0, "snapshot should show bit was 0");

        // Clear bit 3 - should return previous state (bit 3 was 1)
        let Snapshot(before_clear) = bitmap.at(3).zero::<Cooperative>();
        assert_eq!(before_clear & (1 << 3), 1 << 3, "snapshot should show bit was 1");
    }

    /// Test that operations work on already-targeted states (idempotent in
    /// effect)
    #[test]
    fn operations_on_already_targeted_state() {
        let bitmap = AtomicBitmap::zeroed();

        // Setting an already-clear bit
        let _: Snapshot = bitmap.at(2).zero::<Cooperative>();
        assert_eq!(bitmap.snapshot() & (1 << 2), 0, "bit should remain clear");

        // Clearing an already-set bit
        let _: Snapshot = bitmap.at(4).one::<Cooperative>();
        let _: Snapshot = bitmap.at(4).one::<Cooperative>();
        assert_eq!(bitmap.snapshot() & (1 << 4), 1 << 4, "bit should remain set");
    }

    /// Test that multiple concurrent threads can all successfully set different
    /// bits
    #[test]
    fn concurrent_operations_on_different_bits_all_succeed() {
        use alloc::sync::Arc;
        use std::thread;

        let bitmap = Arc::new(AtomicBitmap::zeroed());
        let mut handles = vec![];

        // Spawn threads to set different bits concurrently
        for i in 0..16 {
            let bitmap_clone = Arc::clone(&bitmap);
            handles.push(thread::spawn(move || {
                let _: Snapshot = bitmap_clone.at(i).one::<Cooperative>();
            }));
        }

        for handle in handles {
            handle.join().expect("worker thread panicked");
        }

        // Fence to ensure we see all writes before snapshot (for Miri)
        fence(Ordering::Acquire);

        // All bits should be set - cooperative mode never fails
        let expected = (1_usize << 16) - 1;
        assert_eq!(
            bitmap.snapshot() & expected,
            expected,
            "all bits 0-15 should be set by concurrent operations"
        );
    }

    /// Test that multiple threads operating on the SAME bit all succeed
    /// This is the key difference from Exclusive mode - no failures occur
    #[test]
    fn concurrent_operations_on_same_bit_all_succeed() {
        use alloc::sync::Arc;
        use std::{sync::Barrier, thread};

        let bitmap = Arc::new(AtomicBitmap::zeroed());
        let barrier = Arc::new(Barrier::new(10));
        let mut handles = vec![];

        // Spawn 10 threads that all try to set bit 5 at the same time
        for _ in 0..10 {
            let bitmap_clone = Arc::clone(&bitmap);
            let barrier_clone = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                barrier_clone.wait(); // Synchronize start
                let _: Snapshot = bitmap_clone.at(5).one::<Cooperative>();
                // In cooperative mode, this never fails - no return value to
                // check
            }));
        }

        for handle in handles {
            handle.join().expect("worker thread panicked");
        }

        // Fence to ensure we see all writes before snapshot (for Miri)
        fence(Ordering::Acquire);

        // Bit should definitely be set
        assert_eq!(bitmap.snapshot() & (1 << 5), 1 << 5, "bit 5 should be set");
    }

    /// Test that concurrent set and clear operations all complete without
    /// failure
    #[test]
    fn concurrent_mixed_operations_all_succeed() {
        use alloc::sync::Arc;
        use std::{sync::Barrier, thread};

        let bitmap = Arc::new(AtomicBitmap::zeroed());
        let barrier = Arc::new(Barrier::new(20));
        let mut handles = vec![];

        // 10 threads setting bit 10, 10 threads clearing bit 10
        for i in 0..20 {
            let bitmap_clone = Arc::clone(&bitmap);
            let barrier_clone = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                barrier_clone.wait();
                if i % 2 == 0 {
                    let _: Snapshot = bitmap_clone.at(10).one::<Cooperative>();
                } else {
                    let _: Snapshot = bitmap_clone.at(10).zero::<Cooperative>();
                }
            }));
        }

        for handle in handles {
            handle.join().expect("worker thread panicked");
        }

        // Fence to ensure we see all writes before snapshot (for Miri)
        fence(Ordering::Acquire);

        // Final state is unpredictable, but all operations succeeded
        // The key is that no operation failed - they all completed
        bitmap.snapshot(); // Just verify we can read it
    }

    /// Test operations across the full bit range
    #[test]
    fn operations_across_full_range() {
        let bitmap = AtomicBitmap::zeroed();

        // Set all bits
        for i in 0..usize::BITS {
            let _: Snapshot = bitmap.at(i).one::<Cooperative>();
        }
        assert_eq!(bitmap.snapshot(), usize::MAX, "all bits should be set");

        // Clear all bits
        for i in 0..usize::BITS {
            let _: Snapshot = bitmap.at(i).zero::<Cooperative>();
        }
        assert_eq!(bitmap.snapshot(), 0, "all bits should be cleared");
    }

    /// Test that independent operations on multiple bits work correctly
    #[test]
    fn multiple_independent_bit_operations() {
        let bitmap = AtomicBitmap::zeroed();

        let _: Snapshot = bitmap.at(0).one::<Cooperative>();
        let _: Snapshot = bitmap.at(5).one::<Cooperative>();
        let _: Snapshot = bitmap.at(15).one::<Cooperative>();
        let _: Snapshot = bitmap.at(31).one::<Cooperative>();

        let expected = (1 << 0) | (1 << 5) | (1 << 15) | (1 << 31);
        assert_eq!(bitmap.snapshot(), expected, "selected bits should be set");

        let _: Snapshot = bitmap.at(5).zero::<Cooperative>();
        let expected = (1 << 0) | (1 << 15) | (1 << 31);
        assert_eq!(bitmap.snapshot(), expected, "bit 5 should be cleared");
    }
}

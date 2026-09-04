//! An atomic bitmap composed of a single [`AtomicUsize`].
//!
//! [`AtomicBitmap`] is an atomic type that provides bit-level atomic operations
//! on a single [`AtomicUsize`].

use core::{
    num::NonZero,
    sync::atomic::{
        AtomicUsize,
        Ordering::{Acquire, Relaxed},
    },
};

use nekor_aal_signal::prelude::{Monitor, Monitored};

use crate::atomic::bitmap::{
    at::At,
    conditional::{Condition, Status},
    typeutil::{BitIndex, InBound},
};

pub mod at;

pub mod mode;

pub mod typeutil;

pub mod conditional;

/// An atomic bitmap composed of a single [`AtomicUsize`].
///
/// This is an atomic type that provides bit-level atomic operations on a single
/// [`AtomicUsize`].
#[repr(transparent)]
#[derive(Debug)]
pub struct AtomicBitmap(Monitor<AtomicUsize>);

impl AtomicBitmap {
    /// Construct a new [`AtomicBitmap`] that is zeroed.
    #[inline]
    #[must_use]
    pub const fn zeroed() -> Self {
        Self(Monitor::new(AtomicUsize::new(usize::MIN)))
    }

    /// Construct a new [`AtomicBitmap`] with a raw [`AtomicUsize`].
    #[inline]
    pub const fn raw(target_state: AtomicUsize) -> Self {
        Self(Monitor::new(target_state))
    }
}

impl AtomicBitmap {
    // TODO: Migrate to Selected

    /// Attempt to engage with a specific bit in the [`AtomicBitmap`].
    ///
    /// # Failure
    ///
    /// This will be [`None`] if the bit is not within the bounds of an
    /// [`usize`].
    #[inline]
    pub fn try_at(&self, at_index: u32) -> Option<At<'_>> {
        let &Self(ref target_value) = self;

        // SAFETY: The snapshot is loaded with a memory ordering of `Acquire`.
        unsafe { self.try_at_with(at_index, target_value.load(Acquire)) }
    }

    /// Attempt to engage with a specific bit in the [`AtomicBitmap`].
    ///
    /// # Failure
    ///
    /// This will be [`None`] if the bit is not within the bounds of an
    /// [`usize`].
    ///
    /// # Safety
    ///
    /// The provided snapshot must have been obtained from this
    /// [`AtomicBitmap`], with a memory ordering [`Acquire`] or stronger.
    #[inline]
    pub const unsafe fn try_at_with(&self, at_index: u32, target_snapshot: usize) -> Option<At<'_>> {
        let &Self(ref target_value) = self;

        match at_index {
            0..usize::BITS => Some(At(target_value, target_snapshot, at_index)),
            usize::BITS.. => None,
        }
    }

    /// Engage with a specific bit in the [`AtomicBitmap`].
    ///
    /// # Failure
    ///
    /// This will *panic* if the bit is not within the bounds of an
    /// [`usize`].
    #[inline]
    pub fn at(&self, at_index: u32) -> At<'_> {
        let &Self(ref target_value) = self;

        // SAFETY: The snapshot is loaded with a memory ordering of `Acquire`.
        unsafe { Self::at_with(self, at_index, target_value.load(Acquire)) }
    }

    /// Engage with a specific bit in the [`AtomicBitmap`].
    ///
    /// # Panics
    ///
    /// This will *panic* if the bit is not within the bounds of an
    /// [`usize`].
    ///
    /// # Safety
    ///
    /// The provided snapshot must have been obtained from this
    /// [`AtomicBitmap`], with a memory ordering [`Acquire`] or stronger.
    #[inline]
    pub const unsafe fn at_with(&self, at_index: u32, target_snapshot: usize) -> At<'_> {
        // SAFETY: The snapshot is loaded with a memory ordering of `Acquire`.
        unsafe {
            Self::try_at_with(self, at_index, target_snapshot)
                .expect("invariant: bit not within usize bitwise boundary")
        }
    }

    /// Engage with the least significant bit that is set in the
    /// [`AtomicBitmap`].
    ///
    /// This engages with the rightmost bit that is set.
    ///
    /// # Failure
    ///
    /// This will be [`None`] if no bits are set in the bitmap.
    #[inline]
    pub fn at_rightmost(&self) -> Option<At<'_>> {
        let &Self(ref target_value) = self;

        // SAFETY: The snapshot is loaded with a memory ordering of `Acquire`.
        unsafe { self.at_rightmost_with(target_value.load(Acquire)) }
    }

    /// Engage with the least significant bit that is set in the
    /// [`AtomicBitmap`].
    ///
    /// This engages with the rightmost bit that is set.
    ///
    /// # Failure
    ///
    /// This will be [`None`] if no bits are set in the bitmap.
    ///
    /// # Safety
    ///
    /// The provided snapshot must have been obtained from this
    /// [`AtomicBitmap`], with a memory ordering [`Acquire`] or stronger.
    #[inline]
    pub unsafe fn at_rightmost_with(&self, target_snapshot: usize) -> Option<At<'_>> {
        let &Self(ref target_value) = self;

        NonZero::<usize>::new(target_snapshot)
            .map(NonZero::get)
            .map(|target_snapshot| At(target_value, target_snapshot, usize::trailing_zeros(target_snapshot)))
    }

    /// Engage with the most significant bit that is set in the
    /// [`AtomicBitmap`].
    ///
    /// This engages with the leftmost bit that is set.
    ///
    /// # Failure
    ///
    /// This will be [`None`] if no bits are set in the bitmap.
    #[inline]
    pub fn at_leftmost(&self) -> Option<At<'_>> {
        let &Self(ref target_value) = self;

        // SAFETY: The snapshot is loaded with a memory ordering of `Acquire`.
        unsafe { self.at_leftmost_with(target_value.load(Acquire)) }
    }

    /// Engage with the most significant bit that is set in the
    /// [`AtomicBitmap`].
    ///
    /// This engages with the leftmost bit that is set.
    ///
    /// # Failure
    ///
    /// This will be [`None`] if no bits are set in the bitmap.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the snapshot is loaded with a memory
    /// ordering of `Acquire` or stronger.
    #[inline]
    pub unsafe fn at_leftmost_with(&self, target_snapshot: usize) -> Option<At<'_>> {
        let &Self(ref target_value) = self;

        NonZero::<usize>::new(target_snapshot)
            .map(NonZero::get)
            .map(|target_snapshot| {
                At(
                    target_value,
                    target_snapshot,
                    // NOTE(underflow): Will never underflow as
                    // `usize::leading_zeros` is never >= `usize::BITS`.
                    usize::BITS - 1 - usize::leading_zeros(target_snapshot),
                )
            })
    }

    /// Engage with a bit found by a custom closure predicate.
    ///
    /// The closure is provided with the current value of the [`AtomicBitmap`]
    /// and should return the index of the bit to engage with.
    ///
    /// # Failure
    ///
    /// This will be [`None`] if the closure returns [`None`] or if the bit
    /// index is not within the bounds of an [`usize`].
    #[inline]
    pub fn at_predicate<F>(&self, target_closure: F) -> Option<At<'_>>
    where
        F: FnOnce(usize) -> Option<u32>,
    {
        let &Self(ref target_value) = self;

        let target_snapshot = target_value.load(Acquire);

        target_closure(target_snapshot).and_then(|target_index| Self::try_at(self, target_index))
    }

    /// Engage with a compile-time constant bit index.
    ///
    /// This is useful for engaging with a bit that is known at compile-time.
    #[inline]
    pub fn static_at<const N: u32>(&self) -> At<'_>
    where
        BitIndex<N>: InBound,
    {
        let &Self(ref target_value) = self;

        let snapshot_value = target_value.load(Acquire);

        // NOTE(invariant): The valid bit indice range is maintained by the
        // `BitIndex` generic bound.
        At(target_value, snapshot_value, N)
    }

    /// Engage with a compile-time constant bit index, without acquiring a new
    /// snapshot.
    ///
    /// This is useful for engaging with a bit that is known at compile-time.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the snapshot is loaded with a memory
    /// ordering of `Acquire` or stronger.
    #[inline]
    pub const unsafe fn static_at_with<const N: u32>(&self, snapshot_value: usize) -> At<'_>
    where
        BitIndex<N>: InBound,
    {
        let &Self(ref target_value) = self;

        // NOTE(invariant): The valid bit indice range is maintained by the
        // `BitIndex` generic bound.
        At(target_value, snapshot_value, N)
    }
}

impl AtomicBitmap {
    /// Engage the monitor on the [`AtomicBitmap`].
    ///
    /// This provides a way to make use of the associated [`Monitored`]
    /// utilities for busy-waiting.
    #[inline]
    pub fn monitor(&self) -> Monitored<'_, AtomicUsize> {
        let &Self(ref target_value) = self;

        Monitor::engage(target_value)
    }

    /// Busy-wait until one of the following conditions occur:
    ///
    /// - The [`AtomicBitmap`] is mutated.
    /// - The thread of execution is interrupted.
    /// - The thread of execution is spuriously woken.
    ///
    /// This implies that, if a wake is issued, the underlying atomic bitmap may
    /// have not been mutated at all.
    ///
    /// # Remarks
    ///
    /// This makes use of the existing [`Monitor`] utilities.
    #[inline]
    pub fn wait(&self) {
        let &Self(ref target_value) = self;

        Monitored::wait(Monitor::engage(target_value));
    }

    /// Determine whether the target [`Condition`] is satisfied by the
    /// [`AtomicBitmap`].
    #[inline]
    pub fn condition<C>(&self, target_condition: C) -> Status
    where
        C: Condition,
    {
        let &Self(ref target_value) = self;

        C::determine(target_condition, target_value.load(Acquire))
    }

    /// Snapshot the current value of the [`AtomicBitmap`].
    ///
    /// # Remarks
    ///
    /// This has no validity in terms of the immediately-after atomic bitmap,
    /// and should therefore not be relied upon for regular synchronization.
    ///
    /// This does not create any happens-before relationship with posterior
    /// atomic operations.
    #[inline]
    pub fn snapshot(&self) -> usize {
        let &Self(ref target_value) = self;

        target_value.load(Relaxed)
    }
}

#[doc(hidden)]
const fn _assert_atomic_bitmap_is_send_sync()
where
    AtomicBitmap: Send + Sync,
{
}

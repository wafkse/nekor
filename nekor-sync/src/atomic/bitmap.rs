//! Atomic operations on a single [`AtomicUsize`] bitmap.

use core::{
    num::NonZero,
    sync::atomic::{
        AtomicUsize,
        Ordering::{Acquire, Relaxed},
    },
};

use nekor_aal_signal::prelude::{Monitor, Monitored};
use nekor_backoff::backoff::BackoffState;

use crate::atomic::bitmap::{
    at::At,
    conditional::{Condition, Status},
    typeutil::{BitIndex, InBound},
};

pub mod at;

pub mod mode;

pub mod typeutil;

pub mod conditional;

/// An atomic bitmap backed by a single [`AtomicUsize`].
#[repr(transparent)]
#[derive(Debug)]
// NOTE(invariant): None.
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
    // TODO: Migrate to `Selected`.

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
                    // NOTE: A nonzero value has fewer than `usize::BITS` leading zeros.
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

        target_closure(target_snapshot).and_then(|target_index| {
            // SAFETY: Selection and CAS use the same `Acquire` snapshot.
            unsafe { Self::try_at_with(self, target_index, target_snapshot) }
        })
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

        // NOTE: The valid bit index range is maintained by the
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

        // NOTE: The valid bit index range is maintained by the
        // `BitIndex` generic bound.
        At(target_value, snapshot_value, N)
    }
}

impl AtomicBitmap {
    /// Borrow the bitmap's [`Monitor`] without engaging it.
    ///
    /// Use [`Polling::of`] or [`Polling::with_backoff`] to construct polling state.
    ///
    /// [`Polling::of`]: nekor_aal_signal::monitor::Polling::of
    /// [`Polling::with_backoff`]: nekor_aal_signal::monitor::Polling::with_backoff
    #[inline]
    #[must_use]
    pub const fn value(&self) -> &Monitor<AtomicUsize> {
        let &Self(ref target_value) = self;

        target_value
    }

    /// Engage the bitmap's monitor and return a [`Monitored`] token.
    #[inline]
    pub fn monitor(&self) -> Monitored<'_, AtomicUsize> {
        let &Self(ref target_value) = self;

        Monitor::engage(target_value)
    }

    /// Engage the monitor and wait for a possible write.
    ///
    /// # Remarks
    ///
    /// Interrupts and spurious wakeups may return without a write. Recheck the
    /// condition after waiting. Retain the [`BackoffState`] across calls.
    #[inline]
    pub fn wait(&self, backoff_state: &mut BackoffState) {
        let &Self(ref target_value) = self;

        Monitored::wait(Monitor::engage(target_value), backoff_state);
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

    /// Snapshot the bitmap using `Relaxed` ordering.
    ///
    /// # Remarks
    ///
    /// The snapshot may become stale and does not establish synchronization.
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

#[cfg(test)]
mod tests {
    use nekor_backoff::prelude::{Backoff, Retry};

    use super::{
        AtomicBitmap,
        mode::{
            cooperative::{Cooperative, Snapshot},
            exclusive::{Exclusive, Outcome, Reason},
        },
    };

    /// [`Polling`] and [`AtomicBitmap`] observe the same atomic word.
    ///
    /// [`Polling`]: nekor_aal_signal::monitor::Polling
    #[test]
    fn polling_borrows_bitmap_monitor() {
        use core::sync::atomic::Ordering::{AcqRel, Acquire};

        use nekor_aal_signal::monitor::{Attempt, Immediate, Polling};

        let bitmap = AtomicBitmap::zeroed();
        let mut polling = Polling::<_, Immediate>::with_backoff(bitmap.value(), Backoff::minimal());

        let previous = polling.until(|value| match value.compare_exchange(0, 1, AcqRel, Acquire) {
            Ok(previous) => Attempt::Ready(previous),
            Err(..) => Attempt::Pending,
        });

        assert_eq!(previous, 0);
        assert_eq!(bitmap.snapshot(), 1);
    }

    #[test]
    fn predicate_candidate_keeps_its_selection_snapshot() {
        let bitmap = AtomicBitmap::zeroed();
        let candidate = bitmap
            .at_predicate(|snapshot| {
                assert_eq!(snapshot, 0);

                let _: Snapshot = bitmap.at(0).one::<Cooperative>();
                let _: Snapshot = bitmap.at(1).one::<Cooperative>();
                let _: Snapshot = bitmap.at(0).zero::<Cooperative>();

                Some(0)
            })
            .expect("empty snapshot selected bit zero");

        let mut retry = Retry::oneshot(Backoff::minimal());

        assert!(matches!(
            candidate.one_with::<Exclusive>(&mut retry),
            Outcome::Failure(Reason::Contended, ..)
        ));
        assert_eq!(bitmap.snapshot(), 2);
    }
}

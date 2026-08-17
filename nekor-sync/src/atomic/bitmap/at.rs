use core::sync::atomic::AtomicUsize;

use nekor_aal_signal::{monitor::MonitorGuard, prelude::Monitor};

use crate::atomic::bitmap::mode::{InMode, Mode};

/// An engaged [`AtomicBitmap`] bit.
///
/// This serves as the base for all bit-level atomic operations.
///
/// # Engagement Finalization
///
/// An [`Engaged`] bit in an [`AtomicBitmap`] is finalized through a Finalizer
/// Operation.
///
/// All associated functions contained within the [`Engaged`] type are Finalizer
/// Operations.
///
/// A Finalizer Operation always has an *engagement* [`Mode`] associated to it.
/// The engagement mode determines whether the Finalizer Operation is:
///
/// - [`Exclusive`]: The engaged bit is guaranteed to have been uniquely
///   interacted by us.
/// - [`Cooperative`]: The engaged bit is guaranteed to have been interacted by
///   us, but there is no guarantee that it was uniquely interacted by us.
///
/// This kind of distinction is required for subtle concurrency and
/// synchronization needs. For instance, an [`Exclusive`] engagement mode is
/// required for operations that require exclusive access to the bit, such as
/// setting or clearing it. In contrast, a [`Cooperative`] engagement mode is
/// suitable for operations that only require read access to the bit, such as
/// checking its value.
#[derive(Debug, Copy, Clone)]
pub struct At<'a>(
    pub(in crate::atomic::bitmap) &'a Monitor<AtomicUsize>,
    pub(in crate::atomic::bitmap) usize,
    // NOTE(invariant): The bit index is always within bounds for an `usize`.
    pub(in crate::atomic::bitmap) u32,
);

impl<'a> At<'a> {
    /// Disengage the provided engagement vector.
    ///
    /// This requires proof of being in a [`Mode`] finalizer.
    ///
    /// [`Mode`]: super::mode::Mode
    #[inline]
    #[must_use]
    pub const fn disengage(self, _: &InMode) -> Disengage<'a> {
        let Self(target_value, state_snapshot, bit_index) = self;

        Disengage(Monitor::access(target_value), state_snapshot, bit_index)
    }

    /// Determine the index that has been engaged.
    #[inline]
    #[must_use]
    pub const fn index(&self) -> u32 {
        let &Self(.., target_index) = self;

        target_index
    }

    /// Access the immediately-leftwards (towards MSB) bit engaged by this
    /// [`At`].
    ///
    /// This will be `None` if the current bit is already the leftmost bit.
    #[inline]
    #[must_use]
    pub const fn left(self) -> Option<Self> {
        const USIZE_BITS: u32 = usize::BITS;

        let Self(target_value, state_snapshot, bit_index) = self;

        match bit_index + 1 {
            USIZE_BITS.. => None,
            leftwards_bit @ 0..USIZE_BITS => {
                Some(Self(target_value, state_snapshot, leftwards_bit))
            }
        }
    }

    /// Access the immediately-rightwards (towards LSB) bit engaged by this
    /// [`At`].
    ///
    /// This will be `None` if the current bit is already the rightmost bit.
    #[inline]
    #[must_use]
    pub const fn right(self) -> Option<Self> {
        let Self(target_value, state_snapshot, bit_index) = self;

        match bit_index.checked_sub(1) {
            Some(rightwards_bit) => Some(Self(target_value, state_snapshot, rightwards_bit)),
            None => None,
        }
    }
}

impl At<'_> {
    /// Clear the engaged bit.
    ///
    /// # [`Mode`]-specific behavior
    ///
    /// On the [`Exclusive`] mode, this Finalizer Operation can:
    ///
    /// - Block until the engaged bit is zeroed.
    /// - Bail out if the engaged bit was zeroed already, but not by us.
    ///
    /// For cases where a specified [`Backoff`] is required, the
    /// [`Engaged::zero_with`] associated function can be used.
    ///
    /// This is equivalent to calling [`Engaged::zero_with`] with a
    /// [`Backoff::minimal()`].
    #[inline]
    #[must_use = "the outcome signal may be particularly relevant"]
    pub fn zero<M: Mode>(self) -> M::Signal
    where
        M::State: Default,
    {
        Self::zero_with::<M>(self, &mut Default::default())
    }

    /// Clear the engaged bit.
    ///
    /// # [`Mode`]-specific behavior
    ///
    /// On the [`Exclusive`] mode, this Finalizer Operation can:
    ///
    /// - Block until the engaged bit is zeroed.
    /// - Bail out if the engaged bit was zeroed already, but not by us.
    ///
    /// This will engage with the specified [`Backoff`] strategy when the
    /// [`AtomicBitmap`] is under contention and the operation warrants a retry.
    #[inline]
    #[must_use = "the outcome signal may be particularly relevant"]
    pub fn zero_with<M: Mode>(self, target_state: &mut M::State) -> M::Signal {
        // SAFETY: We are in an engagement finalizer.
        let in_mode = &unsafe { InMode::affirmative() };

        M::zero_with(self, target_state, in_mode)
    }

    /// Set the engaged bit.
    ///
    /// # [`Mode`]-specific behavior
    ///
    /// On the [`Exclusive`] mode, this Finalizer Operation can:
    ///
    /// - Block until the engaged bit is zeroed.
    /// - Bail out if the engaged bit was zeroed already, but not by us.
    ///
    /// For cases where a specified [`Backoff`] is required, the
    /// [`Engaged::one_with`] associated function can be used.
    ///
    /// This is equivalent to calling [`Engaged::one_with`] with a
    /// [`Backoff::minimal()`].
    #[inline]
    #[must_use = "the outcome signal may be particularly relevant"]
    pub fn one<M: Mode>(self) -> M::Signal
    where
        M::State: Default,
    {
        Self::one_with::<M>(self, &mut Default::default())
    }

    /// Set the engaged bit.
    ///
    /// # [`Mode`]-specific behavior
    ///
    /// On the [`Exclusive`] mode, this Finalizer Operation can:
    ///
    /// - Block until the engaged bit is set.
    /// - Bail out if the engaged bit was set already, but not by us.
    ///
    /// This will engage with the specified [`Backoff`] strategy when the
    /// [`AtomicBitmap`] is under contention and the operation warrants a retry.
    #[inline]
    #[must_use = "the outcome signal may be particularly relevant"]
    pub fn one_with<M: Mode>(self, target_state: &mut M::State) -> M::Signal {
        // SAFETY: We are in an engagement finalizer.
        let in_mode = &unsafe { InMode::affirmative() };

        M::one_with(self, target_state, in_mode)
    }
}

/// A structure that encompasses all required state for an [`At`] disengagement.
pub struct Disengage<'a>(pub MonitorGuard<'a, AtomicUsize>, pub usize, pub u32);

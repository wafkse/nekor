use core::{
    num::NonZero,
    ops::Deref,
    sync::atomic::{AtomicUsize, Ordering},
};

use nekor_aal::local::prelude::Area;

use nekor_domain::zeroed::{Zeroable, Zeroed};

use crate::{domain::Cpu, limit::Cores};

/// A reutilization response from a [`CoreId`] that has been revoked.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub enum Reutilization {
    /// Reutilization has been deemed impossible due to a counter overrun.
    Impossible,

    /// Reutilization has been deemed possible, and will be performed.
    Yes,
}

/// A wrapper over an [`AtomicUsize`] that coordinates system-wide [`CoreId`]
/// identification.
///
/// This also serves as a central authority over [`CoreId`] initialization.
#[derive(Debug)]
#[repr(transparent)]
pub struct Handout(AtomicUsize);

// SAFETY: `HandoutId` is `repr(transparent)` over `AtomicUsize`, which is
// intrinsically `Zeroable`.
unsafe impl Zeroable for Handout {}

impl Handout {
    /// Determine the global instance of this [`HandoutId`].
    #[inline]
    #[must_use]
    pub fn global() -> &'static Self {
        Zeroed::explicit_in::<Self, Cpu>()
    }

    /// Acquire a new [`CoreId`] for exclusive use.
    ///
    /// # Safety
    ///
    /// Although this associated function does allow a core to acquire a
    /// [`CoreId`] on behalf of other, however, it must guarantee that:
    ///
    /// - It does not access per-CPU data assigned to the same [`CoreId`] while
    ///   the other is online.
    /// - The [`CoreId`] is written to the per-CPU [`Area`] structure.
    #[inline]
    pub unsafe fn acquire(&self) -> Option<CoreId> {
        let Self(target_counter) = self;

        let mut target_value = target_counter.load(Ordering::Acquire);

        loop {
            if target_value >= NonZero::get(Cores::maximum()) - 1 {
                break None;
            }

            match target_counter.compare_exchange(
                target_value,
                target_value.wrapping_add(1),
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(..) => break Some(CoreId(target_value)),
                // NOTE: No backoff required here, as contention is not a
                // concern.
                Err(updated_value) => target_value = updated_value,
            }
        }
    }

    /// Revoke the target [`CoreId`] from the system.
    ///
    /// This will attempt to reutilize the revoked identifier as possible,
    /// signaling it in its return value.
    ///
    /// # Safety
    ///
    /// The caller processor core that held the provided [`CoreId`] *must be
    /// offline*, and must remain like so for as long as no further [`CoreId`]
    /// is assigned to it.
    #[inline]
    pub unsafe fn revoke(&self, target_id: CoreId) -> Reutilization {
        let Self(target_counter) = self;

        let CoreId(revoked_id) = target_id;

        if revoked_id == 0 {
            Reutilization::Impossible
        } else {
            let snapshot_value = target_counter.load(Ordering::Acquire);

            // NOTE(underflow): Will never underflow due to previous
            // preliminary check.
            if (snapshot_value - 1) == revoked_id {
                let _ = target_counter.fetch_sub(1, Ordering::AcqRel);

                Reutilization::Yes
            } else {
                // NOTE: Missed the slim chance to decrement the counter
                // back.
                Reutilization::Impossible
            }
        }
    }
}

/// The unique numeric identifier of a single processor core.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
#[repr(transparent)]
pub struct CoreId(usize);

impl CoreId {
    /// Determine the [`CoreId`] of this processor core.
    #[inline]
    #[must_use]
    pub fn mine() -> Self {
        Self(Area::value(Area::read()))
    }
}

impl Deref for CoreId {
    type Target = usize;

    #[inline]
    fn deref(&self) -> &Self::Target {
        let Self(target_id) = self;

        target_id
    }
}

// SAFETY: `CoreId` is transparent over `usize`, which is `Zeroable`.
unsafe impl Zeroable for CoreId {}

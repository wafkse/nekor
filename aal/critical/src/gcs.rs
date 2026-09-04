//! Global critical sections coordinate access across processor cores.
//!
//! - *Global Critical Sections* (*GCS*) are those who:
//!     - *Disable interrupts* for the currently-active core.
//!     - *Lock* a corresponding [`Mutex`] for the desired global lock category. This will inhibit
//!       any other global contenders for the same *GCS* kind.
//!
//! Used for access control to a non-concurrent subsystem.
//!
//! Many *GCS* can coexist. Delay depends on per-*GCS* contention.

use core::marker;

/// A marker trait to become a *Global Critical Section* (*GCS*) contender for a
/// [`Gcs`].
pub trait Contender {}

/// A Global Critical Section for a particular global [`Contender`] `C`.
#[repr(transparent)]
pub struct Gcs<C>(marker::PhantomData<fn() -> C>)
where
    C: Contender;

impl<C> Gcs<C>
where
    C: Contender,
{
    /// Acquire a *Global Critical Section* for a [`Contender`] `C`.
    ///
    /// # Safety
    ///
    /// - A call to this function must be matched with a posterior [`Gcs::release`] call for the
    ///   same `C`.
    /// - The output [`GcsToken`] must not outlive the actual *GCS*.
    /// - The calling core must not be in the following Critical Section types:
    ///     - [`Gcs<C>`]: *Global Critical Section* for the same [`Contender`] `C`.
    ///     - [`MsCs`]: *Machine-Stop Critical Section*
    ///
    /// [`MsCs`]: crate::mscs::MsCs
    #[must_use]
    pub const unsafe fn acquire() -> GcsToken<C> {
        // SAFETY: The core has just acquired a critical section, given the
        // contractual safety requirements.
        unsafe { GcsToken::<C>::raw() }
    }
}

/// A private-constructible token that can *prove* whether a caller is currently
/// inside a [`Gcs<C>`] for a particular global [`Contender`].
#[derive(Debug)]
pub struct GcsToken<C>(marker::PhantomData<fn() -> C>)
where
    C: Contender;

impl<C> GcsToken<C>
where
    C: Contender,
{
    /// Construct a [`GcsToken`] out of thin air for a particular [`Contender`].
    ///
    /// # Safety
    ///
    /// The caller must be held in a [`Gcs`], and the lifetime of the
    /// [`GcsToken`] must not outlive such critical section.
    #[inline]
    #[must_use]
    pub const unsafe fn raw() -> Self {
        Self(marker::PhantomData)
    }
}

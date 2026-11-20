//! Authority tokens for a caller-established global critical section.
//!
//! The caller holds the lock associated with a [`Contender`] and establishes any required
//! local interrupt masking before creating a token. This module does not acquire that lock,
//! mask interrupts or release the surrounding critical section.

use core::marker;

/// A marker trait to become a *Global Critical Section* (*GCS*) contender for a
/// [`Gcs`].
pub trait Contender {}

/// A Global Critical Section for a particular global [`Contender`] `C`.
#[repr(transparent)]
// NOTE(invariant): The marker binds this critical section authority to contender `C`.
pub struct Gcs<C>(marker::PhantomData<fn() -> C>)
where
    C: Contender;

impl<C> Gcs<C>
where
    C: Contender,
{
    /// Represent an already-held global critical section for contender `C`.
    ///
    /// # Safety
    ///
    /// - The caller must hold the global lock for `C` while the returned token remains valid.
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
// NOTE(invariant): Only a caller inside the matching global critical section may create this token.
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

//! Opaque type.

use core::fmt;

use nekor_primitive::scalar::Scalar;

/// An opaque wrapper over some [`Scalar`] type.
///
/// This is used to prevent access altogheter, and can be
/// used as a safety barrier, as all accesses are marked unsafe.
#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Hash)]
#[repr(transparent)]
pub struct Opaque<S>(S)
where
    S: Scalar;

impl<S> Opaque<S>
where
    S: Scalar,
{
    /// Construct an opaque view of `S`.
    ///
    /// # Safety
    ///
    /// This is not inherently unsafe, but access to the underlying value is an
    /// intended safety barrier.
    ///
    /// Ensure that any ulterior invariants are guaranteed when accessing an
    /// [`Opaque`] value.
    #[inline]
    pub const unsafe fn value(target_value: S) -> Self {
        Self(target_value)
    }

    /// Unwrap an [`Opaque`] value of type `S`.
    ///
    /// # Safety
    ///
    /// As it is with [`Opaque`] types, this is unsafe to guarantee underlying
    /// invariants.
    ///
    /// See the [`Self::value`] **Safety** section for rationale and any
    /// additional information.
    #[inline]
    pub const unsafe fn unwrap(self) -> S {
        let Self(target_value) = self;

        target_value
    }
}

impl<S> fmt::Debug for Opaque<S>
where
    S: Scalar + fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Opaque").finish_non_exhaustive()
    }
}

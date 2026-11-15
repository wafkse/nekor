//! Opaque type.

use core::fmt;

use nekor_primitive::scalar::Scalar;

/// A scalar whose direct construction and extraction require unsafe code.
///
/// Higher-level APIs can use this boundary to require a local justification before exposing
/// the value. Comparisons and copying are still available through the derived traits, while
/// the [`Debug`](fmt::Debug) implementation hides the value.
#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Hash)]
#[repr(transparent)]
// NOTE(invariant): The scalar remains inaccessible through safe APIs because its only field is
// private.
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
    /// The scalar must satisfy any restrictions imposed by the enclosing abstraction.
    /// `Opaque` does not check those restrictions or impose additional ones of its own.
    #[inline]
    pub const unsafe fn value(target_value: S) -> Self {
        Self(target_value)
    }

    /// Unwrap an [`Opaque`] value of type `S`.
    ///
    /// # Safety
    ///
    /// Extracting the scalar must comply with the enclosing abstraction's access rules.
    /// The unsafe call does not check those rules.
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

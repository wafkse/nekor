//! Type-level field reservation.

use core::mem::MaybeUninit;

use nekor_primitive::scalar::Scalar;

/// A dummy type that is used to fill hardware-dependant structures that have
/// reserved fields.
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Reserved<S>(MaybeUninit<S>)
where
    S: Scalar;

impl<S> Reserved<S>
where
    S: Scalar,
{
    /// Construct a dummy reserved field.
    ///
    /// This will act as zeroed memory.
    #[inline]
    pub const fn field() -> Self {
        Self(MaybeUninit::zeroed())
    }
}

impl<S> Default for Reserved<S>
where
    S: Scalar,
{
    #[inline]
    fn default() -> Self {
        Self::field()
    }
}

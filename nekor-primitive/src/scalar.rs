//! Primitive types that are known to be scalars.

/// A trait pertaining to a scalar value.
///
/// When we say "scalar", we mean the following:
///
/// - A scalar is a single value that is uniquely representable.
/// - A scalar must be trivially copyable. i.e., it must be [`Copy`].
/// - A scalar must posess at least partial ordering. i.e., it must be
///   [`PartialOrd`].
/// - A scalar must be of a fixed size. i.e., it must be [`Sized`].
/// - A scalar must outlive the `'static` lifetime. i.e., it must be `'static`.
/// - A scalar must be trivially shareable between threads through a stable
///   reference.
///
/// This trait makes no further assumptions about the algebraic properties of
/// the scalar other than those listed above.
///
/// # Remarks
///
/// This trait will be automatically implemented for all types that satisfy the
/// above conditions.
pub trait Scalar: 'static + Copy + PartialOrd + Sync + Sized {}

/// Blanket implementation of [`Scalar`] for all suitable types.
impl<T> Scalar for T where T: 'static + Copy + PartialOrd + Sync + Sized {}

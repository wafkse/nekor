//! Traits for basic value identity.
//!
//! This module provides the [`Zero`] and [`One`] traits, which are used to
//! represent the identity elements of a scalar type.
//!
//! These traits are extraneous to the [`Scalar`] trait, but are provided for
//! convenience when needed.

use crate::scalar::Scalar;

/// A macro to implement both the [`Zero`] and [`One`] traits for a given type.
macro_rules! zero {
    () => {};
    (
        for [
            $(
                $scalar_ty:ty
            ),*
        ] use $scalar_zero_expr:expr
    ) => {
        $(
            impl Zero for $scalar_ty {
                #[inline]
                fn zero() -> Self {
                    $scalar_zero_expr
                }
            }
        )*
    };
}

/// Implement [`One`] for a list of scalar types.
macro_rules! one {
    () => {};
    (
        for [
            $(
                $scalar_ty:ty
            ),*
        ] use $scalar_one_expr:expr
    ) => {
        $(
            impl One for $scalar_ty {
                #[inline]
                fn one() -> Self {
                    $scalar_one_expr
                }
            }
        )*
    };
}

/// A trait for types that have a value that is known to exactly behave like
/// zero.
pub trait Zero: Scalar {
    /// Determine the additive identity of the scalar type.
    fn zero() -> Self;
}

/// A trait for types that have a value that is known to exactly behave like
/// one.
pub trait One: Scalar {
    /// Determine the multiplicative identity of the scalar type.
    fn one() -> Self;
}

zero!(for [f32, f64] use 0.0);
zero!(for [i8, i16, i32, i64, isize] use 0);
zero!(for [u8, u16, u32, u64, usize] use 0);
zero!(for [u128, i128] use 0);

one!(for [f32, f64] use 1.0);
one!(for [i8, i16, i32, i64, isize] use 1);
one!(for [u8, u16, u32, u64, usize] use 1);
one!(for [u128, i128] use 1);

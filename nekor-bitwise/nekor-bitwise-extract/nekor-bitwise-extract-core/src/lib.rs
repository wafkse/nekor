#![no_std]
#![forbid(
    clippy::all,
    clippy::perf,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::panic,
    unsafe_code,
    rustdoc::all
)]
// Primitive narrowing is an explicit part of the bit extraction contract.
#![deny(clippy::pedantic)]
#![doc = include_str!("../../README.md")]

#[doc(inline)]
pub use nekor_bitwise_extract_macro::{delegate, extract, implement, metadata};

use nekor_primitive::scalar::Scalar;

mod detail {
    //! Implementation detail module.

    /// A trait to act a seal supertrait to the [`As`] trait.
    ///
    /// [`As`]: super::As
    pub trait Sealed {}
}

/// A helper trait to cast between [`Scalar`] types.
pub trait As<O>: Scalar + detail::Sealed
where
    O: Scalar,
{
    /// Widen the output scalar into the input scalar.
    fn input(target_output: O) -> Self;

    /// Narrow the input scalar to the output width.
    ///
    /// High bits outside the output width are discarded.
    fn output(self) -> O;
}

macro_rules! cast {
    ($target_in:path as [$($target_out:path),+ $(,)?]) => {
        impl detail::Sealed for $target_in {}

        $(
            impl As<$target_out> for $target_in {
                #[inline]
                fn input(target_output: $target_out) -> Self {
                    Self::from(target_output)
                }

                #[inline]
                #[allow(clippy::cast_possible_truncation)]
                fn output(self) -> $target_out {
                    self as $target_out
                }
            }
        )+
    };
}

cast!(u8 as [u8]);

cast!(u16 as [u8, u16]);

cast!(u32 as [u8, u16, u32]);

cast!(u64 as [u8, u16, u32, u64]);

cast!(u128 as [u8, u16, u32, u64, u128]);

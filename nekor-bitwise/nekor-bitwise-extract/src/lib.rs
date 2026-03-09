#![no_std]
#![forbid(
    clippy::all,
    clippy::perf,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::panic,
    clippy::pedantic,
    unsafe_code,
    rustdoc::all
)]
#![doc = include_str!("../README.md")]

use core::marker;

use nekor_primitive::scalar::Scalar;

use nekor_bitwise_size::{For2, Size};

use nekor_bitwise_extract_core::extract;

use nekor_bitwise_extract_8::MetadataU8;
use nekor_bitwise_extract_16::MetadataU16;
use nekor_bitwise_extract_32::MetadataU32;
use nekor_bitwise_extract_64::MetadataU64;
use nekor_bitwise_extract_128::MetadataU128;

mod private {
    /// A trait to act as a supertrait seal for the [`Extract`] trait.
    ///
    /// [`Extract`]: super::Extract
    pub trait Sealed {}
}

/// A macro to implement the [`Sealed`] trait for select types.
///
/// [`Sealed`]: private::Sealed
macro_rules! sealed {
    () => {};
    (
        $(
            $target_type:ident
        ),+ $(,)?
    ) => {
        $(
            impl private::Sealed for $target_type {}
        )+
    };
}

sealed!(u8, u16, u32, u64, u128, usize);

/// A trait for types that can have a sequence of bits `N..=M` (where `N < M`)
/// extracted from them.
pub trait Extract<const N: usize, const M: usize>: Scalar + private::Sealed {
    /// The output scalar type.
    type Output: Scalar;

    /// The number of bits to be extracted.
    const BITSET_WIDTH: usize;

    /// The bitwise mask composed of all to-be-extracted bits.
    const BITSET_MASK: Self;

    /// The fuse mask for the [`Extract::extract`] operation.
    const EXTRACT_MASK: Self;

    /// The fuse mask for the [`Extract::merge`] operation.
    const FUSE_MASK: Self;

    /// Extract the target bits from the target value, and return them as a new
    /// type: [`Extract::Output`].
    ///
    /// This is identical to a biased extract with a bitwise bias of zero.
    fn extract(&self) -> Self::Output;

    /// Fuse the target [`Extract::Output`] value back into the source type.
    fn merge(&mut self, target_value: Self::Output) -> Self::Output;
}

/// A helper type to make use of the [`Extract`] trait const-fn compatible.
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Extractor<const N: usize, const M: usize, E, O = <Size as For2<N, M>>::Target>(
    marker::PhantomData<fn() -> E>,
)
where
    E: Extract<N, M, Output = O>;

/// A helper macro to implement the distinct combinations of the [`Extract`]
/// implementors.
macro_rules! extractor {
    () => {};
    (
        $(
            #[$target_meta:meta]
        )*

        $target_type:ident for [$($target_output:ident),+ $(,)?]
    ) => {
       $(
            impl<const N: usize, const M: usize> Extractor<N, M, $target_type, $target_output>
            where
                $target_type: Extract<N, M, Output = $target_output>,
            {
                /// A const-fn version of the [`Extract::extract`] associated function.
                #[inline]
                pub const fn extract(target_value: &$target_type) -> $target_output {
                    let target_value = *target_value & <$target_type as Extract<N, M>>::EXTRACT_MASK;

                    let target_value = target_value >> N;

                    target_value as $target_output
                }

                /// A const-fn version of the [`Extract::merge`] associated function.
                #[inline]
                pub const fn merge(target_existing: &mut $target_type, target_output: $target_output) -> $target_output {
                    let existing_value = Self::extract(&target_existing);

                    let target_value = target_output as $target_type;

                    let target_value = target_value << N;

                    let target_value = target_value & <$target_type as Extract<N, M>>::EXTRACT_MASK;

                    let target_value = target_value | (*target_existing & <$target_type as Extract<N, M>>::FUSE_MASK);

                    *target_existing = target_value;

                    existing_value
                }
            }
       )+
    };
}

extractor!(u8 for [u8]);

extractor!(u16 for [u8, u16]);

extractor!(u32 for [u8, u16, u32]);

extractor!(u64 for [u8, u16, u32, u64]);

extractor!(u128 for [u8, u16, u32, u64, u128]);

extract!(Extract for [
    u8 use MetadataU8 become [u8],
    u16 use MetadataU16 become [u8, u16],
    u32 use MetadataU32 become [u8, u16, u32],
    u64 use MetadataU64 become [u8, u16, u32, u64],
    u128 use MetadataU128 become [u8, u16, u32, u64, u128],
]);

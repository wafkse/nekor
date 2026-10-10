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
// Const extraction requires explicit primitive narrowing at the output boundary.
#![doc = include_str!("../README.md")]

use core::marker;

use nekor_bitwise_extract_8::MetadataU8;
use nekor_bitwise_extract_16::MetadataU16;
use nekor_bitwise_extract_32::MetadataU32;
use nekor_bitwise_extract_64::MetadataU64;
use nekor_bitwise_extract_128::MetadataU128;
use nekor_bitwise_extract_core::extract;
use nekor_bitwise_size::{For2, Size};
use nekor_primitive::scalar::Scalar;

mod private {
    //! Implementation detail for integer primitive extraction.

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
///
/// On 16-, 32-, and 64-bit targets, `usize` uses the matching fixed-width
/// primitive's field ranges, output types, and metadata.
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

    /// Replace the selected bits and return their previous extracted value.
    ///
    /// Bits of `target_value` exceeding the field width are discarded. Every
    /// source bit outside `N..=M` is preserved. [`Extract::Output`] supplies
    /// the field's integer storage capacity.
    fn merge(&mut self, target_value: Self::Output) -> Self::Output;
}

/// A helper type to make use of the [`Extract`] trait const-fn compatible.
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, PartialOrd, Ord)]
#[repr(transparent)]
// NOTE(invariant): The marker carries the extractor input type while `N` and `M` select the
// compile-time extraction range without runtime state.
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

                /// A const-fn version of [`Extract::merge()`].
                ///
                /// This returns the previous field value, discards excess input bits,
                /// and preserves every source bit outside the selected field.
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

#[cfg(target_pointer_width = "16")]
extractor!(usize for [u8, u16]);

#[cfg(target_pointer_width = "32")]
extractor!(usize for [u8, u16, u32]);

#[cfg(target_pointer_width = "64")]
extractor!(usize for [u8, u16, u32, u64]);

extract!(Extract for [
    u8 use MetadataU8 become [u8],
    u16 use MetadataU16 become [u8, u16],
    u32 use MetadataU32 become [u8, u16, u32],
    u64 use MetadataU64 become [u8, u16, u32, u64],
    u128 use MetadataU128 become [u8, u16, u32, u64, u128],
]);

/// Delegate pointer-sized extraction to the target's equal-width primitive.
macro_rules! pointer {
    () => {};
    (
        $(
            #[$target_meta:meta]
        )*

        usize as $target_type:ident
    ) => {
        $(
            #[$target_meta]
        )*
        impl<const N: usize, const M: usize> Extract<N, M> for usize
        where
            $target_type: Extract<N, M>,
        {
            type Output = <$target_type as Extract<N, M>>::Output;

            const BITSET_WIDTH: usize = <$target_type as Extract<N, M>>::BITSET_WIDTH;

            const BITSET_MASK: Self = <$target_type as Extract<N, M>>::BITSET_MASK as Self;

            const EXTRACT_MASK: Self = <$target_type as Extract<N, M>>::EXTRACT_MASK as Self;

            const FUSE_MASK: Self = <$target_type as Extract<N, M>>::FUSE_MASK as Self;

            #[inline]
            fn extract(&self) -> Self::Output {
                let target_value = *self as $target_type;

                <$target_type as Extract<N, M>>::extract(&target_value)
            }

            #[inline]
            fn merge(&mut self, target_output: Self::Output) -> Self::Output {
                let mut target_value = *self as $target_type;

                let existing_value = <$target_type as Extract<N, M>>::merge(&mut target_value, target_output);

                *self = target_value as Self;

                existing_value
            }
        }
    };
}

#[cfg(target_pointer_width = "16")]
pointer!(usize as u16);

#[cfg(target_pointer_width = "32")]
pointer!(usize as u32);

#[cfg(target_pointer_width = "64")]
pointer!(usize as u64);

#[cfg(test)]
mod tests {
    //! Pointer-sized extraction and metadata checks.

    #[cfg(target_pointer_width = "16")]
    use core::primitive::u16 as Word;
    #[cfg(target_pointer_width = "32")]
    use core::primitive::u32 as Word;
    #[cfg(target_pointer_width = "64")]
    use core::primitive::u64 as Word;

    use super::{Extract, Extractor};

    #[test]
    fn pointer_metadata_matches_the_target_word() {
        assert_eq!(
            <usize as Extract<4, 11>>::BITSET_WIDTH,
            <Word as Extract<4, 11>>::BITSET_WIDTH
        );
        assert_eq!(
            <usize as Extract<4, 11>>::BITSET_MASK,
            <Word as Extract<4, 11>>::BITSET_MASK as usize
        );
        assert_eq!(
            <usize as Extract<4, 11>>::EXTRACT_MASK,
            <Word as Extract<4, 11>>::EXTRACT_MASK as usize
        );
        assert_eq!(
            <usize as Extract<4, 11>>::FUSE_MASK,
            <Word as Extract<4, 11>>::FUSE_MASK as usize
        );
    }

    #[test]
    fn pointer_merge_preserves_outside_bits_and_returns_the_old_field() {
        let mut value = usize::MAX;

        let previous: u8 = <usize as Extract<4, 11>>::merge(&mut value, 0x12);

        assert_eq!(previous, u8::MAX);
        assert_eq!(<usize as Extract<4, 11>>::extract(&value), 0x12);
        assert_eq!(value, usize::MAX ^ 0xed0);

        let previous = Extractor::<4, 11, usize>::merge(&mut value, 0x34);

        assert_eq!(previous, 0x12);
        assert_eq!(Extractor::<4, 11, usize>::extract(&value), 0x34);
        assert_eq!(value, usize::MAX ^ 0xcb0);
    }

    #[test]
    fn pointer_const_extractor_covers_the_high_and_full_word() {
        const HIGH: usize = usize::BITS as usize - 1;
        const START: usize = usize::BITS as usize - 8;
        const VALUE: u8 = Extractor::<START, HIGH, usize>::extract(&usize::MAX);
        const MERGED: (usize, u8) = {
            let mut value = usize::MAX;

            let previous = Extractor::<START, HIGH, usize>::merge(&mut value, 0);

            (value, previous)
        };
        const FULL: Word = Extractor::<0, HIGH, usize>::extract(&usize::MAX);
        const REPLACED: (usize, Word) = {
            let mut value = usize::MAX;

            let previous = Extractor::<0, HIGH, usize>::merge(&mut value, 0);

            (value, previous)
        };

        assert_eq!(VALUE, u8::MAX);
        assert_eq!(MERGED, (usize::MAX >> 8, u8::MAX));
        assert_eq!(FULL, Word::MAX);
        assert_eq!(REPLACED, (0, Word::MAX));
        assert_eq!(<usize as Extract<0, HIGH>>::BITSET_WIDTH, usize::BITS as usize);
        assert_eq!(<usize as Extract<0, HIGH>>::BITSET_MASK, usize::MAX);
        assert_eq!(<usize as Extract<0, HIGH>>::FUSE_MASK, 0);
    }
}

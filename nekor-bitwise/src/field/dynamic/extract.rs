//! Extraction and merging for runtime-selected integer fields.
//!
//! [`Extract`] supplies field operations for [`Interval`] selections.
//! [`Extractor`] provides their const-compatible primitive implementations.

use core::marker::PhantomData;

use crate::{bit::Bitwise, field::dynamic::select::Interval};

/// An integer supporting runtime-selected field extraction and merging.
pub trait Extract: Bitwise {
    /// Extract the selected bits with the first bit moved to bit zero.
    fn extract(&self, target_interval: Interval<Self>) -> Self;

    /// Replace the selected bits and return their previous extracted value.
    ///
    /// Excess input bits are discarded. Every bit outside the interval is preserved.
    fn merge(&mut self, target_interval: Interval<Self>, target_input: Self) -> Self;
}

/// Const-compatible extraction and merging for a primitive integer.
///
/// The selected [`Interval`] supplies checked inclusive endpoints. Signed
/// integers are projected through their same-width unsigned bit representation.
// NOTE(invariant): None.
pub struct Extractor<B>(PhantomData<fn() -> B>)
where
    B: Extract;

/// Implement runtime and const field operations for primitive integer pairs.
macro_rules! extractor {
    () => {};
    (
        $($target_type:ty as $target_unsigned:ty);+ $(;)?
    ) => {
        crate::reexport::tokel::stream! {
            $(
            impl Extractor<$target_type> {
                /// The source-positioned mask of the selected interval.
                #[inline]
                #[must_use]
                pub const fn mask(target_interval: Interval<$target_type>) -> $target_type {
                    let target_width = target_interval.end() - target_interval.start() + 1;
                    let target_mask = <$target_unsigned>::MAX >> (<$target_unsigned>::BITS - target_width);
                    let target_mask = target_mask << target_interval.start();

                    <$target_type>::from_ne_bytes(target_mask.to_ne_bytes())
                }

                /// A const-compatible counterpart of [`Extract::extract()`].
                #[inline]
                #[must_use]
                pub const fn extract(target_value: &$target_type, target_interval: Interval<$target_type>) -> $target_type {
                    let target_mask = Self::mask(target_interval);
                    let target_mask = <$target_unsigned>::from_ne_bytes(target_mask.to_ne_bytes());
                    let target_value = <$target_unsigned>::from_ne_bytes(target_value.to_ne_bytes());

                    let target_value = target_value & target_mask;
                    let target_value = target_value >> target_interval.start();

                    <$target_type>::from_ne_bytes(target_value.to_ne_bytes())
                }

                /// A const-compatible counterpart of [`Extract::merge()`].
                ///
                /// This returns the previous field value, discards excess input bits,
                /// and preserves every source bit outside the selected interval.
                #[inline]
                pub const fn merge(target_existing: &mut $target_type, target_interval: Interval<$target_type>, target_input: $target_type) -> $target_type {
                    let existing_value = Self::extract(target_existing, target_interval);
                    let target_mask = Self::mask(target_interval);
                    let target_mask = <$target_unsigned>::from_ne_bytes(target_mask.to_ne_bytes());
                    let existing_bits = <$target_unsigned>::from_ne_bytes(target_existing.to_ne_bytes());
                    let target_value = <$target_unsigned>::from_ne_bytes(target_input.to_ne_bytes());

                    let target_value = target_value << target_interval.start();
                    let target_value = target_value & target_mask;
                    let target_value = target_value | (existing_bits & !target_mask);

                    *target_existing = <$target_type>::from_ne_bytes(target_value.to_ne_bytes());

                    existing_value
                }
            }

            impl Extract for $target_type {
                #[inline]
                fn extract(&self, target_interval: Interval<Self>) -> Self {
                    Extractor::<Self>::extract(self, target_interval)
                }

                #[inline]
                fn merge(&mut self, target_interval: Interval<Self>, target_input: Self) -> Self {
                    Extractor::<Self>::merge(self, target_interval, target_input)
                }
            }
            )+
        }
    };
}

extractor! {
    u8 as u8;
    u16 as u16;
    u32 as u32;
    u64 as u64;
    u128 as u128;
    usize as usize;
    i8 as u8;
    i16 as u16;
    i32 as u32;
    i64 as u64;
    i128 as u128;
    isize as usize;
}

#[cfg(test)]
mod tests {
    //! Test const-evaluated runtime intervals and signed bit representations.

    use super::{Extractor, Interval};

    /// Full-width and high-bit intervals remain valid during const evaluation.
    #[test]
    fn const_full_width_and_signed_projection() {
        const FULL: Interval<u128> = match Interval::new(u32::MIN, 127) {
            Some(interval) => interval,
            None => unreachable!(),
        };
        const VALUE: u128 = Extractor::<u128>::extract(&u128::MAX, FULL);
        const MASK: u128 = Extractor::<u128>::mask(FULL);
        const MERGED: (u128, u128) = {
            let mut value = u128::MAX;
            let previous = Extractor::<u128>::merge(&mut value, FULL, 5);

            (value, previous)
        };
        const HIGH: Interval<i128> = match Interval::new(126, 127) {
            Some(interval) => interval,
            None => unreachable!(),
        };
        const SIGNED: i128 = Extractor::<i128>::extract(&i128::MIN, HIGH);

        assert_eq!(VALUE, u128::MAX);
        assert_eq!(MASK, u128::MAX);
        assert_eq!(MERGED, (5, u128::MAX));
        assert_eq!(SIGNED, 2);
    }
}

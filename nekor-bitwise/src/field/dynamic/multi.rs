//! Multi-bit bitwise fields for an already-existing integer.
//!
//! See [`FieldDyn`] and [`FieldDynMut`] for additional information.

use crate::field::{
    counterpart::Counterpart,
    dynamic::{
        extract::{Extract, Extractor},
        select::Interval,
    },
};

/// A managed immutable handle to a runtime-selected bit field in `B`.
///
/// # Value Semantics
///
/// Equality compares the extracted values. Integer extraction uses bit masks
/// and shifts.
///
/// # Const Evaluation
///
/// [`FieldDyn::const_value()`] provides const-compatible primitive extraction.
#[derive(Debug, Copy, Clone)]
// NOTE(invariant): The stored interval proves every selected index is valid for `B`; its width is
// also bounded by `B::BITS`, so rebasing the field at zero remains in bounds.
pub struct FieldDyn<'a, B>(&'a B, Interval<B>)
where
    B: Extract;

impl<'a, B> FieldDyn<'a, B>
where
    B: Extract,
{
    /// Construct a [`FieldDyn`] over an existing integer and bit interval.
    #[inline]
    pub const fn wrap(target_value: &'a B, target_interval: Interval<B>) -> Self {
        Self(target_value, target_interval)
    }

    /// Move the selected interval inside this wrapper to a new interval.
    #[inline]
    #[must_use]
    pub const fn to(self, target_interval: Interval<B>) -> Self {
        let Self(target_value, ..) = self;

        Self(target_value, target_interval)
    }

    /// Determine the selected bit interval.
    #[inline]
    #[must_use]
    pub const fn interval(&self) -> Interval<B> {
        let &Self(.., target_interval) = self;

        target_interval
    }

    /// Extract the selected field and bias its first bit to bit zero.
    #[inline]
    #[must_use]
    pub fn value(&self) -> B {
        let &Self(target_value, target_interval) = self;

        <B as Extract>::extract(target_value, target_interval)
    }
}

impl<B> PartialEq for FieldDyn<'_, B>
where
    B: Extract,
{
    #[inline]
    fn eq(&self, target_value: &Self) -> bool {
        self.value() == target_value.value()
    }
}

impl<B> Eq for FieldDyn<'_, B> where B: Extract + Eq {}

impl<'a, B> Counterpart for FieldDyn<'a, B>
where
    B: Extract,
{
    type Immut = Self;
    type Mut = FieldDynMut<'a, B>;
}

/// A managed mutable handle to a runtime-selected bit field in `B`.
///
/// # Value Semantics
///
/// Equality compares extracted values, as with [`FieldDyn`]. Integer merges
/// preserve every bit outside the selected interval.
///
/// # Const Evaluation
///
/// [`FieldDynMut::const_value()`] and [`FieldDynMut::const_merge()`] provide
/// const-compatible primitive field operations.
#[derive(Debug)]
// NOTE(invariant): The stored interval proves every selected index is valid for `B`; its width is
// also bounded by `B::BITS`, so rebasing the field at zero remains in bounds.
pub struct FieldDynMut<'a, B>(&'a mut B, Interval<B>)
where
    B: Extract;

impl<'a, B> FieldDynMut<'a, B>
where
    B: Extract,
{
    /// Construct a [`FieldDynMut`] over an existing integer and bit interval.
    #[inline]
    pub const fn wrap(target_value: &'a mut B, target_interval: Interval<B>) -> Self {
        Self(target_value, target_interval)
    }

    /// Move the selected interval inside this wrapper to a new interval.
    #[inline]
    #[must_use]
    pub const fn to(self, target_interval: Interval<B>) -> Self {
        let Self(target_value, ..) = self;

        Self(target_value, target_interval)
    }

    /// Determine the selected bit interval.
    #[inline]
    #[must_use]
    pub const fn interval(&self) -> Interval<B> {
        let &Self(.., target_interval) = self;

        target_interval
    }

    /// Extract the selected field and bias its first bit to bit zero.
    #[inline]
    #[must_use]
    pub fn value(&self) -> B {
        let Self(target_value, target_interval) = self;

        <B as Extract>::extract(target_value, *target_interval)
    }

    /// Merge a biased field value into the selected interval.
    ///
    /// This yields the previous selected field value. Input bits exceeding the
    /// field width are discarded, and every bit outside the interval is preserved.
    #[inline]
    #[must_use]
    pub fn merge(&mut self, target_input: B) -> B {
        let &mut Self(ref mut target_value, target_interval) = self;

        <B as Extract>::merge(target_value, target_interval, target_input)
    }
}

impl<B> PartialEq for FieldDynMut<'_, B>
where
    B: Extract,
{
    #[inline]
    fn eq(&self, target_value: &Self) -> bool {
        self.value() == target_value.value()
    }
}

impl<B> Eq for FieldDynMut<'_, B> where B: Extract + Eq {}

impl<'a, B> Counterpart for FieldDynMut<'a, B>
where
    B: Extract,
{
    type Immut = FieldDyn<'a, B>;
    type Mut = Self;
}

/// Implement const-compatible dynamic views for primitive integers.
macro_rules! constant {
    () => {};
    ($($target_type:ident),* $(,)?) => {
        crate::reexport::tokel::stream! {
            $(
            impl FieldDyn<'_, $target_type> {
                /// A const-compatible counterpart of [`FieldDyn::value()`].
                #[inline]
                #[must_use]
                pub const fn const_value(&self) -> $target_type {
                    let &Self(target_value, target_interval) = self;

                    Extractor::<$target_type>::extract(target_value, target_interval)
                }
            }

            impl FieldDynMut<'_, $target_type> {
                /// A const-compatible counterpart of [`FieldDynMut::value()`].
                #[inline]
                #[must_use]
                pub const fn const_value(&self) -> $target_type {
                    let &Self(ref target_value, target_interval) = self;

                    Extractor::<$target_type>::extract(target_value, target_interval)
                }

                /// A const-compatible counterpart of [`FieldDynMut::merge()`].
                #[inline]
                #[must_use]
                pub const fn const_merge(&mut self, target_input: $target_type) -> $target_type {
                    let &mut Self(ref mut target_value, target_interval) = self;

                    Extractor::<$target_type>::merge(target_value, target_interval, target_input)
                }
            }

            )*
        }


    };
}

constant!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);

#[cfg(test)]
mod tests {
    //! Behavioral tests for runtime-selected multi-bit fields.

    use super::{FieldDyn, FieldDynMut};
    use crate::{
        bit::BitOp,
        field::dynamic::select::{Interval, Selected},
    };

    /// Dynamic primitive views can extract and merge during const evaluation.
    #[test]
    fn const_field_operations() {
        const HIGH: Interval<i128> = match Interval::new(126, 127) {
            Some(interval) => interval,
            None => unreachable!(),
        };
        const PROJECTED: i128 = FieldDyn::wrap(&i128::MIN, HIGH).const_value();
        const MERGED: (i128, i128, i128) = {
            let mut value = i128::MIN;
            let mut field = FieldDynMut::wrap(&mut value, HIGH);
            let previous = field.const_merge(1);
            let current = field.const_value();

            (value, previous, current)
        };

        assert_eq!(PROJECTED, 2);
        assert_eq!(MERGED, (i128::MAX / 2 + 1, 2, 1));
    }

    /// Different source integers and intervals can project the same value.
    #[test]
    fn projected_equality() {
        let low = Interval::<u8>::new(u32::MIN, 3).expect("the low nibble is in bounds");
        let high = Interval::<u8>::new(4, 7).expect("the high nibble is in bounds");
        let left = 0x01_u8;
        let right = 0x10_u8;

        assert_eq!(FieldDyn::wrap(&left, low), FieldDyn::wrap(&right, high));

        let mut left = left;
        let mut right = right;

        assert_eq!(FieldDynMut::wrap(&mut left, low), FieldDynMut::wrap(&mut right, high));
    }

    /// Verify primitive field operations against individual-bit projections.
    macro_rules! ranges {
        () => {};
        ($($target_type:ident),* $(,)?) => {
            crate::reexport::tokel::stream! {
            {
                $(
                for (start, end) in [
                    (u32::MIN, 1),
                    (u32::MIN, $target_type::BITS - 1),
                    (1, $target_type::BITS - 1),
                    ($target_type::BITS - 2, $target_type::BITS - 1),
                ] {
                    let interval = Interval::<$target_type>::new(start, end).expect("the endpoints are in bounds");

                    for source in [0, $target_type::MIN, $target_type::MAX, !0] {
                        let mut expected: $target_type = 0;

                        for index in start..=end {
                            let source_bit = Selected::<$target_type>::new(index);
                            let output_bit = Selected::<$target_type>::new(index - start);
                            let state = BitOp::get(&source, source_bit);

                            _ = BitOp::set(&mut expected, output_bit, state);
                        }

                        assert_eq!(FieldDyn::wrap(&source, interval).value(), expected);

                        for input in [0, $target_type::MIN, $target_type::MAX, !0] {
                            let mut expected_source = source;
                            let mut actual_source = source;

                            for index in start..=end {
                                let source_bit = Selected::<$target_type>::new(index);
                                let input_bit = Selected::<$target_type>::new(index - start);
                                let state = BitOp::get(&input, input_bit);

                                _ = BitOp::set(&mut expected_source, source_bit, state);
                            }

                            let previous = FieldDynMut::wrap(&mut actual_source, interval).merge(input);

                            assert_eq!(previous, expected);
                            assert_eq!(actual_source, expected_source);
                        }
                    }
                }

                )*
            }
        }


        };
    }

    /// Full-width and sign-bit fields retain their raw bit patterns.
    #[test]
    fn primitive_range_boundaries() {
        ranges!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);
    }

    /// Every byte value and interval projects identically to its individual bits.
    #[test]
    fn exhaustive_byte_projection() {
        for start in u32::MIN..u8::BITS {
            for end in start + 1..u8::BITS {
                let interval = Interval::<u8>::new(start, end).expect("the interval is in bounds");

                for source in u8::MIN..=u8::MAX {
                    let mut expected = u8::MIN;

                    for index in start..=end {
                        let source_bit = Selected::<u8>::new(index);
                        let output_bit = Selected::<u8>::new(index - start);
                        let state = BitOp::get(&source, source_bit);

                        _ = BitOp::set(&mut expected, output_bit, state);
                    }

                    assert_eq!(FieldDyn::wrap(&source, interval).value(), expected);
                }
            }
        }
    }

    /// Verify that extraction biases the selected field to bit zero.
    #[test]
    fn extract_selected_field() {
        let target_value = 0b1101_0110_u8;
        let target_interval = Interval::<u8>::new(1, 4);

        assert!(target_interval.is_some());

        if let Some(target_interval) = target_interval {
            let target_field = FieldDyn::wrap(&target_value, target_interval);

            assert_eq!(target_field.value(), 0b1011);
        }
    }

    /// Verify that an immutable field can be retargeted without changing its
    /// underlying integer.
    #[test]
    fn retarget_selected_field() {
        let target_value = 0b1111_0000_u8;
        let low_interval = Interval::<u8>::new(0, 3);
        let high_interval = Interval::<u8>::new(4, 7);

        assert!(low_interval.is_some());
        assert!(high_interval.is_some());

        if let (Some(low_interval), Some(high_interval)) = (low_interval, high_interval) {
            let target_field = FieldDyn::wrap(&target_value, low_interval);

            assert_eq!(target_field.value(), 0);
            assert_eq!(target_field.to(high_interval).value(), 0b1111);
        }
    }

    /// Verify that merging replaces only the selected bits and yields the old
    /// field value.
    #[test]
    fn merge_selected_field() {
        let mut target_value = 0b1100_0011_u8;
        let target_interval = Interval::<u8>::new(2, 5);

        assert!(target_interval.is_some());

        if let Some(target_interval) = target_interval {
            let mut target_field = FieldDynMut::wrap(&mut target_value, target_interval);
            let previous_value = target_field.merge(0b0101);

            assert_eq!(previous_value, 0);
            assert_eq!(target_field.value(), 0b0101);
        }

        assert_eq!(target_value, 0b1101_0111);
    }

    /// Verify that merge ignores input bits above the selected field width.
    #[test]
    fn merge_truncates_input_to_field_width() {
        let mut target_value = 0_u8;
        let target_interval = Interval::<u8>::new(2, 4);

        assert!(target_interval.is_some());

        if let Some(target_interval) = target_interval {
            let mut target_field = FieldDynMut::wrap(&mut target_value, target_interval);

            _ = target_field.merge(0b1111_1101);
        }

        assert_eq!(target_value, 0b0001_0100);
    }
}

//! Multi-bit bitwise fields for an already-existing integer.
//!
//! See [`FieldDyn`] and [`FieldDynMut`] for additional information.

use crate::{
    bit::BitOp,
    field::{
        counterpart::Counterpart,
        dynamic::select::{Interval, Selected},
    },
    prelude::State,
};

/// A managed immutable handle to a runtime-selected bit field in `B`.
// NOTE(invariant): The stored interval proves every selected index is valid for
// `B`. Its width cannot exceed `B::BITS`, so biasing those bits to indices
// starting at zero also remains within the bitwise bounds of `B`.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct FieldDyn<'a, B>(&'a B, Interval<B>)
where
    B: BitOp;

impl<'a, B> FieldDyn<'a, B>
where
    B: BitOp,
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

        Self::extract(target_value, target_interval)
    }

    /// Extract a runtime-selected interval from an integer.
    fn extract(target_value: &B, target_interval: Interval<B>) -> B {
        let output_zero = Selected::<B>::new(0);
        let mut target_output = <B as BitOp>::single(output_zero);
        _ = <B as BitOp>::set(&mut target_output, output_zero, State::Cleared);

        let mut source_index = target_interval.start();
        let mut output_index = 0;

        while source_index <= target_interval.end() {
            let source_bit = Selected::<B>::new(source_index);
            let output_bit = Selected::<B>::new(output_index);

            if <B as BitOp>::get(target_value, source_bit) == State::Set {
                _ = <B as BitOp>::set(&mut target_output, output_bit, State::Set);
            }

            source_index += 1;
            output_index += 1;
        }

        target_output
    }
}

impl<'a, B> Counterpart for FieldDyn<'a, B>
where
    B: BitOp,
{
    type Mut = FieldDynMut<'a, B>;

    type Immut = Self;
}

/// A managed mutable handle to a runtime-selected bit field in `B`.
// NOTE(invariant): The stored interval proves every selected index is valid for
// `B`. Its width cannot exceed `B::BITS`, so biasing those bits to indices
// starting at zero also remains within the bitwise bounds of `B`.
#[derive(Debug, Eq, PartialEq)]
pub struct FieldDynMut<'a, B>(&'a mut B, Interval<B>)
where
    B: BitOp;

impl<'a, B> FieldDynMut<'a, B>
where
    B: BitOp,
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

        FieldDyn::<B>::extract(target_value, *target_interval)
    }

    /// Merge a biased field value into the selected interval.
    ///
    /// This yields the previous selected field value.
    #[inline]
    #[must_use]
    pub fn merge(&mut self, target_input: B) -> B {
        let &mut Self(ref mut target_value, target_interval) = self;
        let previous_value = FieldDyn::<B>::extract(target_value, target_interval);
        let mut target_index = target_interval.start();
        let mut input_index = 0;

        while target_index <= target_interval.end() {
            let target_bit = Selected::<B>::new(target_index);
            let input_bit = Selected::<B>::new(input_index);
            let target_state = <B as BitOp>::get(&target_input, input_bit);

            _ = <B as BitOp>::set(target_value, target_bit, target_state);

            target_index += 1;
            input_index += 1;
        }

        previous_value
    }
}

impl<'a, B> Counterpart for FieldDynMut<'a, B>
where
    B: BitOp,
{
    type Mut = Self;

    type Immut = FieldDyn<'a, B>;
}

#[cfg(test)]
mod tests {
    //! Behavioral tests for runtime-selected multi-bit fields.

    use super::{FieldDyn, FieldDynMut};
    use crate::field::dynamic::select::Interval;

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

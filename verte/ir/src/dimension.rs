//! Exact physical dimensions and declaration-qualified nominal count factors.

use alloc::collections::BTreeMap;

use num_bigint::BigInt;
use num_traits::Zero as _;

use crate::source::Origin;

/// The fixed physical dimension basis used by Verte quantities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// Time.
    Time,

    /// Length.
    Length,

    /// Mass.
    Mass,

    /// Electric current.
    Current,

    /// Thermodynamic temperature.
    Temperature,

    /// Amount of substance.
    Substance,

    /// Luminous intensity.
    Luminosity,

    /// Information measured in bits.
    Information,
}

impl Axis {
    /// Select this physical component in the fixed dimension basis.
    #[inline]
    pub const fn index(self) -> usize {
        match self {
            Self::Time => 0,
            Self::Length => 1,
            Self::Mass => 2,
            Self::Current => 3,
            Self::Temperature => 4,
            Self::Substance => 5,
            Self::Luminosity => 6,
            Self::Information => 7,
        }
    }
}

/// A semantic dimension that cannot be erased by physical dimensional equality.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tag {
    /// A nominal count declared by a source.
    Count(Origin),

    /// Symbols, distinct from untagged events or information bits.
    Symbol,

    /// Angles measured in degrees without an implicit conversion to radians.
    Degree,

    /// Angles measured in radians without an implicit conversion to degrees.
    Radian,
}

/// A normalized dimension signature with arbitrary integral exponents.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
// NOTE(invariant): Nominal factors with a zero exponent are absent from `nominal`.
pub struct Dimension {
    /// Exponents in the fixed physical basis.
    physical: [BigInt; 8],

    /// Count dimensions identified by their canonical declarations.
    nominal: BTreeMap<Tag, BigInt>,
}

impl Dimension {
    /// Construct a dimensionless signature.
    #[inline]
    pub fn scalar() -> Self {
        Self {
            physical: core::array::from_fn(|_| BigInt::from(0_u8)),
            nominal: BTreeMap::new(),
        }
    }

    /// Construct a physical signature without nominal semantic factors.
    #[inline]
    pub const fn physical(physical: [BigInt; 8]) -> Self {
        Self {
            physical,
            nominal: BTreeMap::new(),
        }
    }

    /// Construct a physical basis dimension.
    #[inline]
    pub fn axis(axis: Axis) -> Self {
        let mut result = Self::scalar();

        let Self { physical, .. } = &mut result;

        physical[axis.index()] = BigInt::from(1_u8);

        result
    }

    /// Construct a semantic basis dimension.
    #[inline]
    pub fn tag(tag: Tag) -> Self {
        let mut result = Self::scalar();

        let Self { nominal, .. } = &mut result;

        nominal.insert(tag, BigInt::from(1_u8));

        result
    }

    /// Borrow the exponent of a physical basis component.
    #[inline]
    pub const fn exponent(&self, axis: Axis) -> &BigInt {
        let Self { physical, .. } = self;

        &physical[axis.index()]
    }

    /// Iterate over semantic factors in stable tag order.
    #[inline]
    pub fn tags(&self) -> impl Iterator<Item = (&Tag, &BigInt)> {
        let Self { nominal, .. } = self;

        nominal.iter()
    }

    /// Multiply dimensions while preserving every nominal factor.
    #[inline]
    pub fn multiply(&self, other: &Self) -> Self {
        let Self {
            physical: left,
            nominal,
        } = self;

        let Self {
            physical: right,
            nominal: other_nominal,
        } = other;

        let physical = core::array::from_fn(|index| &left[index] + &right[index]);
        let mut nominal = nominal.clone();

        for (origin, exponent) in other_nominal {
            let value = nominal.entry(origin.clone()).or_insert_with(|| BigInt::from(0_u8));

            *value += exponent;
        }

        nominal.retain(|_, exponent| !exponent.is_zero());

        Self { physical, nominal }
    }

    /// Raise every dimension factor to an arbitrary integral power.
    #[inline]
    pub fn power(&self, exponent: &BigInt) -> Self {
        let Self { physical, nominal } = self;

        let physical = core::array::from_fn(|index| &physical[index] * exponent);
        let nominal = nominal
            .iter()
            .filter_map(|(origin, value)| {
                let value = value * exponent;

                (!value.is_zero()).then(|| (origin.clone(), value))
            })
            .collect();

        Self { physical, nominal }
    }

    /// Divide dimensions, removing factors whose exponents cancel exactly.
    #[inline]
    pub fn divide(&self, other: &Self) -> Self {
        self.multiply(&other.power(&BigInt::from(-1)))
    }
}

impl Default for Dimension {
    #[inline]
    fn default() -> Self {
        Self::scalar()
    }
}

#[cfg(test)]
mod tests {
    use aldebaran::prelude::Span;
    use num_bigint::BigInt;

    use super::{Axis, Dimension, Tag};
    use crate::source::{Id, Origin};

    #[test]
    fn physical_and_nominal_factors_cancel_without_erasing_other_domains() {
        let page = Dimension::tag(Tag::Count(Origin::new(Id::new("units"), Span::unit(0))));
        let sector = Dimension::tag(Tag::Count(Origin::new(Id::new("units"), Span::unit(1))));
        let size = Dimension::axis(Axis::Information);
        let duration = Dimension::axis(Axis::Time);
        let rate = size.divide(&duration);

        assert_eq!(rate.multiply(&duration), size);
        assert_eq!(page.divide(&page), Dimension::scalar());
        assert_ne!(page, sector);
        assert_ne!(page.divide(&sector), Dimension::scalar());
    }

    #[test]
    fn exponent_arithmetic_is_not_limited_to_machine_widths() {
        let exponent = BigInt::from(1_u8) << 256_usize;
        let dimension = Dimension::axis(Axis::Time).power(&exponent);

        assert_eq!(dimension.exponent(Axis::Time), &exponent);
        assert_eq!(dimension.power(&BigInt::from(0)), Dimension::scalar());
    }
}

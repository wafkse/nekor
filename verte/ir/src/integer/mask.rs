//! Unsigned bit masks with declared widths and a type distinct from ordinary integers.

use num_bigint::BigInt;

use super::{Arithmetic, Binary as IntegerBinary, Format, Integer, Representability, Shift, Width};
use crate::{number::Number, source::Origin};

/// A nominal `mask[U]` value with checked unsigned representability.
#[derive(Debug, Clone, PartialEq, Eq)]
// NOTE(invariant): The integer has an unsigned format.
pub struct Mask(Integer);

impl Mask {
    /// Check a mathematical magnitude against the mask's explicit unsigned width.
    #[inline]
    pub fn new(width: Width, value: Number, origin: Origin) -> Result<Self, Representability> {
        Integer::new(Format::Unsigned(width), value, origin).map(Self)
    }

    /// Retrieve the nominal mask width without converting to an integer value.
    #[inline]
    pub const fn width(&self) -> Width {
        let Self(value) = self;

        value.format().width()
    }

    /// Borrow the exact checked bit magnitude.
    #[inline]
    pub const fn value(&self) -> &Number {
        let Self(value) = self;

        value.value()
    }

    /// Combine masks of the same width without implicit integer conversion.
    #[inline]
    pub fn apply(&self, operation: Binary, other: &Self, origin: Origin) -> Result<Self, Arithmetic> {
        let Self(left) = self;

        let Self(right) = other;

        let operation = match operation {
            Binary::And => IntegerBinary::And,
            Binary::Or => IntegerBinary::Or,
            Binary::Xor => IntegerBinary::Xor,
        };

        left.apply(operation, right, origin).map(Self)
    }

    /// Complement the declared unsigned width without introducing high bits.
    #[inline]
    pub fn invert(&self, origin: Origin) -> Result<Self, Representability> {
        let Self(value) = self;

        value.invert(origin).map(Self)
    }

    /// Shift with the same amount and representability checks as unsigned integers.
    #[inline]
    pub fn shift(&self, direction: Shift, amount: &BigInt, origin: Origin) -> Result<Self, Arithmetic> {
        let Self(value) = self;

        value.shift(direction, amount, origin).map(Self)
    }
}

/// Bitwise operations admitted for nominal masks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binary {
    /// Intersection of set bits.
    And,

    /// Union of set bits.
    Or,

    /// Symmetric difference of set bits.
    Xor,
}

#[cfg(test)]
mod tests {
    use aldebaran::prelude::Span;
    use num_bigint::BigInt;

    use super::{Binary, Mask};
    use crate::{
        integer::{Shift, Width},
        number::Number,
        source::{Id, Origin},
    };

    #[test]
    fn masks_preserve_width_and_complement_only_their_declared_bits() {
        let origin = Origin::new(Id::new("mask"), Span::unit(0));

        for width in [Width::W8, Width::W16, Width::W32, Width::W64, Width::W128] {
            let low = Mask::new(width, Number::integer(1), origin.clone()).expect("low bit");
            let inverse = low.invert(origin.clone()).expect("width complement");
            let maximum = Mask::new(
                width,
                Number::integer((BigInt::from(1_u8) << width.bits()) - 1_u8),
                origin.clone(),
            )
            .expect("maximum");

            assert_eq!(low.apply(Binary::Or, &inverse, origin.clone()).expect("union"), maximum);
            assert_eq!(low.apply(Binary::Xor, &inverse, origin.clone()).expect("xor"), maximum);
            assert_eq!(
                low.apply(Binary::And, &inverse, origin.clone())
                    .expect("intersection")
                    .value(),
                &Number::integer(0)
            );
            assert_eq!(inverse.invert(origin.clone()).expect("involution"), low);
            assert_eq!(low.width(), width);
            assert!(maximum.shift(Shift::Left, &1.into(), origin.clone()).is_err());
        }

        let narrow = Mask::new(Width::W8, Number::integer(1), origin.clone()).expect("narrow");
        let wide = Mask::new(Width::W16, Number::integer(1), origin.clone()).expect("wide");

        assert!(narrow.apply(Binary::Or, &wide, origin.clone()).is_err());
        assert!(Mask::new(Width::W8, Number::integer(-1), origin.clone()).is_err());
        assert!(Mask::new(Width::W8, Number::integer(256), origin).is_err());
    }
}

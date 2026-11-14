//! Declared integer formats and checked integral values.

use aldebaran::{
    prelude::Report,
    report::annotated::{InlineAnnotations, label::Label},
};
use fack::prelude::Error;
use num_bigint::BigInt;
use num_traits::{Signed as _, ToPrimitive as _, Zero as _};

use crate::{
    number::{Number, ZeroDivisor},
    source::Origin,
};

pub mod mask;

/// Language-defined widths, independent of the compilation host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Width {
    /// Eight bits.
    W8,

    /// Sixteen bits.
    W16,

    /// Thirty-two bits.
    W32,

    /// Sixty-four bits.
    W64,

    /// One hundred twenty-eight bits.
    W128,
}

impl Width {
    /// Retrieve the declared width in bits.
    #[inline]
    pub const fn bits(self) -> usize {
        match self {
            Self::W8 => 8,
            Self::W16 => 16,
            Self::W32 => 32,
            Self::W64 => 64,
            Self::W128 => 128,
        }
    }
}

/// A signed or unsigned fixed-width integer type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Format {
    /// Two's-complement signed representability.
    Signed(Width),

    /// Nonnegative unsigned representability.
    Unsigned(Width),
}

impl Format {
    /// Retrieve the width without changing signedness or interpreting a value.
    #[inline]
    pub const fn width(self) -> Width {
        match self {
            Self::Signed(width) | Self::Unsigned(width) => width,
        }
    }

    /// Determine exact representability without narrowing the candidate first.
    #[inline]
    pub fn accepts(self, number: &Number) -> bool {
        let value = number.magnitude();

        if !value.is_integer() {
            return false;
        }

        let integer = value.to_integer();

        let (minimum, maximum) = match self {
            Self::Signed(width) => {
                let boundary = BigInt::from(1_u8) << (width.bits() - 1);

                (-&boundary, boundary - 1_u8)
            },
            Self::Unsigned(width) => {
                let boundary = BigInt::from(1_u8) << width.bits();

                (BigInt::from(0_u8), boundary - 1_u8)
            },
        };

        minimum <= integer && integer <= maximum
    }
}

/// An integral magnitude proven representable in its declared format.
#[derive(Debug, Clone, PartialEq, Eq)]
// NOTE(invariant): `value` is integral and representable in `format`.
pub struct Integer {
    /// Declared signedness and width.
    format: Format,

    /// Exact mathematical value without conversion to the host integer width.
    value: Number,
}

impl Integer {
    /// Check representability after evaluating the mathematical expression.
    #[inline]
    pub fn new(format: Format, value: Number, origin: Origin) -> Result<Self, Representability> {
        if format.accepts(&value) {
            Ok(Self { format, value })
        } else {
            let labels = InlineAnnotations::new(
                Label::new(
                    "this value is not an integer representable in the declared format",
                    origin.span(),
                ),
                [],
            );

            Err(Representability { origin, format, labels })
        }
    }

    /// Retrieve the declared integer format.
    #[inline]
    pub const fn format(&self) -> Format {
        let &Self { format, .. } = self;

        format
    }

    /// Borrow the exact checked value.
    #[inline]
    pub const fn value(&self) -> &Number {
        let Self { value, .. } = self;

        value
    }

    /// Evaluate a binary operation without implicit signedness or width conversions.
    #[inline]
    pub fn apply(&self, operation: Binary, other: &Self, origin: Origin) -> Result<Self, Arithmetic> {
        let Self { format, value: left } = self;

        let Self {
            format: right_format,
            value: right,
        } = other;

        match (format, right_format) {
            (left_format, right_format) if left_format == right_format => {
                let value = match operation {
                    Binary::Add => Ok(left.add(right)),
                    Binary::Subtract => Ok(left.subtract(right)),
                    Binary::Multiply => Ok(left.multiply(right)),
                    Binary::Divide => left.divide(right, origin.clone()).map_err(Arithmetic::Zero),
                    Binary::Remainder if right.magnitude().is_zero() => {
                        Err(Arithmetic::Zero(ZeroDivisor::new(origin.clone())))
                    },
                    Binary::Remainder => Ok(Number::integer(
                        left.magnitude().to_integer() % right.magnitude().to_integer(),
                    )),
                    Binary::And => Ok(Number::integer(
                        left.magnitude().to_integer() & right.magnitude().to_integer(),
                    )),
                    Binary::Or => Ok(Number::integer(
                        left.magnitude().to_integer() | right.magnitude().to_integer(),
                    )),
                    Binary::Xor => Ok(Number::integer(
                        left.magnitude().to_integer() ^ right.magnitude().to_integer(),
                    )),
                }?;

                Self::new(*format, value, origin).map_err(Arithmetic::Representation)
            },
            _ => Err(Arithmetic::Operand(Operand::new(origin, Requirement::Format))),
        }
    }

    /// Negate mathematically and reject a result outside the original format.
    #[inline]
    pub fn negate(&self, origin: Origin) -> Result<Self, Representability> {
        let Self { format, value } = self;

        Self::new(*format, value.negate(), origin)
    }

    /// Complement the declared bits using signed two's-complement semantics.
    #[inline]
    pub fn invert(&self, origin: Origin) -> Result<Self, Representability> {
        let Self { format, value } = self;

        let integer = value.magnitude().to_integer();
        let value = match format {
            Format::Signed(_) => !integer,
            Format::Unsigned(width) => {
                let maximum = (BigInt::from(1_u8) << width.bits()) - 1_u8;

                maximum ^ integer
            },
        };
        let value = Number::integer(value);

        Self::new(*format, value, origin)
    }

    /// Shift by a nonnegative amount strictly smaller than the declared width.
    ///
    /// Left shifts use checked mathematical multiplication.
    /// Signed right shifts extend the sign bit.
    #[inline]
    pub fn shift(&self, direction: Shift, amount: &BigInt, origin: Origin) -> Result<Self, Arithmetic> {
        let Self { format, value } = self;

        match amount.to_usize() {
            Some(amount) if amount < format.width().bits() => {
                let integer = value.magnitude().to_integer();
                let value = match direction {
                    Shift::Left => integer << amount,
                    Shift::Right => integer >> amount,
                };
                let value = Number::integer(value);

                Self::new(*format, value, origin).map_err(Arithmetic::Representation)
            },
            _ => Err(Arithmetic::Operand(Operand::new(origin, Requirement::Shift))),
        }
    }

    /// Raise to an integral power, checking each needed product.
    ///
    /// Exponents have no resource cap. Intermediate overflow rejects a typed result
    /// without first allocating its arbitrarily large mathematical expansion.
    #[inline]
    pub fn power(&self, exponent: &BigInt, origin: Origin) -> Result<Self, Arithmetic> {
        let mut base = if exponent.is_negative() {
            let reciprocal = Number::integer(1)
                .divide(self.value(), origin.clone())
                .map_err(Arithmetic::Zero)?;

            Self::new(self.format(), reciprocal, origin.clone()).map_err(Arithmetic::Representation)?
        } else {
            self.clone()
        };
        let mut remaining = exponent.magnitude().clone();
        let mut result =
            Self::new(self.format(), Number::integer(1), origin.clone()).map_err(Arithmetic::Representation)?;

        while !remaining.is_zero() {
            if remaining.bit(0) {
                result = result.apply(Binary::Multiply, &base, origin.clone())?;
            }

            remaining >>= 1_usize;

            if !remaining.is_zero() {
                base = base.apply(Binary::Multiply, &base, origin.clone())?;
            }
        }

        Ok(result)
    }
}

/// Checked binary operations on identically declared integer formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binary {
    /// Mathematical addition.
    Add,

    /// Mathematical subtraction.
    Subtract,

    /// Mathematical multiplication.
    Multiply,

    /// Exact division whose result must remain an integer.
    Divide,

    /// Remainder with the dividend's sign.
    Remainder,

    /// Bitwise intersection.
    And,

    /// Bitwise union.
    Or,

    /// Bitwise exclusive union.
    Xor,
}

/// Shift direction independent of operand signedness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shift {
    /// Checked multiplication by a power of two.
    Left,

    /// Arithmetic shifting for signed values, logical shifting for unsigned values.
    Right,
}

/// A typed integer operation failed representability or an operand requirement.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[report(forward)]
pub enum Arithmetic {
    /// The exact result cannot inhabit the declared format.
    #[error(transparent(0))]
    Representation(Representability),

    /// The divisor or remainder modulus is zero.
    #[error(transparent(0))]
    Zero(ZeroDivisor),

    /// The operation has incompatible formats or an invalid shift.
    #[error(transparent(0))]
    Operand(Operand),
}

/// Operand requirements independent of result representability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Requirement {
    /// Binary operands must share signedness and width.
    Format,

    /// The shift amount must be nonnegative and less than the operand width.
    Shift,
}

impl Requirement {
    /// Explain the violated integer operand requirement.
    #[inline]
    const fn message(self) -> &'static str {
        match self {
            Self::Format => "integer operands must have the same declared signedness and width",
            Self::Shift => "a shift amount must be nonnegative and smaller than the declared integer width",
        }
    }
}

/// An integer operation's operands violate its arithmetic contract.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("invalid integer operands")]
#[report(title = "invalid integer operands", annotations = labels)]
// NOTE(invariant): The label uses the coordinates qualified by `origin`.
pub struct Operand {
    /// Source-qualified operand or operation location.
    origin: Origin,

    /// Failed operand requirement.
    requirement: Requirement,

    /// Primary operand location.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

impl Operand {
    /// Report the failed arithmetic requirement at the operation location.
    #[inline]
    const fn new(origin: Origin, requirement: Requirement) -> Self {
        let labels = InlineAnnotations::new(Label::new(requirement.message(), origin.span()), []);

        Self {
            origin,
            requirement,
            labels,
        }
    }

    /// Retrieve the source-qualified operation location.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        let Self { origin, .. } = self;

        origin
    }

    /// Retrieve the operand requirement independently of diagnostic wording.
    #[inline]
    pub const fn requirement(&self) -> Requirement {
        let &Self { requirement, .. } = self;

        requirement
    }
}

/// A candidate is fractional or outside its declared integer range.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("value is not representable in its declared integer format")]
#[report(title = "integer representability failure", annotations = labels)]
// NOTE(invariant): The label uses the byte coordinates of `origin`.
pub struct Representability {
    /// Canonical location of the failed conversion.
    origin: Origin,

    /// Format whose representability check failed.
    format: Format,

    /// Candidate location.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

impl Representability {
    /// Retrieve the canonical location of the candidate.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        let Self { origin, .. } = self;

        origin
    }

    /// Retrieve the rejected target format.
    #[inline]
    pub const fn format(&self) -> Format {
        let &Self { format, .. } = self;

        format
    }
}

#[cfg(test)]
mod tests {
    use aldebaran::prelude::Span;
    use num_bigint::BigInt;

    use super::{Arithmetic, Binary, Format, Integer, Requirement, Shift, Width};
    use crate::{
        number::Number,
        source::{Id, Origin},
    };

    #[test]
    fn every_declared_width_checks_both_bounds_before_narrowing() {
        for width in [Width::W8, Width::W16, Width::W32, Width::W64, Width::W128] {
            let signed = Format::Signed(width);
            let unsigned = Format::Unsigned(width);
            let boundary = BigInt::from(1_u8) << (width.bits() - 1);
            let unsigned_boundary = &boundary * 2_u8;

            assert!(signed.accepts(&Number::integer(-&boundary)));
            assert!(signed.accepts(&Number::integer(&boundary - 1_u8)));
            assert!(!signed.accepts(&Number::integer(-&boundary - 1_u8)));
            assert!(!signed.accepts(&Number::integer(boundary)));
            assert!(unsigned.accepts(&Number::integer(0)));
            assert!(unsigned.accepts(&Number::integer(&unsigned_boundary - 1_u8)));
            assert!(!unsigned.accepts(&Number::integer(unsigned_boundary)));
            assert!(!unsigned.accepts(&Number::integer(-1)));
        }
    }

    #[test]
    fn operations_check_results_without_wrapping_or_reinterpreting_signs() {
        let origin = Origin::new(Id::new("integer"), Span::unit(0));

        for width in [Width::W8, Width::W16, Width::W32, Width::W64, Width::W128] {
            let format = Format::Signed(width);
            let boundary = BigInt::from(1_u8) << (width.bits() - 1);
            let minimum = Integer::new(format, Number::integer(-&boundary), origin.clone()).expect("minimum");
            let maximum = Integer::new(format, Number::integer(&boundary - 1_u8), origin.clone()).expect("maximum");
            let one = Integer::new(format, Number::integer(1), origin.clone()).expect("one");
            let minus_one = Integer::new(format, Number::integer(-1), origin.clone()).expect("negative one");

            assert!(maximum.apply(Binary::Add, &one, origin.clone()).is_err());
            assert!(minimum.apply(Binary::Subtract, &one, origin.clone()).is_err());
            assert!(minimum.apply(Binary::Divide, &minus_one, origin.clone()).is_err());
            assert!(minimum.negate(origin.clone()).is_err());
            assert_eq!(minimum.power(&1.into(), origin.clone()).expect("first power"), minimum);
            assert_eq!(
                minus_one.invert(origin.clone()).expect("complement").value(),
                &Number::integer(0)
            );
            assert_eq!(
                minimum
                    .shift(Shift::Right, &(width.bits() - 1).into(), origin.clone())
                    .expect("arithmetic shift"),
                minus_one
            );
        }

        let signed = Integer::new(Format::Signed(Width::W8), Number::integer(1), origin.clone()).expect("signed");
        let unsigned = Integer::new(Format::Unsigned(Width::W8), Number::integer(1), origin.clone()).expect("unsigned");

        assert!(
            matches!(signed.apply(Binary::Add, &unsigned, origin), Err(Arithmetic::Operand(error)) if error.requirement() == Requirement::Format)
        );
    }

    #[test]
    fn exact_division_remainders_shifts_and_large_exponents_have_defined_failures() {
        let origin = Origin::new(Id::new("integer"), Span::unit(5));
        let format = Format::Signed(Width::W8);
        let three = Integer::new(format, Number::integer(3), origin.clone()).expect("three");
        let two = Integer::new(format, Number::integer(2), origin.clone()).expect("two");
        let zero = Integer::new(format, Number::integer(0), origin.clone()).expect("zero");
        let negative = Integer::new(format, Number::integer(-7), origin.clone()).expect("negative");

        assert!(three.apply(Binary::Divide, &two, origin.clone()).is_err());
        assert!(three.apply(Binary::Divide, &zero, origin.clone()).is_err());
        assert!(three.apply(Binary::Remainder, &zero, origin.clone()).is_err());
        assert_eq!(
            negative
                .apply(Binary::Remainder, &three, origin.clone())
                .expect("signed remainder")
                .value(),
            &Number::integer(-1)
        );
        assert!(three.shift(Shift::Left, &(-1).into(), origin.clone()).is_err());
        assert!(three.shift(Shift::Right, &8.into(), origin.clone()).is_err());
        assert!(
            three
                .shift(Shift::Left, &(BigInt::from(1_u8) << 256_usize), origin.clone())
                .is_err()
        );
        assert!(three.power(&8.into(), origin.clone()).is_err());
        assert!(three.power(&(-1).into(), origin.clone()).is_err());
        assert!(zero.power(&(-1).into(), origin.clone()).is_err());

        let exponent = BigInt::from(1_u8) << 256_usize;
        let minus_one = Integer::new(format, Number::integer(-1), origin.clone()).expect("negative one");

        assert_eq!(
            minus_one
                .power(&exponent, origin.clone())
                .expect("large even power")
                .value(),
            &Number::integer(1)
        );
        assert_eq!(
            minus_one
                .power(&(-exponent - 1_u8), origin)
                .expect("negative odd power"),
            minus_one
        );
    }
}

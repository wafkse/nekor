//! Finite IEEE values with deterministic, format-preserving arithmetic.
//!
//! Exact magnitudes are rounded directly to the selected format with nearest,
//! ties-to-even rounding. Arithmetic uses the same rounding through software
//! IEEE operations, independently of the host's floating-point environment.

use core::cmp::Ordering;

use aldebaran::{
    ice::Ice,
    prelude::Report,
    report::annotated::{InlineAnnotations, label::Label},
};
use fack::prelude::Error;
use num_integer::Integer as _;
use num_traits::{Signed as _, ToPrimitive as _};
use rustc_apfloat::{
    Float as Ieee, Round,
    ieee::{Double, Single},
};

use crate::{number::Number, source::Origin};

/// The explicit IEEE precision of a declared floating-point type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// Binary32 with a 24-bit significand.
    Single,

    /// Binary64 with a 53-bit significand.
    Double,
}

/// A finite IEEE value with a declared format and support for signed zero.
#[derive(Debug, Clone, Copy, PartialEq)]
// NOTE(invariant): The wrapped IEEE value is finite.
pub struct Float(Value);

/// Format-specific software values, inaccessible before finite admission.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Value {
    /// A binary32 value.
    Single(Single),

    /// A binary64 value.
    Double(Double),
}

impl Eq for Float {}

impl Float {
    /// Recover the exact rational value of the stored IEEE magnitude.
    ///
    /// This loses only the sign distinction between the two zeros. It does not
    /// round, narrow a subnormal, or reinterpret the declared precision.
    #[inline]
    pub fn exact(self) -> Number {
        let Self(value) = self;

        match value {
            Value::Single(value) => Self::rational(value),
            Value::Double(value) => Self::rational(value),
        }
    }

    /// Extract an integral significand through IEEE arithmetic operations.
    fn rational<F>(value: F) -> Number
    where
        F: Ieee,
    {
        if value.is_zero() {
            Number::integer(0)
        } else {
            let precision = Ice::some(i32::try_from(F::PRECISION).ok());
            let scale = value.ilogb() - precision + 1;

            // NOTE: Scaling to the format's significand produces an exact integer of at most 53
            // bits.
            let significand = value.abs().scalbn(-scale).to_u128(128).value;
            let exponent = Ice::some(isize::try_from(scale).ok());
            let magnitude = Number::binary(significand, exponent);

            if value.is_negative() {
                magnitude.negate()
            } else {
                magnitude
            }
        }
    }

    /// Round an exact magnitude once, directly to the declared precision.
    #[inline]
    pub fn new(format: Format, number: &Number, origin: Origin) -> Result<Self, Failure> {
        let value = match format {
            Format::Single => Value::Single(Self::rounded::<Single>(number)),
            Format::Double => Value::Double(Self::rounded::<Double>(number)),
        };

        Self::admit(value, origin)
    }

    /// Round a rational to the IEEE significand grid, including subnormal spacing.
    fn rounded<F>(number: &Number) -> F
    where
        F: Ieee,
    {
        let magnitude = number.magnitude();
        let numerator = magnitude.numer().magnitude();
        let denominator = magnitude.denom().magnitude();
        let precision = Ice::some(i32::try_from(F::PRECISION).ok());
        let minimum = F::MIN_EXP - precision + 1;
        let estimate = i128::from(numerator.bits()) - i128::from(denominator.bits());

        let value = match estimate {
            _ if numerator.bits() == 0 => F::ZERO,
            exponent if exponent > i128::from(F::MAX_EXP) + 1 => F::INFINITY,
            exponent if exponent < i128::from(minimum) - 1 => F::ZERO,
            exponent => {
                // NOTE: The preceding bounds restrict this exponent to the selected IEEE format.
                let mut exponent = Ice::some(i32::try_from(exponent).ok());
                let shift = Ice::some(usize::try_from(exponent.unsigned_abs()).ok());
                let is_below = if exponent < 0 {
                    (numerator << shift) < *denominator
                } else {
                    *numerator < (denominator << shift)
                };

                if is_below {
                    exponent -= 1;
                }

                let scale = (exponent - precision + 1).max(minimum);
                let shift = Ice::some(usize::try_from(scale.unsigned_abs()).ok());

                let (numerator, denominator) = if scale < 0 {
                    (numerator << shift, denominator.clone())
                } else {
                    (numerator.clone(), denominator << shift)
                };

                let (mut significand, remainder) = numerator.div_rem(&denominator);

                match ((remainder * 2_u8).cmp(&denominator), significand.bit(0)) {
                    (Ordering::Greater, _) | (Ordering::Equal, true) => significand += 1_u8,
                    (Ordering::Less, _) | (Ordering::Equal, false) => {},
                }

                // NOTE: Rounding can add a carry bit to the format's significand, but no more.
                let significand = Ice::some(significand.to_u128());

                F::from_u128_r(significand, Round::NearestTiesToEven)
                    .value
                    .scalbn_r(scale, Round::NearestTiesToEven)
            },
        };

        if magnitude.is_negative() { -value } else { value }
    }

    /// Reject infinities and NaNs before a value leaves its arithmetic owner.
    #[inline]
    fn admit(value: Value, origin: Origin) -> Result<Self, Failure> {
        let is_finite = match value {
            Value::Single(value) => value.is_finite(),
            Value::Double(value) => value.is_finite(),
        };

        if is_finite {
            Ok(Self(value))
        } else {
            Err(Failure::new(origin, Reason::Nonfinite))
        }
    }

    /// Retrieve the declared precision without converting the magnitude.
    #[inline]
    pub const fn format(self) -> Format {
        let Self(value) = self;

        match value {
            Value::Single(_) => Format::Single,
            Value::Double(_) => Format::Double,
        }
    }

    /// Retrieve the IEEE encoding in its declared width's low bits.
    #[inline]
    pub fn bits(self) -> u128 {
        let Self(value) = self;

        match value {
            Value::Single(value) => value.to_bits(),
            Value::Double(value) => value.to_bits(),
        }
    }

    /// Negation preserves precision and reverses the sign of zero.
    #[inline]
    pub fn negate(self) -> Self {
        let Self(value) = self;

        let value = match value {
            Value::Single(value) => Value::Single(-value),
            Value::Double(value) => Value::Double(-value),
        };

        Self(value)
    }

    /// Add values of the same precision with nearest, ties-to-even rounding.
    #[inline]
    pub fn add(self, other: Self, origin: Origin) -> Result<Self, Failure> {
        self.calculate(other, Operation::Add, origin)
    }

    /// Subtract without widening either operand.
    #[inline]
    pub fn subtract(self, other: Self, origin: Origin) -> Result<Self, Failure> {
        self.calculate(other, Operation::Subtract, origin)
    }

    /// Multiply at the declared precision, rejecting a nonfinite result.
    #[inline]
    pub fn multiply(self, other: Self, origin: Origin) -> Result<Self, Failure> {
        self.calculate(other, Operation::Multiply, origin)
    }

    /// Divide at the declared precision, rejecting either signed zero as divisor.
    #[inline]
    pub fn divide(self, other: Self, origin: Origin) -> Result<Self, Failure> {
        self.calculate(other, Operation::Divide, origin)
    }

    /// Dispatch only matching formats to software IEEE arithmetic.
    fn calculate(self, other: Self, operation: Operation, origin: Origin) -> Result<Self, Failure> {
        let Self(left) = self;

        let Self(right) = other;

        let value = match (left, right) {
            (Value::Single(left), Value::Single(right)) => operation.apply(left, right).map(Value::Single),
            (Value::Double(left), Value::Double(right)) => operation.apply(left, right).map(Value::Double),
            _ => Err(Reason::Precision),
        }
        .map_err(|reason| Failure::new(origin.clone(), reason))?;

        Self::admit(value, origin)
    }

    /// Compare only values sharing their declared precision.
    #[inline]
    pub fn compare(self, other: Self, origin: Origin) -> Result<Ordering, Failure> {
        let Self(left) = self;

        let Self(right) = other;

        match (left, right) {
            (Value::Single(left), Value::Single(right)) => Ok(Ice::some(left.partial_cmp(&right))),
            (Value::Double(left), Value::Double(right)) => Ok(Ice::some(left.partial_cmp(&right))),
            _ => Err(Failure::new(origin, Reason::Precision)),
        }
    }
}

/// Binary IEEE arithmetic performed after format matching.
#[derive(Debug, Clone, Copy)]
enum Operation {
    /// Rounded sum.
    Add,

    /// Rounded difference.
    Subtract,

    /// Rounded product.
    Multiply,

    /// Rounded quotient with a nonzero divisor.
    Divide,
}

impl Operation {
    /// Apply the operation within an already matched IEEE format.
    #[inline]
    fn apply<F>(self, left: F, right: F) -> Result<F, Reason>
    where
        F: Ieee,
    {
        let rounding = Round::NearestTiesToEven;

        match self {
            Self::Add => Ok(left.add_r(right, rounding).value),
            Self::Subtract => Ok(left.sub_r(right, rounding).value),
            Self::Multiply => Ok(left.mul_r(right, rounding).value),
            Self::Divide if right.is_zero() => Err(Reason::Zero),
            Self::Divide => Ok(left.div_r(right, rounding).value),
        }
    }
}

/// A floating-point requirement violated by a conversion or operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// Typed operands have different declared precisions.
    Precision,

    /// A divisor is positive or negative zero.
    Zero,

    /// Rounding or arithmetic would produce an infinity or NaN.
    Nonfinite,
}

impl Reason {
    /// Explain the violated IEEE requirement.
    #[inline]
    const fn message(self) -> &'static str {
        match self {
            Self::Precision => "floating-point operands must have the same declared precision",
            Self::Zero => "this floating-point divisor is zero",
            Self::Nonfinite => "this value cannot be represented as a finite value at the declared precision",
        }
    }
}

/// A floating-point expression failed its declared arithmetic requirements.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("invalid floating-point expression")]
#[report(title = "invalid floating-point expression", annotations = labels)]
// NOTE(invariant): The label uses the coordinates qualified by `origin`.
pub struct Failure {
    /// Source-qualified conversion or operation location.
    origin: Origin,

    /// Arithmetic requirement that failed.
    reason: Reason,

    /// Primary expression location.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

impl Failure {
    /// Associate an arithmetic failure with its originating expression.
    #[inline]
    const fn new(origin: Origin, reason: Reason) -> Self {
        let labels = InlineAnnotations::new(Label::new(reason.message(), origin.span()), []);

        Self { origin, reason, labels }
    }

    /// Retrieve the source identity and coordinates of the failed expression.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        let Self { origin, .. } = self;

        origin
    }

    /// Retrieve the arithmetic failure independently of rendered wording.
    #[inline]
    pub const fn reason(&self) -> Reason {
        let &Self { reason, .. } = self;

        reason
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;
    use core::cmp::Ordering;

    use aldebaran::prelude::Span;
    use num_bigint::BigInt;
    use rustc_apfloat::{
        Float as _, Round,
        ieee::{Double, Single},
    };

    use super::{Float, Format, Reason};
    use crate::{
        number::Number,
        source::{Id, Origin},
    };

    #[test]
    fn rational_rounding_is_direct_and_ties_choose_an_even_significand() {
        let origin = Origin::new(Id::new("float"), Span::unit(0));
        let midpoint = Number::ratio(
            (BigInt::from(1_u8) << 24_usize) + 1_u8,
            BigInt::from(1_u8) << 24_usize,
            origin.clone(),
        )
        .expect("midpoint");
        let epsilon = Number::ratio(1.into(), BigInt::from(1_u8) << 80_usize, origin.clone()).expect("epsilon");
        let above = midpoint.add(&epsilon);
        let direct = Float::new(Format::Single, &above, origin.clone()).expect("rounded single");
        let double = Float::new(Format::Double, &above, origin.clone()).expect("rounded double");

        assert_eq!(
            Float::new(Format::Single, &midpoint, origin.clone())
                .expect("tie")
                .bits(),
            u128::from(1_f32.to_bits())
        );
        assert_eq!(direct.bits(), u128::from(1_f32.to_bits() + 1));
        assert_eq!(double.bits(), (1_f64 + 2_f64.powi(-24)).to_bits().into());

        let odd_midpoint = Number::ratio(
            (BigInt::from(1_u8) << 24_usize) + 3_u8,
            BigInt::from(1_u8) << 24_usize,
            origin.clone(),
        )
        .expect("odd midpoint");

        assert_eq!(
            Float::new(Format::Single, &odd_midpoint, origin)
                .expect("tie rounds up")
                .bits(),
            u128::from(1_f32.to_bits() + 2)
        );
    }

    #[test]
    fn subnormal_rounding_and_signed_zero_follow_the_selected_grid() {
        let origin = Origin::new(Id::new("float"), Span::unit(0));

        for (format, shift) in [(Format::Single, 149_usize), (Format::Double, 1074)] {
            let smallest = Number::ratio(1.into(), BigInt::from(1_u8) << shift, origin.clone()).expect("smallest");
            let half = smallest.divide(&Number::integer(2), origin.clone()).expect("half");
            let zero = Float::new(format, &half, origin.clone()).expect("underflow");
            let negative = Float::new(format, &half.negate(), origin.clone()).expect("negative underflow");

            assert_eq!(
                Float::new(format, &smallest, origin.clone()).expect("subnormal").bits(),
                1
            );
            assert_eq!(zero.bits(), 0);
            assert_eq!(zero, negative);
            assert_ne!(zero.bits(), negative.bits());
            assert_eq!(negative.negate().bits(), 0);
            assert_eq!(
                Float::new(format, &half.multiply(&Number::integer(3)), origin.clone())
                    .expect("odd tie")
                    .bits(),
                2
            );
        }
    }

    #[test]
    fn arithmetic_keeps_precision_and_rejects_nonfinite_results_and_zero_divisors() {
        let origin = Origin::new(Id::new("float"), Span::unit(3));
        let single = Float::new(Format::Single, &Number::integer(1), origin.clone()).expect("single");
        let double = Float::new(Format::Double, &Number::integer(1), origin.clone()).expect("double");
        let zero = single.subtract(single, origin.clone()).expect("zero");
        let huge = Number::integer(BigInt::from(1_u8) << 2000_usize);
        let maximum = Number::integer(((BigInt::from(1_u8) << 24_usize) - 1_u8) << 104_usize);
        let largest = Float::new(Format::Single, &maximum, origin.clone()).expect("largest");

        assert_eq!(
            single.add(double, origin.clone()).expect_err("precision").reason(),
            Reason::Precision
        );
        assert_eq!(
            single.compare(double, origin.clone()).expect_err("precision").reason(),
            Reason::Precision
        );
        assert_eq!(
            single.compare(zero, origin.clone()).expect("ordering"),
            Ordering::Greater
        );
        assert_eq!(
            single
                .divide(zero.negate(), origin.clone())
                .expect_err("negative zero")
                .reason(),
            Reason::Zero
        );
        assert_eq!(
            largest.add(largest, origin.clone()).expect_err("overflow").reason(),
            Reason::Nonfinite
        );
        assert_eq!(largest.bits(), u128::from(f32::MAX.to_bits()));
        assert_eq!(
            Float::new(Format::Double, &huge, origin.clone())
                .expect_err("overflow")
                .reason(),
            Reason::Nonfinite
        );
        assert_eq!(single.multiply(single, origin.clone()).expect("product"), single);
        assert_eq!(single.divide(single, origin.clone()).expect("quotient"), single);
        assert_eq!(
            Float::new(Format::Single, &huge, origin.clone())
                .expect_err("source")
                .origin(),
            &origin
        );
    }

    #[test]
    fn exact_decimal_conversion_agrees_with_the_independent_ieee_decimal_reader() {
        let origin = Origin::new(Id::new("decimal"), Span::unit(0));

        for coefficient in [-17, -1, 1, 13, 0x0100_0001] {
            for exponent in [-400_i32, -50, -1, 0, 1, 50, 400] {
                let factor = BigInt::from(10_u8).pow(exponent.unsigned_abs());
                let magnitude = if exponent < 0 {
                    Number::ratio(coefficient.into(), factor, origin.clone()).expect("decimal fraction")
                } else {
                    Number::integer(BigInt::from(coefficient) * factor)
                };
                let text = format!("{coefficient}e{exponent}");
                let single = Single::from_str_r(&text, Round::NearestTiesToEven)
                    .expect("decimal IEEE reader")
                    .value;
                let double = Double::from_str_r(&text, Round::NearestTiesToEven)
                    .expect("decimal IEEE reader")
                    .value;

                for (format, is_finite, bits) in [
                    (Format::Single, single.is_finite(), single.to_bits()),
                    (Format::Double, double.is_finite(), double.to_bits()),
                ] {
                    let actual = Float::new(format, &magnitude, origin.clone());

                    assert_eq!(actual.is_ok(), is_finite, "{text}, {format:?}");

                    if is_finite {
                        let actual = actual.expect("finite conversion");
                        let exact = actual.exact();

                        assert_eq!(actual.bits(), bits, "{text}, {format:?}");

                        if exact != Number::integer(0) {
                            assert_eq!(
                                Float::new(format, &exact, origin.clone())
                                    .expect("exact roundtrip")
                                    .bits(),
                                bits,
                                "{text}, {format:?}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn rounding_carries_across_normal_and_finite_boundaries() {
        let origin = Origin::new(Id::new("boundary"), Span::unit(0));

        for (format, precision, minimum, maximum, normal_bits, largest_bits) in [
            (
                Format::Single,
                24_usize,
                -126_isize,
                127_isize,
                u128::from(f32::MIN_POSITIVE.to_bits()),
                u128::from(f32::MAX.to_bits()),
            ),
            (
                Format::Double,
                53,
                -1022,
                1023,
                u128::from(f64::MIN_POSITIVE.to_bits()),
                u128::from(f64::MAX.to_bits()),
            ),
        ] {
            let precision_scale = isize::try_from(precision).expect("IEEE precision");
            let normal = Number::binary(1, minimum);
            let half_subnormal = Number::binary(1, minimum - precision_scale);
            let normal_midpoint = normal.subtract(&half_subnormal);
            let overflow_midpoint = Number::binary(
                (BigInt::from(1_u8) << (precision + 1)) - 1_u8,
                maximum - precision_scale,
            );
            let below_overflow = overflow_midpoint.subtract(&Number::integer(1));

            assert_eq!(
                Float::new(format, &normal_midpoint, origin.clone())
                    .expect("tie to normal")
                    .bits(),
                normal_bits
            );
            assert_eq!(
                Float::new(format, &below_overflow, origin.clone())
                    .expect("below overflow")
                    .bits(),
                largest_bits
            );
            assert_eq!(
                Float::new(format, &overflow_midpoint, origin.clone())
                    .expect_err("tie to overflow")
                    .reason(),
                Reason::Nonfinite
            );
        }
    }
}

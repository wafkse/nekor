//! Exact mathematical magnitudes before declared-width conversion.

use core::num::NonZero;

use aldebaran::{
    prelude::Report,
    report::annotated::{InlineAnnotations, label::Label},
};
use fack::prelude::Error;
use num_bigint::{BigInt, BigUint};
use num_rational::BigRational;
use num_traits::{One as _, Signed as _, ToPrimitive as _, Zero as _};

use crate::source::Origin;

/// A normalized arbitrary-precision rational with a nonzero denominator.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
// NOTE(invariant): The denominator is positive and the fraction is reduced.
pub struct Number(BigRational);

impl Number {
    /// Scale an integral significand by an exact power of two without floating-point conversion.
    #[inline]
    pub fn binary(significand: impl Into<BigInt>, exponent: isize) -> Self {
        let significand = significand.into();
        let shift = exponent.unsigned_abs();
        let value = if exponent < 0 {
            let denominator = BigInt::from(1_u8) << shift;

            BigRational::new(significand, denominator)
        } else {
            BigRational::from_integer(significand << shift)
        };

        Self(value)
    }

    /// Represent an integer without narrowing it to a machine width.
    #[inline]
    pub fn integer(value: impl Into<BigInt>) -> Self {
        Self(BigRational::from_integer(value.into()))
    }

    /// Construct an exact fraction whose positive denominator is already proven nonzero.
    #[inline]
    pub fn fraction(numerator: impl Into<BigInt>, denominator: NonZero<u64>) -> Self {
        Self(BigRational::new(numerator.into(), BigInt::from(denominator.get())))
    }

    /// Construct a rational, rejecting a zero denominator at its source occurrence.
    #[inline]
    pub fn ratio(numerator: BigInt, denominator: BigInt, origin: Origin) -> Result<Self, ZeroDivisor> {
        if denominator.is_zero() {
            return Err(ZeroDivisor::new(origin));
        }

        Ok(Self(BigRational::new(numerator, denominator)))
    }

    /// Borrow the normalized exact magnitude.
    #[inline]
    pub const fn magnitude(&self) -> &BigRational {
        let Self(value) = self;

        value
    }

    /// Add exact mathematical magnitudes.
    #[inline]
    pub fn add(&self, other: &Self) -> Self {
        let Self(left) = self;

        let Self(right) = other;

        Self(left + right)
    }

    /// Subtract exact mathematical magnitudes.
    #[inline]
    pub fn subtract(&self, other: &Self) -> Self {
        let Self(left) = self;

        let Self(right) = other;

        Self(left - right)
    }

    /// Multiply without rounding or a fixed intermediate width.
    #[inline]
    pub fn multiply(&self, other: &Self) -> Self {
        let Self(left) = self;

        let Self(right) = other;

        Self(left * right)
    }

    /// Divide exactly and locate a failure at the divisor.
    #[inline]
    pub fn divide(&self, other: &Self, origin: Origin) -> Result<Self, ZeroDivisor> {
        let Self(left) = self;

        let Self(right) = other;

        if right.is_zero() {
            return Err(ZeroDivisor::new(origin));
        }

        Ok(Self(left / right))
    }

    /// Negate before any signed-width representability check.
    #[inline]
    pub fn negate(&self) -> Self {
        let Self(value) = self;

        Self(-value)
    }

    /// Raise a magnitude to an arbitrary integral exponent without imposing a cap.
    #[inline]
    pub fn power(&self, exponent: &BigInt, origin: Origin) -> Result<Self, ZeroDivisor> {
        let Self(value) = self;

        if value.is_zero() && exponent.is_negative() {
            return Err(ZeroDivisor::new(origin));
        }

        let is_reciprocal = exponent.is_negative();
        let mut exponent: BigUint = exponent.magnitude().clone();
        let mut base = if exponent.is_zero() {
            BigRational::one()
        } else {
            value.clone()
        };
        let mut result = BigRational::one();

        while !exponent.is_zero() {
            if exponent.bit(0) {
                result *= &base;
            }

            exponent >>= 1_usize;

            if !exponent.is_zero() {
                base = &base * &base;
            }
        }

        Ok(Self(if is_reciprocal { result.recip() } else { result }))
    }

    /// Convert an integral magnitude when it fits the entire signed 128-bit range.
    #[inline]
    pub fn signed(&self) -> Option<i128> {
        let Self(value) = self;

        value.is_integer().then(|| value.to_integer().to_i128()).flatten()
    }

    /// Convert an integral magnitude when it fits the unsigned 128-bit range.
    #[inline]
    pub fn unsigned(&self) -> Option<u128> {
        let Self(value) = self;

        value.is_integer().then(|| value.to_integer().to_u128()).flatten()
    }

    /// Recover a nonnegative integral magnitude without imposing a machine-width bound.
    #[inline]
    pub fn natural(&self) -> Option<BigUint> {
        let Self(value) = self;

        value.is_integer().then(|| value.to_integer().to_biguint()).flatten()
    }
}

/// An exact operation attempted division by zero.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("division by zero")]
#[report(title = "division by zero", annotations = labels)]
// NOTE(invariant): The annotation uses the byte coordinates of `origin`.
pub struct ZeroDivisor {
    /// Source qualifying the reported byte coordinates.
    origin: Origin,

    /// Location of the zero divisor or invalid reciprocal power.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

impl ZeroDivisor {
    /// Identify the operand whose zero value prevents the operation.
    #[inline]
    pub const fn new(origin: Origin) -> Self {
        let labels = InlineAnnotations::new(Label::new("this divisor evaluates to zero", origin.span()), []);

        Self { origin, labels }
    }

    /// Retrieve the canonical source associated with the report.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        let Self { origin, .. } = self;

        origin
    }
}

#[cfg(test)]
mod tests {
    use aldebaran::prelude::{Report as _, Span};
    use num_bigint::BigInt;

    use super::Number;
    use crate::source::{Id, Origin};

    #[test]
    fn exact_arithmetic_preserves_fractional_results_and_signed_minima() {
        let origin = Origin::new(Id::new("test"), Span::unit(0));
        let third = Number::ratio(1.into(), 3.into(), origin).expect("nonzero denominator");

        assert_eq!(third.add(&third).add(&third), Number::integer(1));
        assert_eq!(third.multiply(&Number::integer(3)), Number::integer(1));
        assert_eq!(third.signed(), None);

        let magnitude = Number::integer(BigInt::from(1_u8) << 127_usize);

        assert_eq!(magnitude.signed(), None);
        assert_eq!(magnitude.negate().signed(), Some(i128::MIN));
        assert_eq!(magnitude.negate().unsigned(), None);
    }

    #[test]
    fn powers_support_reciprocals_and_exponents_larger_than_machine_words() {
        let origin = Origin::new(Id::new("test"), Span::unit(0));
        let reciprocal = Number::integer(2)
            .power(&BigInt::from(-3), origin.clone())
            .expect("reciprocal");
        let expected = Number::ratio(1.into(), 8.into(), origin.clone()).expect("nonzero denominator");
        let exponent = BigInt::from(1_u8) << 256_usize;

        assert_eq!(reciprocal, expected);
        assert_eq!(
            Number::integer(1).power(&exponent, origin).expect("arbitrary exponent"),
            Number::integer(1)
        );
    }

    #[test]
    fn zero_divisors_preserve_the_canonical_source_and_report() {
        let origin = Origin::new(Id::new("host:divisor"), Span::unit(7));
        let error = Number::integer(1)
            .divide(&Number::integer(0), origin.clone())
            .expect_err("zero divisor");

        assert_eq!(error.origin(), &origin);
        assert_eq!(error.title(), "division by zero");
        assert!(Number::integer(0).power(&BigInt::from(-1), origin.clone()).is_err());
        assert!(Number::ratio(1.into(), 0.into(), origin).is_err());
    }
}

//! Integral byte extents obtained without truncating fractional information units.

use aldebaran::{
    prelude::Report,
    report::annotated::{InlineAnnotations, label::Label},
};
use fack::prelude::Error;
use num_bigint::{BigInt, BigUint};

use crate::{
    dimension::{Axis, Dimension},
    number::Number,
    quantity::{Kind, Quantity},
    source::Origin,
};

/// A nonnegative integral number of bytes, independent of address width.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
// NOTE(invariant): None.
pub struct Bytes(BigUint);

impl Bytes {
    /// Interpret an unsigned mathematical magnitude explicitly as a byte count.
    #[inline]
    pub const fn new(value: BigUint) -> Self {
        Self(value)
    }

    /// Check information dimension, semantic kind, and exact byte integrality.
    #[inline]
    pub fn from_quantity(value: &Quantity, origin: Origin) -> Result<Self, Admission> {
        let information = Dimension::axis(Axis::Information);

        match (value.kind(), value.dimension()) {
            (Kind::Linear, dimension) if *dimension == information => {},
            _ => return Err(Admission::new(origin, Reason::Dimension)),
        }

        let bits = value.magnitude().magnitude();
        let bytes = bits / BigInt::from(8_u8);

        match (bytes.is_integer(), bytes.to_integer().to_biguint()) {
            (true, Some(value)) => Ok(Self(value)),
            (true, None) => Err(Admission::new(origin, Reason::Negative)),
            (false, _) => Err(Admission::new(origin, Reason::Fractional)),
        }
    }

    /// Borrow the exact byte count, including extents larger than an address.
    #[inline]
    pub const fn magnitude(&self) -> &BigUint {
        let Self(value) = self;

        value
    }

    /// Express this extent as an exact canonical information quantity.
    #[inline]
    pub fn quantity(&self) -> Quantity {
        let Self(value) = self;

        let magnitude = Number::integer(value * 8_u8);
        let dimension = Dimension::axis(Axis::Information);

        Quantity::linear(magnitude, dimension)
    }
}

/// Why an expression is not a nonnegative integral byte extent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// The expression is not a linear information quantity.
    Dimension,

    /// The magnitude is not an integral number of bytes.
    Fractional,

    /// The byte magnitude is negative.
    Negative,
}

impl Reason {
    /// Describe the exact byte-extent requirement that failed.
    #[inline]
    const fn message(self) -> &'static str {
        match self {
            Self::Dimension => "an address extent must have information dimension",
            Self::Fractional => "an address extent must be an integral number of bytes",
            Self::Negative => "an address extent cannot be negative",
        }
    }
}

/// A dimensional quantity cannot be used as an address extent.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("invalid address extent")]
#[report(title = "invalid address extent", annotations = labels)]
// NOTE(invariant): The label uses the coordinates qualified by `origin`.
pub struct Admission {
    /// Canonical source location of the rejected extent.
    origin: Origin,

    /// The byte-extent requirement that failed.
    reason: Reason,

    /// Primary extent location.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

impl Admission {
    /// Report the failed byte-extent requirement.
    #[inline]
    const fn new(origin: Origin, reason: Reason) -> Self {
        let labels = InlineAnnotations::new(Label::new(reason.message(), origin.span()), []);

        Self { origin, reason, labels }
    }

    /// Retrieve the source-qualified location of the rejected extent.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        let Self { origin, .. } = self;

        origin
    }

    /// Retrieve the failed byte-extent requirement.
    #[inline]
    pub const fn reason(&self) -> Reason {
        let &Self { reason, .. } = self;

        reason
    }
}

#[cfg(test)]
mod tests {
    use aldebaran::prelude::Span;

    use super::{Bytes, Reason};
    use crate::{
        number::Number,
        source::{Id, Origin},
        unit::Unit,
    };

    #[test]
    fn byte_conversion_rejects_fractional_bytes_negative_extents_and_wrong_dimensions() {
        let origin = Origin::new(Id::new("extent"), Span::unit(0));

        for (unit, magnitude, reason) in [
            ("bit", 1, Reason::Fractional),
            ("B", -1, Reason::Negative),
            ("s", 1, Reason::Dimension),
            ("dB", 1, Reason::Dimension),
        ] {
            let quantity = Unit::lookup(unit)
                .expect("intrinsic unit")
                .apply(&Number::integer(magnitude), None, origin.clone())
                .expect("quantity");

            assert_eq!(
                Bytes::from_quantity(&quantity, origin.clone())
                    .expect_err("invalid extent")
                    .reason(),
                reason
            );
        }

        let quantity = Unit::lookup("KiB")
            .expect("intrinsic unit")
            .apply(&Number::integer(1), None, origin.clone())
            .expect("quantity");
        let bytes = Bytes::from_quantity(&quantity, origin).expect("integral bytes");

        assert_eq!(bytes.magnitude(), &1024_u16.into());
        assert_eq!(bytes.quantity(), quantity);
    }
}

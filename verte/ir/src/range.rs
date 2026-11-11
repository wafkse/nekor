//! Ordered intervals preserve their endpoint mode and nominal coordinate type.
//!
//! Address intervals reuse the byte-coordinate representation, whose mathematical
//! exclusive boundary may be one past its largest address. Other scalar intervals
//! store both typed endpoints and check measure representability during projection.

use alloc::sync::Arc;
use core::cmp::Ordering;

use aldebaran::{
    prelude::Report,
    report::annotated::{InlineAnnotations, label::Label},
};
use fack::prelude::Error;

use crate::{
    address::{
        Admission as AddressAdmission,
        range::{self as address, Mode},
    },
    number::Number,
    scalar::{self, Scalar},
    source::Origin,
    types::Endpoint,
};

/// A typed interval, including address spaces with a mathematical one-past boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Range {
    /// Byte coordinates whose exclusive boundary need not be an address.
    Address(address::Range),

    /// An ordered pair of compatible, representable scalar endpoints.
    Scalar(Interval),
}

impl Range {
    /// Construct an interval from compatible typed endpoints without converting domains.
    #[inline]
    pub fn between(start: Scalar, end: Scalar, mode: Mode, origin: Origin) -> Result<Self, Construction> {
        match (start, end) {
            (Scalar::Address(start), Scalar::Address(end)) => address::Range::between(start, &end, mode, origin)
                .map(Self::Address)
                .map_err(Construction::Address),
            (start, end) => match start.compare(&end, origin.clone()).map_err(Construction::Types)? {
                Ordering::Greater => Err(Construction::Reversed(Reversed::new(origin))),
                Ordering::Less | Ordering::Equal => Ok(Self::Scalar(Interval { start, end, mode })),
            },
        }
    }

    /// Retrieve the ordered endpoint type independently of its magnitudes.
    #[inline]
    pub fn endpoint(&self) -> Endpoint {
        match self {
            Self::Address(range) => Endpoint::Address(Arc::clone(range.start().domain())),
            Self::Scalar(interval) => interval.start().endpoint(),
        }
    }

    /// Retrieve the typed starting coordinate.
    #[inline]
    pub fn start(&self) -> Scalar {
        match self {
            Self::Address(range) => Scalar::Address(range.start().clone()),
            Self::Scalar(interval) => interval.start().clone(),
        }
    }

    /// Project the stored endpoint, rejecting an unrepresentable one-past address.
    #[inline]
    pub fn end(&self, origin: Origin) -> Result<Scalar, AddressAdmission> {
        match self {
            Self::Address(range) => range.end(origin).map(Scalar::Address),
            Self::Scalar(interval) => Ok(interval.end().clone()),
        }
    }

    /// Retrieve whether the stored endpoint is included in the interval.
    #[inline]
    pub const fn mode(&self) -> Mode {
        match self {
            Self::Address(range) => range.mode(),
            Self::Scalar(interval) => interval.mode(),
        }
    }

    /// Compute the exact coordinate measure without narrowing to an endpoint type.
    ///
    /// Discrete scalar intervals count their included coordinates. Continuous
    /// dimensional and IEEE intervals use the difference of their bounds.
    #[inline]
    pub fn measure(&self) -> Number {
        match self {
            Self::Address(range) => Number::integer(range.size().magnitude().clone()),
            Self::Scalar(interval) => {
                let difference = interval.end().exact().subtract(&interval.start().exact());

                match (interval.start(), interval.mode()) {
                    (Scalar::Integer(_) | Scalar::Index(_) | Scalar::Count(_), Mode::Inclusive) => {
                        difference.add(&Number::integer(1))
                    },
                    _ => difference,
                }
            },
        }
    }
}

/// Representable scalar bounds admitted in their declared type's ordering.
#[derive(Debug, Clone, PartialEq, Eq)]
// NOTE(invariant): Both endpoints share a non-address type and `start` does not exceed `end`.
pub struct Interval {
    /// Typed starting coordinate.
    start: Scalar,

    /// Typed stored endpoint.
    end: Scalar,

    /// Whether the endpoint itself belongs to the interval.
    mode: Mode,
}

impl Interval {
    /// Borrow the typed starting coordinate.
    #[inline]
    pub const fn start(&self) -> &Scalar {
        let Self { start, .. } = self;

        start
    }

    /// Borrow the typed stored endpoint.
    #[inline]
    pub const fn end(&self) -> &Scalar {
        let Self { end, .. } = self;

        end
    }

    /// Retrieve whether the stored endpoint is included.
    #[inline]
    pub const fn mode(&self) -> Mode {
        let &Self { mode, .. } = self;

        mode
    }
}

/// An interval's endpoints violate ordering, compatibility, or address requirements.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[report(forward)]
pub enum Construction {
    /// Address width, domain, or ordering requirements failed.
    #[error(transparent(0))]
    Address(address::Admission),

    /// Scalar endpoints have incompatible declared types.
    #[error(transparent(0))]
    Types(scalar::Incompatible),

    /// A scalar endpoint precedes its start.
    #[error(transparent(0))]
    Reversed(Reversed),
}

/// An ordered scalar interval ends before its starting coordinate.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("range endpoint precedes its start")]
#[report(title = "reversed range", annotations = labels)]
// NOTE(invariant): The label uses the coordinates qualified by `origin`.
pub struct Reversed {
    /// Source-qualified interval expression.
    origin: Origin,

    /// Primary interval location.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

impl Reversed {
    /// Locate reversed scalar bounds at the interval expression.
    #[inline]
    const fn new(origin: Origin) -> Self {
        let labels = InlineAnnotations::new(
            Label::new("the endpoint must not precede the starting coordinate", origin.span()),
            [],
        );

        Self { origin, labels }
    }

    /// Retrieve the canonical source and original coordinates of the interval.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        let Self { origin, .. } = self;

        origin
    }
}

#[cfg(test)]
mod tests {
    use alloc::sync::Arc;

    use aldebaran::prelude::Span;
    use num_bigint::BigInt;

    use super::Range;
    use crate::{
        address::{
            Address,
            bytes::Bytes,
            range::{Mode, Range as AddressRange},
        },
        domain,
        integer::{Format, Integer, Width},
        number::Number,
        scalar::Scalar,
        source::{Id, Origin},
    };

    #[test]
    fn full_width_integer_intervals_defer_measure_representability() {
        let origin = Origin::new(Id::new("range"), Span::unit(0));
        let format = Format::Signed(Width::W128);
        let boundary = BigInt::from(1_u8) << 127_usize;
        let start =
            Scalar::Integer(Integer::new(format, Number::integer(-&boundary), origin.clone()).expect("minimum"));
        let end =
            Scalar::Integer(Integer::new(format, Number::integer(&boundary - 1_u8), origin.clone()).expect("maximum"));
        let range =
            Range::between(start.clone(), end.clone(), Mode::Inclusive, origin.clone()).expect("whole integer domain");

        assert_eq!(range.measure(), Number::integer(BigInt::from(1_u8) << 128_usize));
        assert!(!format.accepts(&range.measure()));
        assert_eq!(range.end(origin.clone()).expect("endpoint"), end);
        assert!(Range::between(end, start.clone(), Mode::Exclusive, origin.clone()).is_err());
        assert_eq!(
            Range::between(start.clone(), start, Mode::Exclusive, origin)
                .expect("empty")
                .measure(),
            Number::integer(0)
        );
    }

    #[test]
    fn address_ranges_keep_their_mathematical_boundary_through_the_shared_value_model() {
        let origin = Origin::new(Id::new("range"), Span::unit(0));
        let domain = Arc::new(domain::Address::new(
            origin.clone(),
            domain::Width::new(8).expect("width"),
        ));
        let start = Address::new(Arc::clone(&domain), Number::integer(240), origin.clone()).expect("coordinate");
        let extent = Bytes::new(16_u8.into());
        let address = AddressRange::extent(start.clone(), &extent, origin.clone()).expect("full tail");
        let range = Range::Address(address);

        assert_eq!(range.start(), Scalar::Address(start));
        assert_eq!(range.measure(), Number::integer(16));
        assert_eq!(range.mode(), Mode::Exclusive);
        assert!(range.end(origin.clone()).is_err());

        let zero = Scalar::Address(Address::new(domain, Number::integer(0), origin.clone()).expect("zero"));
        let integer = Scalar::Integer(
            Integer::new(Format::Unsigned(Width::W8), Number::integer(0), origin.clone()).expect("integer"),
        );

        assert!(Range::between(zero, integer, Mode::Exclusive, origin).is_err());
    }
}

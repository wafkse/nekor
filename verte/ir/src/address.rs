//! Width-checked nominal coordinates, distinct from mathematical range boundaries.
//!
//! These representations enforce coordinate widths. Source-declared validity
//! predicates are additional contracts checked by the evaluation layer.

use alloc::{boxed::Box, sync::Arc};
use core::cmp::Ordering;

use aldebaran::{
    prelude::Report,
    report::annotated::{InlineAnnotations, label::Label},
};
use fack::prelude::Error;
use num_bigint::BigUint;

use crate::{domain, number::Number, source::Origin};

pub mod bytes;
pub mod range;

/// A nonnegative integral coordinate representable in its nominal address domain.
#[derive(Debug, Clone, PartialEq, Eq)]
// NOTE(invariant): `coordinate` is strictly less than 2^domain.width().
pub struct Address {
    /// Nominal declaration and source-defined coordinate width.
    domain: Arc<domain::Address>,

    /// Exact byte coordinate.
    coordinate: BigUint,
}

impl Address {
    /// Check an exact magnitude before admitting it as an address coordinate.
    #[inline]
    pub fn new(domain: Arc<domain::Address>, value: Number, origin: Origin) -> Result<Self, Admission> {
        let magnitude = value.magnitude();
        let coordinate = match (magnitude.is_integer(), magnitude.to_integer().to_biguint()) {
            (true, Some(coordinate)) if coordinate < domain.boundary() => coordinate,
            (true, Some(_)) => return Err(Admission::new(origin, domain, value, Reason::Width)),
            (true, None) => return Err(Admission::new(origin, domain, value, Reason::Negative)),
            (false, _) => return Err(Admission::new(origin, domain, value, Reason::Fractional)),
        };

        Ok(Self { domain, coordinate })
    }

    /// Borrow the nominal domain, including the declaration origin.
    #[inline]
    pub const fn domain(&self) -> &Arc<domain::Address> {
        let Self { domain, .. } = self;

        domain
    }

    /// Borrow the exact byte coordinate without narrowing it to a host pointer.
    #[inline]
    pub const fn coordinate(&self) -> &BigUint {
        let Self { coordinate, .. } = self;

        coordinate
    }

    /// Add a nonnegative byte extent, checking representability of the resulting address.
    #[inline]
    pub fn add(&self, extent: &bytes::Bytes, origin: Origin) -> Result<Self, Admission> {
        let Self { domain, coordinate } = self;

        let value = Number::integer(coordinate + extent.magnitude());

        Self::new(Arc::clone(domain), value, origin)
    }

    /// Subtract a byte extent, rejecting coordinates below zero.
    #[inline]
    pub fn subtract(&self, extent: &bytes::Bytes, origin: Origin) -> Result<Self, Admission> {
        let Self { domain, coordinate } = self;

        let value = Number::integer(coordinate.clone()).subtract(&Number::integer(extent.magnitude().clone()));

        Self::new(Arc::clone(domain), value, origin)
    }
}

/// Why a mathematical magnitude cannot be an address coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// A coordinate must be an integral number of bytes.
    Fractional,

    /// Coordinates cannot be negative.
    Negative,

    /// The coordinate is outside the source-declared width.
    Width,
}

impl Reason {
    /// Describe the violated address requirement.
    #[inline]
    const fn message(self) -> &'static str {
        match self {
            Self::Fractional => "an address coordinate must be an integral number of bytes",
            Self::Negative => "an address coordinate cannot be negative",
            Self::Width => "this coordinate is outside the domain's declared width",
        }
    }
}

/// An exact magnitude failed width-based address admission.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("invalid address coordinate")]
#[report(title = "invalid address coordinate", annotations = labels)]
// NOTE(invariant): The report label uses coordinates qualified by `origin`.
pub struct Admission {
    /// Canonical location of the rejected coordinate expression.
    origin: Origin,

    /// Domain whose coordinate requirements apply.
    domain: Arc<domain::Address>,

    /// Exact rejected magnitude, before any narrowing.
    value: Box<Number>,

    /// Violated representability requirement.
    reason: Reason,

    /// Primary coordinate location in its original source.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

impl Admission {
    /// Report the exact rejected value and its nominal domain.
    #[inline]
    fn new(origin: Origin, domain: Arc<domain::Address>, value: Number, reason: Reason) -> Self {
        let labels = InlineAnnotations::new(Label::new(reason.message(), origin.span()), []);
        let value = Box::new(value);

        Self {
            origin,
            domain,
            value,
            reason,
            labels,
        }
    }

    /// Retrieve the source identity and coordinates of the rejected value.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        let Self { origin, .. } = self;

        origin
    }

    /// Borrow the domain, including its independently qualified declaration location.
    #[inline]
    pub const fn domain(&self) -> &Arc<domain::Address> {
        let Self { domain, .. } = self;

        domain
    }

    /// Borrow the rejected exact magnitude.
    #[inline]
    pub const fn value(&self) -> &Number {
        let Self { value, .. } = self;

        value
    }

    /// Retrieve the violated coordinate requirement.
    #[inline]
    pub const fn reason(&self) -> Reason {
        let &Self { reason, .. } = self;

        reason
    }
}

impl Address {
    /// Measure a nonnegative byte distance within the same nominal address domain.
    #[inline]
    pub fn difference(&self, other: &Self, origin: Origin) -> Result<bytes::Bytes, Difference> {
        let Self {
            domain: left_domain,
            coordinate: left,
        } = self;

        let Self {
            domain: right_domain,
            coordinate: right,
        } = other;

        match (left_domain == right_domain, left.cmp(right)) {
            (true, Ordering::Equal | Ordering::Greater) => Ok(bytes::Bytes::new(left - right)),
            _ => {
                let labels = InlineAnnotations::new(
                    Label::new(
                        "address subtraction requires the same domain and a nonnegative byte difference",
                        origin.span(),
                    ),
                    [],
                );
                let left = Arc::clone(left_domain);
                let right = Arc::clone(right_domain);

                Err(Difference {
                    origin,
                    left,
                    right,
                    labels,
                })
            },
        }
    }
}

/// Address subtraction has mismatched domains or a negative mathematical difference.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("invalid address difference")]
#[report(title = "invalid address difference", annotations = labels)]
// NOTE(invariant): The label uses coordinates qualified by `origin`.
pub struct Difference {
    /// Canonical source occurrence of the subtraction.
    origin: Origin,

    /// Domain of the left operand.
    left: Arc<domain::Address>,

    /// Domain of the right operand.
    right: Arc<domain::Address>,

    /// Primary operation location.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

impl Difference {
    /// Retrieve the canonical source of the failed subtraction.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        let Self { origin, .. } = self;

        origin
    }

    /// Borrow both domain declarations for independent source-qualified diagnostics.
    #[inline]
    pub const fn domains(&self) -> (&Arc<domain::Address>, &Arc<domain::Address>) {
        let Self { left, right, .. } = self;

        (left, right)
    }
}

#[cfg(test)]
mod tests {
    use alloc::sync::Arc;
    use core::num::NonZero;

    use aldebaran::prelude::Span;

    use super::{Address, Reason, bytes::Bytes};
    use crate::{
        domain::{self, Width},
        number::Number,
        source::{Id, Origin},
    };

    #[test]
    fn coordinate_admission_checks_mathematical_values_before_narrowing() {
        let origin = Origin::new(Id::new("coordinate"), Span::unit(3));
        let declaration = Origin::new(Id::new("domain"), Span::unit(7));
        let domain = Arc::new(domain::Address::new(declaration, Width::new(8).expect("width")));
        let fraction = Number::fraction(1, NonZero::new(2).expect("nonzero"));

        for (value, reason) in [
            (Number::integer(-1), Reason::Negative),
            (Number::integer(256), Reason::Width),
            (fraction, Reason::Fractional),
        ] {
            let failure = Address::new(Arc::clone(&domain), value, origin.clone()).expect_err("invalid coordinate");

            assert_eq!(failure.origin(), &origin);
            assert_eq!(failure.reason(), reason);
            assert_eq!(failure.domain(), &domain);
        }

        assert_eq!(
            Address::new(domain, Number::integer(255), origin)
                .expect("maximum coordinate")
                .coordinate(),
            &255_u16.into()
        );
    }

    #[test]
    fn address_arithmetic_checks_domains_and_negative_results() {
        let origin = Origin::new(Id::new("coordinate"), Span::unit(0));
        let first = Arc::new(domain::Address::new(origin.clone(), Width::new(8).expect("width")));
        let second = Arc::new(domain::Address::new(
            Origin::new(Id::new("other"), Span::unit(0)),
            Width::new(8).expect("width"),
        ));
        let low = Address::new(Arc::clone(&first), Number::integer(10), origin.clone()).expect("address");
        let high = Address::new(first, Number::integer(20), origin.clone()).expect("address");
        let foreign = Address::new(second, Number::integer(10), origin.clone()).expect("address");
        let distance = high.difference(&low, origin.clone()).expect("same domain difference");

        assert_eq!(distance.magnitude(), &10_u8.into());
        assert_eq!(low.add(&distance, origin.clone()).expect("addition"), high);
        assert_eq!(high.subtract(&distance, origin.clone()).expect("subtraction"), low);
        assert!(low.difference(&high, origin.clone()).is_err());
        assert!(high.difference(&foreign, origin.clone()).is_err());
        assert!(low.subtract(&Bytes::new(11_u8.into()), origin).is_err());
    }
}

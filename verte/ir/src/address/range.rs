//! Address intervals with mathematical endpoints beyond representable coordinates.

use alloc::sync::Arc;
use core::cmp::Ordering;

use aldebaran::{
    prelude::Report,
    report::annotated::{InlineAnnotations, label::Label},
};
use fack::prelude::Error;
use num_bigint::BigUint;

use super::{Address, Admission as CoordinateAdmission, bytes::Bytes};
use crate::{domain, number::Number, source::Origin};

/// Whether the stored endpoint is included in the interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// A half-open interval, including the empty interval.
    Exclusive,

    /// A closed interval, including both endpoints.
    Inclusive,
}

/// An ordered interval entirely representable inside a nominal address space.
///
/// The stored endpoint may equal the mathematical one-past domain boundary for
/// an exclusive interval. Predicate-based domain validity remains an evaluation
/// contract over every included coordinate.
#[derive(Debug, Clone, PartialEq, Eq)]
// NOTE(invariant): The endpoint is at least the start and is inside the domain, or one-past when
// exclusive.
pub struct Range {
    /// Width-checked starting coordinate and nominal domain.
    start: Address,

    /// Mathematical endpoint without truncation to the address width.
    endpoint: BigUint,

    /// Interpretation of the stored endpoint.
    mode: Mode,
}

impl Range {
    /// Check ordering and width while preserving an exclusive mathematical endpoint.
    #[inline]
    pub fn new(start: Address, endpoint: BigUint, mode: Mode, origin: Origin) -> Result<Self, Admission> {
        let limit = start.domain().boundary();

        match (endpoint.cmp(start.coordinate()), mode, endpoint.cmp(&limit)) {
            (Ordering::Less, _, _) => Err(Admission::new(origin, Arc::clone(start.domain()), Reason::Reversed)),
            (_, Mode::Exclusive, Ordering::Less | Ordering::Equal) | (_, Mode::Inclusive, Ordering::Less) => {
                Ok(Self { start, endpoint, mode })
            },
            _ => Err(Admission::new(origin, Arc::clone(start.domain()), Reason::Width)),
        }
    }

    /// Construct an interval whose explicit endpoint belongs to the same nominal space.
    #[inline]
    pub fn between(start: Address, end: &Address, mode: Mode, origin: Origin) -> Result<Self, Admission> {
        if start.domain() != end.domain() {
            return Err(Admission::new(origin, Arc::clone(start.domain()), Reason::Domain));
        }

        Self::new(start, end.coordinate().clone(), mode, origin)
    }

    /// Construct a half-open interval from an exact integral byte extent.
    #[inline]
    pub fn extent(start: Address, extent: &Bytes, origin: Origin) -> Result<Self, Admission> {
        let endpoint = start.coordinate() + extent.magnitude();

        Self::new(start, endpoint, Mode::Exclusive, origin)
    }

    /// Borrow the starting address and its nominal identity.
    #[inline]
    pub const fn start(&self) -> &Address {
        let Self { start, .. } = self;

        start
    }

    /// Borrow the exact stored endpoint, which need not itself be an address.
    #[inline]
    pub const fn endpoint(&self) -> &BigUint {
        let Self { endpoint, .. } = self;

        endpoint
    }

    /// Retrieve whether the endpoint is inclusive or exclusive.
    #[inline]
    pub const fn mode(&self) -> Mode {
        let &Self { mode, .. } = self;

        mode
    }

    /// Project the endpoint as an address only when the coordinate is representable.
    #[inline]
    pub fn end(&self, origin: Origin) -> Result<Address, CoordinateAdmission> {
        let Self { start, endpoint, .. } = self;

        let value = Number::integer(endpoint.clone());

        Address::new(Arc::clone(start.domain()), value, origin)
    }

    /// Compute the exact byte measure, including a full-width address-space measure.
    #[inline]
    pub fn size(&self) -> Bytes {
        let Self { start, endpoint, mode } = self;

        let difference = endpoint - start.coordinate();
        let size = match mode {
            Mode::Exclusive => difference,
            Mode::Inclusive => difference + 1_u8,
        };

        Bytes::new(size)
    }

    /// Enumerate every included byte coordinate without imposing a validation cutoff.
    #[inline]
    pub fn coordinates(&self) -> Coordinates {
        let Self { start, endpoint, mode } = self;

        let next = start.coordinate().clone();
        let end = match mode {
            Mode::Exclusive => endpoint.clone(),
            Mode::Inclusive => endpoint + 1_u8,
        };

        Coordinates { next, end }
    }
}

/// Exact coordinate traversal for whole-range domain validation.
#[derive(Debug, Clone)]
// NOTE(invariant): `next` never exceeds the exclusive `end` boundary.
pub struct Coordinates {
    /// Next coordinate to be examined.
    next: BigUint,

    /// Mathematical exclusive traversal boundary.
    end: BigUint,
}

impl Iterator for Coordinates {
    type Item = BigUint;

    #[inline]
    fn next(&mut self) -> Option<BigUint> {
        let Self { next, end } = self;

        if *next == *end {
            return None;
        }

        let value = next.clone();

        *next += 1_u8;

        Some(value)
    }
}

/// The range requirement that failed admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// The mathematical endpoint precedes the start.
    Reversed,

    /// Included coordinates would exceed the source-declared address width.
    Width,

    /// Explicit endpoints belong to different nominal domains.
    Domain,
}

impl Reason {
    /// Describe the violated range requirement.
    #[inline]
    const fn message(self) -> &'static str {
        match self {
            Self::Reversed => "the range endpoint precedes its starting address",
            Self::Width => "this interval includes coordinates outside the declared address width",
            Self::Domain => "range endpoints must belong to the same nominal address domain",
        }
    }
}

/// An interval cannot be admitted within its starting address's nominal space.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("invalid address range")]
#[report(title = "invalid address range", annotations = labels)]
// NOTE(invariant): The label uses coordinates qualified by `origin`.
pub struct Admission {
    /// Canonical source location of the range expression.
    origin: Origin,

    /// Address-space declaration whose requirements apply.
    domain: Arc<domain::Address>,

    /// Violated interval requirement.
    reason: Reason,

    /// Primary range location.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

impl Admission {
    /// Report the violated interval requirement and the applicable domain.
    #[inline]
    const fn new(origin: Origin, domain: Arc<domain::Address>, reason: Reason) -> Self {
        let labels = InlineAnnotations::new(Label::new(reason.message(), origin.span()), []);

        Self {
            origin,
            domain,
            reason,
            labels,
        }
    }

    /// Retrieve the canonical source location of the range.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        let Self { origin, .. } = self;

        origin
    }

    /// Borrow the independently source-qualified domain declaration.
    #[inline]
    pub const fn domain(&self) -> &Arc<domain::Address> {
        let Self { domain, .. } = self;

        domain
    }

    /// Retrieve the violated interval requirement.
    #[inline]
    pub const fn reason(&self) -> Reason {
        let &Self { reason, .. } = self;

        reason
    }
}

#[cfg(test)]
mod tests {
    use alloc::sync::Arc;

    use aldebaran::prelude::Span;
    use num_bigint::BigUint;

    use super::{Mode, Range, Reason};
    use crate::{
        address::{Address, bytes::Bytes},
        domain::{self, Width},
        number::Number,
        source::{Id, Origin},
    };

    #[test]
    fn a_full_tail_range_preserves_an_unrepresentable_exclusive_endpoint() {
        let origin = Origin::new(Id::new("range"), Span::unit(0));
        let domain = Arc::new(domain::Address::new(origin.clone(), Width::new(8).expect("width")));
        let start = Address::new(domain, Number::integer(0xF0), origin.clone()).expect("coordinate");
        let extent = Bytes::new(16_u8.into());
        let range = Range::extent(start.clone(), &extent, origin.clone()).expect("full tail");
        let coordinates = range.coordinates().collect::<alloc::vec::Vec<_>>();

        assert_eq!(range.endpoint(), &256_u16.into());
        assert_eq!(range.size(), extent);
        assert_eq!(coordinates.len(), 16);
        assert_eq!(coordinates.first(), Some(&240_u16.into()));
        assert_eq!(coordinates.last(), Some(&255_u16.into()));
        assert!(range.end(origin.clone()).is_err());
        assert!(start.add(&extent, origin).is_err());
    }

    #[test]
    fn full_width_measures_are_not_truncated_to_an_address_or_u128() {
        let origin = Origin::new(Id::new("range"), Span::unit(0));
        let domain = Arc::new(domain::Address::new(origin.clone(), Width::new(128).expect("width")));
        let boundary = domain.boundary();
        let start = Address::new(domain, Number::integer(0), origin.clone()).expect("coordinate");
        let range = Range::new(start, boundary.clone(), Mode::Exclusive, origin.clone()).expect("whole domain");

        assert_eq!(range.size().magnitude(), &(BigUint::from(1_u8) << 128_usize));
        assert_eq!(range.endpoint(), &boundary);
        assert!(range.end(origin).is_err());
    }

    #[test]
    fn ordering_inclusive_bounds_and_empty_intervals_are_distinct() {
        let origin = Origin::new(Id::new("range"), Span::unit(0));
        let domain = Arc::new(domain::Address::new(origin.clone(), Width::new(8).expect("width")));
        let start = Address::new(domain, Number::integer(10), origin.clone()).expect("coordinate");
        let empty = Range::new(start.clone(), 10_u8.into(), Mode::Exclusive, origin.clone()).expect("empty range");
        let singleton =
            Range::new(start.clone(), 10_u8.into(), Mode::Inclusive, origin.clone()).expect("closed singleton");

        assert_eq!(empty.coordinates().next(), None);
        assert_eq!(empty.size().magnitude(), &0_u8.into());
        assert_eq!(singleton.coordinates().count(), 1);
        assert_eq!(singleton.size().magnitude(), &1_u8.into());
        assert_eq!(
            Range::new(start.clone(), 9_u8.into(), Mode::Exclusive, origin.clone())
                .expect_err("reversed")
                .reason(),
            Reason::Reversed
        );
        assert_eq!(
            Range::new(start.clone(), 256_u16.into(), Mode::Inclusive, origin.clone())
                .expect_err("closed one past")
                .reason(),
            Reason::Width
        );
        assert_eq!(
            Range::new(start, 257_u16.into(), Mode::Exclusive, origin)
                .expect_err("overflow")
                .reason(),
            Reason::Width
        );
    }

    #[test]
    fn explicit_endpoints_cannot_cross_nominal_domains() {
        let origin = Origin::new(Id::new("range"), Span::unit(0));
        let first = Arc::new(domain::Address::new(origin.clone(), Width::new(8).expect("width")));
        let second = Arc::new(domain::Address::new(
            Origin::new(Id::new("other"), Span::unit(0)),
            Width::new(8).expect("width"),
        ));
        let start = Address::new(first, Number::integer(1), origin.clone()).expect("coordinate");
        let end = Address::new(second, Number::integer(2), origin.clone()).expect("coordinate");

        assert_eq!(
            Range::between(start, &end, Mode::Exclusive, origin)
                .expect_err("different domains")
                .reason(),
            Reason::Domain
        );
    }
}

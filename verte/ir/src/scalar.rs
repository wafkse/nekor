//! Ordered scalar coordinates with declared representations and nominal identities.
//!
//! Exact magnitudes support comparisons and interval arithmetic without widening
//! typed integers or converting quantities to floating point. An equal magnitude
//! alone never establishes compatibility between different scalar types.

use alloc::{boxed::Box, sync::Arc};
use core::cmp::Ordering;

use aldebaran::{
    prelude::Report,
    report::annotated::{InlineAnnotations, label::Label},
};
use fack::prelude::Error;

use crate::{
    address::Address,
    float::Float,
    integer::Integer,
    number::Number,
    quantity::Quantity,
    resource::{count::Count, index::Index},
    source::Origin,
    types::Endpoint,
};

/// A representable ordered value with its fixed or nominal scalar type intact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scalar {
    /// A checked fixed-width integer.
    Integer(Integer),

    /// A finite IEEE value at its declared precision.
    Float(Float),

    /// An exact dimensional magnitude with an arithmetic kind.
    Quantity(Box<Quantity>),

    /// A width-checked coordinate in a nominal byte-addressed space.
    Address(Address),

    /// A nominal interrupt or GPIO coordinate.
    Index(Index),

    /// An integral magnitude in a nominal count kind.
    Count(Count),
}

impl Scalar {
    /// Recover the magnitude-independent ordered scalar type.
    #[inline]
    pub fn endpoint(&self) -> Endpoint {
        match self {
            Self::Integer(value) => Endpoint::Integer(value.format()),
            Self::Float(value) => Endpoint::Float(value.format()),
            Self::Quantity(value) => Endpoint::Quantity(Arc::new(value.signature())),
            Self::Address(value) => Endpoint::Address(Arc::clone(value.domain())),
            Self::Index(value) => Endpoint::Index(Arc::clone(value.domain())),
            Self::Count(value) => Endpoint::Count(Arc::clone(value.domain())),
        }
    }

    /// Recover the exact mathematical coordinate in canonical units.
    ///
    /// The returned magnitude does not carry permission to reinterpret a nominal
    /// value. Callers must preserve the endpoint type when constructing a result.
    #[inline]
    pub fn exact(&self) -> Number {
        match self {
            Self::Integer(value) => value.value().clone(),
            Self::Float(value) => value.exact(),
            Self::Quantity(value) => value.magnitude().clone(),
            Self::Address(value) => Number::integer(value.coordinate().clone()),
            Self::Index(value) => Number::integer(value.coordinate().clone()),
            Self::Count(value) => Number::integer(value.magnitude().clone()),
        }
    }

    /// Compare magnitudes only after declared formats, dimensions, and domains agree.
    #[inline]
    pub fn compare(&self, other: &Self, origin: Origin) -> Result<Ordering, Incompatible> {
        let left = self.endpoint();
        let right = other.endpoint();

        if left == right {
            Ok(self.exact().cmp(&other.exact()))
        } else {
            let labels = InlineAnnotations::new(
                Label::new(
                    "these scalar values have incompatible types or nominal domains",
                    origin.span(),
                ),
                [],
            );

            Err(Incompatible {
                origin,
                left,
                right,
                labels,
            })
        }
    }
}

/// Ordered scalar operands disagree in representation, dimensions, or nominal domain.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("incompatible scalar operands")]
#[report(title = "incompatible scalar operands", annotations = labels)]
// NOTE(invariant): The labels use the source coordinates qualified by `origin`.
pub struct Incompatible {
    /// Source-qualified operation location.
    origin: Origin,

    /// Type of the left operand.
    left: Endpoint,

    /// Type of the right operand.
    right: Endpoint,

    /// Primary operation location.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

impl Incompatible {
    /// Retrieve the canonical source and original coordinates of the operation.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        let Self { origin, .. } = self;

        origin
    }

    /// Inspect both incompatible endpoint types without parsing diagnostic text.
    #[inline]
    pub const fn endpoints(&self) -> (&Endpoint, &Endpoint) {
        let Self { left, right, .. } = self;

        (left, right)
    }
}

#[cfg(test)]
mod tests {
    use alloc::sync::Arc;
    use core::cmp::Ordering;

    use aldebaran::prelude::Span;

    use super::Scalar;
    use crate::{
        domain,
        float::{Float, Format as FloatFormat},
        integer::{Format, Integer, Width},
        number::Number,
        resource::count::Count,
        source::{Id, Origin},
    };

    #[test]
    fn equal_magnitudes_do_not_create_implicit_width_or_domain_conversions() {
        let origin = Origin::new(Id::new("scalar"), Span::unit(0));
        let narrow = Scalar::Integer(
            Integer::new(Format::Unsigned(Width::W8), Number::integer(1), origin.clone()).expect("narrow"),
        );
        let wide = Scalar::Integer(
            Integer::new(Format::Unsigned(Width::W16), Number::integer(1), origin.clone()).expect("wide"),
        );
        let first = Scalar::Count(Count::new(
            Arc::new(domain::Count::new(origin.clone(), "page")),
            1_u8.into(),
        ));
        let second = Scalar::Count(Count::new(
            Arc::new(domain::Count::new(Origin::new(Id::new("other"), Span::unit(0)), "page")),
            1_u8.into(),
        ));

        assert_eq!(narrow.exact(), wide.exact());
        assert_eq!(first.exact(), second.exact());
        assert!(narrow.compare(&wide, origin.clone()).is_err());
        assert!(first.compare(&second, origin.clone()).is_err());
        assert_eq!(
            narrow.compare(&narrow, origin).expect("identical type"),
            Ordering::Equal
        );
    }

    #[test]
    fn finite_ieee_coordinates_recover_exact_rationals_including_subnormals() {
        let origin = Origin::new(Id::new("scalar"), Span::unit(0));
        let smallest = Number::binary(1, -1074);
        let value = Scalar::Float(Float::new(FloatFormat::Double, &smallest, origin.clone()).expect("subnormal"));
        let negative =
            Scalar::Float(Float::new(FloatFormat::Double, &smallest.negate(), origin.clone()).expect("negative"));

        assert_eq!(value.exact(), smallest);
        assert_eq!(negative.exact(), smallest.negate());
        assert_eq!(
            value.compare(&negative, origin).expect("same precision"),
            Ordering::Greater
        );
    }
}

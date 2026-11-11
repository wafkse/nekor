//! Exact dimensional quantities with affine and logarithmic arithmetic rules.

use aldebaran::{
    prelude::Report,
    report::annotated::{InlineAnnotations, label::Label},
};
use fack::prelude::Error;

use crate::{
    dimension::{Axis, Dimension},
    number::{Number, ZeroDivisor},
    source::Origin,
};

/// Arithmetic behavior not expressed by physical dimensions alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Ordinary multiplicative dimensional arithmetic, including temperature differences.
    Linear,

    /// An absolute temperature expressed canonically in kelvin.
    Temperature,

    /// A logarithmic gain expressed in decibels.
    Gain,
}

/// A quantity's physical dimensions and arithmetic kind, independent of magnitude.
#[derive(Debug, Clone, PartialEq, Eq)]
// NOTE(invariant): Absolute temperatures have temperature dimension; gains are dimensionless.
pub struct Signature {
    /// Physical and semantic dimension factors required by the type.
    dimension: Dimension,

    /// Affine or logarithmic arithmetic category.
    kind: Kind,
}

impl Signature {
    /// Describe an ordinary multiplicative quantity type.
    #[inline]
    pub const fn linear(dimension: Dimension) -> Self {
        Self {
            dimension,
            kind: Kind::Linear,
        }
    }

    /// Describe absolute temperature independently of its source unit scale.
    #[inline]
    pub fn temperature() -> Self {
        let dimension = Dimension::axis(Axis::Temperature);

        Self {
            dimension,
            kind: Kind::Temperature,
        }
    }

    /// Describe logarithmic gain without identifying it with a linear ratio.
    #[inline]
    pub fn gain() -> Self {
        let dimension = Dimension::scalar();

        Self {
            dimension,
            kind: Kind::Gain,
        }
    }

    /// Retrieve the arithmetic category required by this signature.
    #[inline]
    pub const fn kind(&self) -> Kind {
        let &Self { kind, .. } = self;

        kind
    }

    /// Borrow the required physical and semantic dimensions.
    #[inline]
    pub const fn dimension(&self) -> &Dimension {
        let Self { dimension, .. } = self;

        dimension
    }

    /// Compare kind and dimensions independently of magnitudes and unit names.
    #[inline]
    pub fn accepts(&self, value: &Quantity) -> bool {
        let Self { dimension, kind } = self;

        *kind == value.kind() && dimension == value.dimension()
    }
}

/// A dimensional magnitude with exact canonical units.
#[derive(Debug, Clone, PartialEq, Eq)]
// NOTE(invariant): Absolute temperatures have temperature dimension; gains are dimensionless.
pub struct Quantity {
    /// Magnitude in the dimension's canonical scale.
    magnitude: Number,

    /// Physical and semantic dimension factors.
    dimension: Dimension,

    /// Affine or logarithmic restrictions on permitted operations.
    kind: Kind,
}

impl Quantity {
    /// Retrieve the magnitude-independent type without forgetting semantic tags.
    #[inline]
    pub fn signature(&self) -> Signature {
        let Self { dimension, kind, .. } = self;

        let dimension = dimension.clone();
        let kind = *kind;

        Signature { dimension, kind }
    }

    /// Construct a linear quantity in canonical units.
    #[inline]
    pub const fn linear(magnitude: Number, dimension: Dimension) -> Self {
        Self {
            magnitude,
            dimension,
            kind: Kind::Linear,
        }
    }

    /// Construct an absolute temperature from an exact kelvin magnitude.
    #[inline]
    pub fn temperature(magnitude: Number) -> Self {
        let dimension = Dimension::axis(Axis::Temperature);

        Self {
            magnitude,
            dimension,
            kind: Kind::Temperature,
        }
    }

    /// Construct a gain in decibels without interpreting it as a linear ratio.
    #[inline]
    pub fn gain(magnitude: Number) -> Self {
        let dimension = Dimension::scalar();

        Self {
            magnitude,
            dimension,
            kind: Kind::Gain,
        }
    }

    /// Borrow the exact canonical magnitude.
    #[inline]
    pub const fn magnitude(&self) -> &Number {
        let Self { magnitude, .. } = self;

        magnitude
    }

    /// Borrow physical and semantic dimension factors.
    #[inline]
    pub const fn dimension(&self) -> &Dimension {
        let Self { dimension, .. } = self;

        dimension
    }

    /// Retrieve restrictions independent of the dimension signature.
    #[inline]
    pub const fn kind(&self) -> Kind {
        let &Self { kind, .. } = self;

        kind
    }

    /// Add compatible quantities, allowing an absolute temperature and a difference.
    #[inline]
    pub fn add(&self, other: &Self, origin: Origin) -> Result<Self, Incompatible> {
        let Self {
            magnitude: left,
            dimension,
            kind: left_kind,
        } = self;

        let Self {
            magnitude: right,
            dimension: right_dimension,
            kind: right_kind,
        } = other;

        let kind = match (dimension == right_dimension, left_kind, right_kind) {
            (true, Kind::Linear, Kind::Linear) => Kind::Linear,
            (true, Kind::Gain, Kind::Gain) => Kind::Gain,
            (true, Kind::Temperature, Kind::Linear) | (true, Kind::Linear, Kind::Temperature) => Kind::Temperature,
            _ => return Err(Incompatible::new(origin)),
        };
        let magnitude = left.add(right);
        let dimension = dimension.clone();

        Ok(Self {
            magnitude,
            dimension,
            kind,
        })
    }

    /// Subtract absolute temperatures into a linear temperature difference.
    #[inline]
    pub fn subtract(&self, other: &Self, origin: Origin) -> Result<Self, Incompatible> {
        let Self {
            magnitude: left,
            dimension,
            kind: left_kind,
        } = self;

        let Self {
            magnitude: right,
            dimension: right_dimension,
            kind: right_kind,
        } = other;

        let kind = match (dimension == right_dimension, left_kind, right_kind) {
            (true, Kind::Linear, Kind::Linear) | (true, Kind::Temperature, Kind::Temperature) => Kind::Linear,
            (true, Kind::Gain, Kind::Gain) => Kind::Gain,
            (true, Kind::Temperature, Kind::Linear) => Kind::Temperature,
            _ => return Err(Incompatible::new(origin)),
        };
        let magnitude = left.subtract(right);
        let dimension = dimension.clone();

        Ok(Self {
            magnitude,
            dimension,
            kind,
        })
    }

    /// Multiply linear quantities without dropping semantic dimension factors.
    #[inline]
    pub fn multiply(&self, other: &Self, origin: Origin) -> Result<Self, Incompatible> {
        let Self {
            magnitude: left,
            dimension: left_dimension,
            kind: left_kind,
        } = self;

        let Self {
            magnitude: right,
            dimension: right_dimension,
            kind: right_kind,
        } = other;

        match (left_kind, right_kind) {
            (Kind::Linear, Kind::Linear) => {
                let magnitude = left.multiply(right);
                let dimension = left_dimension.multiply(right_dimension);

                Ok(Self::linear(magnitude, dimension))
            },
            _ => Err(Incompatible::new(origin)),
        }
    }

    /// Divide linear quantities with exact dimensions and a checked divisor.
    #[inline]
    pub fn divide(&self, other: &Self, origin: Origin) -> Result<Self, DivisionError> {
        let Self {
            magnitude: left,
            dimension: left_dimension,
            kind: left_kind,
        } = self;

        let Self {
            magnitude: right,
            dimension: right_dimension,
            kind: right_kind,
        } = other;

        match (left_kind, right_kind) {
            (Kind::Linear, Kind::Linear) => {
                let magnitude = left.divide(right, origin).map_err(DivisionError::Zero)?;
                let dimension = left_dimension.divide(right_dimension);

                Ok(Self::linear(magnitude, dimension))
            },
            _ => Err(DivisionError::Kinds(Incompatible::new(origin))),
        }
    }
}

/// A dimensional or semantic kind mismatch in quantity arithmetic.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("incompatible quantity operands")]
#[report(title = "incompatible quantity operands", annotations = labels)]
// NOTE(invariant): The label uses the coordinates qualified by `origin`.
pub struct Incompatible {
    /// Canonical source of the rejected operation.
    origin: Origin,

    /// Location of the operation.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

impl Incompatible {
    /// Identify an operation with incompatible dimensions or semantic kinds.
    #[inline]
    pub const fn new(origin: Origin) -> Self {
        let labels = InlineAnnotations::new(
            Label::new(
                "these quantity operands cannot be combined by this operation",
                origin.span(),
            ),
            [],
        );

        Self { origin, labels }
    }

    /// Retrieve the canonical source of the rejected operation.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        let Self { origin, .. } = self;

        origin
    }
}

/// A quantity division rejected by arithmetic kind or zero-divisor rules.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[report(forward)]
pub enum DivisionError {
    /// The operands do not permit multiplicative division.
    #[error(transparent(0))]
    Kinds(Incompatible),

    /// The divisor's exact magnitude is zero.
    #[error(transparent(0))]
    Zero(ZeroDivisor),
}

#[cfg(test)]
mod tests {
    use aldebaran::prelude::Span;

    use super::{Kind, Quantity};
    use crate::{
        dimension::{Axis, Dimension, Tag},
        number::Number,
        source::{Id, Origin},
    };

    #[test]
    fn affine_temperatures_preserve_the_distinction_between_values_and_differences() {
        let origin = Origin::new(Id::new("test"), Span::unit(0));
        let absolute = Quantity::temperature(Number::integer(300));
        let other = Quantity::temperature(Number::integer(290));
        let difference = absolute
            .subtract(&other, origin.clone())
            .expect("temperature difference");

        assert_eq!(difference.kind(), Kind::Linear);
        assert_eq!(difference.magnitude(), &Number::integer(10));
        assert_eq!(
            other.add(&difference, origin.clone()).expect("absolute plus delta"),
            absolute
        );
        assert!(absolute.add(&other, origin.clone()).is_err());
        assert!(difference.subtract(&absolute, origin.clone()).is_err());

        let scalar = Quantity::linear(Number::integer(2), Dimension::scalar());

        assert!(absolute.multiply(&scalar, origin).is_err());
    }

    #[test]
    fn dimensional_cancellation_preserves_semantic_tags() {
        let origin = Origin::new(Id::new("test"), Span::unit(0));
        let size = Quantity::linear(Number::integer(8), Dimension::axis(Axis::Information));
        let duration = Quantity::linear(Number::integer(2), Dimension::axis(Axis::Time));
        let rate = size.divide(&duration, origin.clone()).expect("information rate");

        assert_eq!(rate.multiply(&duration, origin.clone()).expect("information"), size);

        let degrees = Quantity::linear(Number::integer(1), Dimension::tag(Tag::Degree));
        let radians = Quantity::linear(Number::integer(1), Dimension::tag(Tag::Radian));
        let frequency = Quantity::linear(Number::integer(1), Dimension::axis(Axis::Time).power(&(-1).into()));
        let symbols = Dimension::tag(Tag::Symbol).divide(&Dimension::axis(Axis::Time));
        let baud = Quantity::linear(Number::integer(1), symbols);

        assert!(degrees.add(&radians, origin.clone()).is_err());
        assert!(frequency.add(&baud, origin).is_err());
    }

    #[test]
    fn logarithmic_gain_is_not_a_linear_ratio() {
        let origin = Origin::new(Id::new("test"), Span::unit(0));
        let gain = Quantity::gain(Number::integer(3));
        let ratio = Quantity::linear(Number::integer(3), Dimension::scalar());

        assert_eq!(
            gain.add(&gain, origin.clone()).expect("decibel addition"),
            Quantity::gain(Number::integer(6))
        );
        assert!(gain.add(&ratio, origin.clone()).is_err());
        assert!(gain.multiply(&gain, origin).is_err());
    }
}

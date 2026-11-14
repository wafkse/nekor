//! Owned structural types and nominal scalar identities shared across evaluation phases.
//!
//! Compatibility describes whether declarations can participate in inheritance.
//! Array lengths, enumeration sets, and closed record shapes remain part of their
//! respective types and must be checked on the resulting value as well.

use alloc::{boxed::Box, sync::Arc};

use num_bigint::BigUint;

use crate::{domain, float, integer, quantity::Signature, record::Record};

pub mod enumeration;

use enumeration::Enumeration;

/// A resolved type, independent of aliases and source-buffer lifetimes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    /// Boolean truth values.
    Bool,

    /// Decoded UTF-8 strings.
    String,

    /// Fixed-width, signed or unsigned integers.
    Integer(integer::Format),

    /// Finite IEEE values of an explicit precision.
    Float(float::Format),

    /// Exact dimensional values with physical and semantic factors.
    Quantity(Arc<Signature>),

    /// Coordinates in a source-declared byte-addressed space.
    Address(Arc<domain::Address>),

    /// Interrupt or GPIO identifiers in a source-declared namespace.
    Index(Arc<domain::Index>),

    /// String references admitted by a source-declared namespace.
    Reference(Arc<domain::Reference>),

    /// A nonnegative integral nominal count.
    Count(Arc<domain::Count>),

    /// Nominal unsigned masks distinct from integers of the same width.
    Mask(integer::Width),

    /// Homogeneous sequences with an optional exact length requirement.
    Array(Array),

    /// Closed records with declaration locations for their member types.
    Record(Record<Type>),

    /// Nonempty closed sets of decoded strings.
    Enumeration(Enumeration),

    /// Intervals whose endpoints have a supported ordered scalar type.
    Range(Endpoint),
}

impl Type {
    /// Determine underlying compatibility without erasing either type's value constraints.
    ///
    /// Record members shared by both declarations must be compatible. Their closed
    /// shapes still apply independently during validation. The same applies to array
    /// lengths and enumeration membership, which are not winner-selection policies.
    #[inline]
    pub fn compatible(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Bool, Self::Bool) | (Self::String, Self::String) => true,
            (Self::Integer(left), Self::Integer(right)) => left == right,
            (Self::Float(left), Self::Float(right)) => left == right,
            (Self::Quantity(left), Self::Quantity(right)) => left == right,
            (Self::Address(left), Self::Address(right)) => left == right,
            (Self::Index(left), Self::Index(right)) => left == right,
            (Self::Reference(left), Self::Reference(right)) => left == right,
            (Self::Count(left), Self::Count(right)) => left == right,
            (Self::Mask(left), Self::Mask(right)) => left == right,
            (Self::Array(left), Self::Array(right)) => left.element().compatible(right.element()),
            (Self::Record(left), Self::Record(right)) => left.iter().all(|member| {
                right
                    .get(member.name())
                    .is_none_or(|other| member.value().compatible(other.value()))
            }),
            (Self::Enumeration(_), Self::Enumeration(_)) => true,
            (Self::Range(left), Self::Range(right)) => left == right,
            _ => false,
        }
    }
}

/// A homogeneous list or fixed-size array with an exact mathematical length.
#[derive(Debug, Clone, PartialEq, Eq)]
// NOTE(invariant): None.
pub struct Array {
    /// Resolved element type.
    element: Box<Type>,

    /// Required length, absent for lists and not narrowed to a host pointer width.
    length: Option<BigUint>,
}

impl Array {
    /// Describe a homogeneous sequence with an optional exact length requirement.
    #[inline]
    pub fn new(element: Type, length: Option<BigUint>) -> Self {
        let element = Box::new(element);

        Self { element, length }
    }

    /// Borrow the resolved element type.
    #[inline]
    pub const fn element(&self) -> &Type {
        let Self { element, .. } = self;

        element
    }

    /// Borrow the fixed length, preserving values beyond the host collection width.
    #[inline]
    pub const fn length(&self) -> Option<&BigUint> {
        let Self { length, .. } = self;

        length.as_ref()
    }
}

/// Scalar types with a defined interval ordering and measure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Endpoint {
    /// Fixed-width integral coordinates.
    Integer(integer::Format),

    /// Finite IEEE coordinates at their declared precision.
    Float(float::Format),

    /// Dimensional coordinates distinguished by linear, affine, or logarithmic kind.
    Quantity(Arc<Signature>),

    /// Byte coordinates in a nominal address domain.
    Address(Arc<domain::Address>),

    /// Nonnegative nominal resource coordinates.
    Index(Arc<domain::Index>),

    /// Nonnegative nominal count coordinates.
    Count(Arc<domain::Count>),
}

impl Endpoint {
    /// Recover the endpoint's scalar type for contextual checking and projections.
    #[inline]
    pub fn scalar(&self) -> Type {
        match self {
            Self::Integer(format) => Type::Integer(*format),
            Self::Float(format) => Type::Float(*format),
            Self::Quantity(signature) => Type::Quantity(Arc::clone(signature)),
            Self::Address(domain) => Type::Address(Arc::clone(domain)),
            Self::Index(domain) => Type::Index(Arc::clone(domain)),
            Self::Count(domain) => Type::Count(Arc::clone(domain)),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::sync::Arc;

    use aldebaran::prelude::Span;
    use num_bigint::BigUint;

    use super::{Array, Type};
    use crate::{
        domain,
        integer::{Format, Width},
        record::{Member, Record},
        source::{Id, Origin},
    };

    #[test]
    fn compatibility_keeps_widths_nominal_domains_and_array_length_contracts_distinct() {
        let origin = Origin::new(Id::new("type"), Span::unit(0));
        let other = Origin::new(Id::new("other"), Span::unit(0));
        let narrow = Type::Integer(Format::Unsigned(Width::W8));
        let wide = Type::Integer(Format::Unsigned(Width::W16));
        let length = BigUint::from(1_u8) << 256_usize;
        let fixed = Type::Array(Array::new(narrow.clone(), Some(length.clone())));
        let list = Type::Array(Array::new(narrow.clone(), None));
        let first = Type::Address(Arc::new(domain::Address::new(
            origin,
            domain::Width::new(8).expect("width"),
        )));
        let second = Type::Address(Arc::new(domain::Address::new(
            other,
            domain::Width::new(8).expect("width"),
        )));

        assert!(!narrow.compatible(&wide));
        assert!(!first.compatible(&second));
        assert!(!narrow.compatible(&Type::Mask(Width::W8)));
        assert!(fixed.compatible(&list));
        assert!(list.compatible(&fixed));
        assert_eq!(fixed, Type::Array(Array::new(narrow, Some(length))));
        assert_ne!(fixed, list);
    }

    #[test]
    fn record_compatibility_checks_common_members_without_replacing_closed_shapes() {
        let origin = Origin::new(Id::new("record type"), Span::unit(0));
        let mut left = Record::new();
        let mut right = Record::new();
        let mut incompatible = Record::new();

        left.insert(Member::new("common", origin.clone(), Type::Bool))
            .expect("member");
        right
            .insert(Member::new("common", origin.clone(), Type::Bool))
            .expect("member");
        right
            .insert(Member::new("additional", origin.clone(), Type::String))
            .expect("member");
        incompatible
            .insert(Member::new("common", origin, Type::String))
            .expect("member");

        let left = Type::Record(left);
        let right = Type::Record(right);

        assert!(left.compatible(&right));
        assert!(right.compatible(&left));
        assert_ne!(left, right);
        assert!(!left.compatible(&Type::Record(incompatible)));
    }
}

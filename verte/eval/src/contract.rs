//! Structural type requirements with locations for nested declarations.
//!
//! Admission contextualizes abstract numbers but never narrows an already typed
//! scalar or converts between nominal domains. A replacement must satisfy every
//! applicable requirement, including closed records and fixed array lengths.

use alloc::{boxed::Box, sync::Arc, vec, vec::Vec};
use core::slice::Iter;

use num_bigint::BigUint;
use verte_ir::{
    address::Address,
    dimension::{Dimension, Tag},
    float::Float,
    integer::{Integer, mask::Mask},
    quantity::{Kind, Quantity},
    record::Record,
    resource::{count::Count, index::Index, reference::Reference},
    scalar::Scalar,
    source::Origin,
    types::{Array, Type},
    value::{Data, Value},
};

pub mod admission;

use admission::{Admission, Failure};

/// A declared structural requirement with independently located nested types.
#[derive(Debug, Clone)]
// NOTE(invariant): Atomic shapes never contain an array or record type.
pub struct Contract {
    /// Source-qualified type declaration.
    origin: Origin,

    /// Requirement owning every nested member and element contract.
    shape: Shape,
}

/// Structural composition of independently owned type requirements.
#[derive(Debug, Clone)]
enum Shape {
    /// A scalar, enumeration, or interval type.
    Atom(Type),

    /// A homogeneous sequence and its optional mathematical length.
    Array {
        /// Requirement applied to every element.
        element: Arc<Contract>,

        /// Exact required length, absent for lists.
        length: Option<BigUint>,
    },

    /// A closed set of independently located member requirements.
    Record(Record<Arc<Contract>>),
}

impl Contract {
    /// Establish a type requirement with declaration locations for record members.
    #[inline]
    pub fn new(origin: Origin, ty: Type) -> Self {
        match ty {
            Type::Array(array) => {
                let element = Arc::new(Self::new(origin.clone(), array.element().clone()));

                Self::array(origin, element, array.length().cloned())
            },
            Type::Record(record) => {
                let members = record.map(|_, origin, ty| Arc::new(Self::new(origin.clone(), ty)));

                Self::record(origin, members)
            },
            ty => Self {
                origin,
                shape: Shape::Atom(ty),
            },
        }
    }

    /// Require every sequence element to satisfy its own located contract.
    #[inline]
    pub const fn array(origin: Origin, element: Arc<Self>, length: Option<BigUint>) -> Self {
        let shape = Shape::Array { element, length };

        Self { origin, shape }
    }

    /// Require exactly the declared record members and their nested contracts.
    #[inline]
    pub const fn record(origin: Origin, members: Record<Arc<Self>>) -> Self {
        let shape = Shape::Record(members);

        Self { origin, shape }
    }

    /// Retrieve the canonical source and coordinates of the type requirement.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        let Self { origin, .. } = self;

        origin
    }

    /// Recover the structural type, including fixed lengths and closed record members.
    #[inline]
    pub fn ty(&self) -> Type {
        let Self { shape, .. } = self;

        match shape {
            Shape::Atom(ty) => ty.clone(),
            Shape::Array { element, length } => Type::Array(Array::new(element.ty(), length.clone())),
            Shape::Record(members) => Type::Record(members.clone().map(|_, _, contract| contract.ty())),
        }
    }

    /// Check a candidate and attach the requirement that established its representation.
    #[inline]
    pub fn admit(self: &Arc<Self>, value: Value) -> Result<Checked, Admission> {
        let value = self.convert(value)?;
        let contracts = vec![Arc::clone(self)];

        Ok(Checked { value, contracts })
    }

    /// Contextualize and check the candidate without forgetting nested provenance.
    fn convert(&self, value: Value) -> Result<Value, Admission> {
        let Self { shape, .. } = self;

        let origin = value.origin().clone();
        let data = match (shape, value.into_data()) {
            (Shape::Atom(ty), data) => self.atom(ty, data, &origin),
            (Shape::Array { element, length }, Data::Array(values)) => match length {
                Some(expected) if *expected != BigUint::from(values.len()) => Err(Admission::new(
                    self,
                    origin.clone(),
                    Failure::Length {
                        expected: expected.clone(),
                        actual: values.len(),
                    },
                )),
                Some(_) | None => {
                    let values = values
                        .into_iter()
                        .map(|value| element.convert(value))
                        .collect::<Result<Vec<_>, _>>()?;

                    Ok(Data::Array(values))
                },
            },
            (Shape::Record(required), Data::Record(values)) => {
                match required.iter().find(|member| values.get(member.name()).is_none()) {
                    Some(member) => Err(Admission::new(
                        member.value(),
                        origin.clone(),
                        Failure::Missing(Arc::from(member.name())),
                    )),
                    None => values
                        .try_map(|name, key, value| match required.get(name) {
                            Some(member) => member.value().convert(value),
                            None => Err(Admission::new(self, key.clone(), Failure::Unexpected(Arc::from(name)))),
                        })
                        .map(Data::Record),
                }
            },
            (_, _) => Err(Admission::new(self, origin.clone(), Failure::Type)),
        }?;

        Ok(Value::new(origin, data))
    }

    /// Apply an atomic requirement without introducing implicit typed conversions.
    fn atom(&self, ty: &Type, data: Data, origin: &Origin) -> Result<Data, Admission> {
        let result = match (ty, data) {
            (Type::Bool, Data::Bool(value)) => Ok(Data::Bool(value)),
            (Type::String, Data::String(value)) => Ok(Data::String(value)),
            (Type::Enumeration(required), Data::String(value)) if required.contains(&value) => Ok(Data::String(value)),
            (Type::Enumeration(_), Data::String(_)) => Err(Failure::Enumeration),
            (Type::Integer(format), Data::Number(value)) => Integer::new(*format, value, origin.clone())
                .map(Scalar::Integer)
                .map(Data::Scalar)
                .map_err(Failure::Integer),
            (Type::Float(format), Data::Number(value)) => Float::new(*format, &value, origin.clone())
                .map(Scalar::Float)
                .map(Data::Scalar)
                .map_err(Failure::Float),
            (Type::Address(domain), Data::Number(value)) => Address::new(Arc::clone(domain), value, origin.clone())
                .map(Scalar::Address)
                .map(Data::Scalar)
                .map_err(Failure::Address),
            (Type::Index(domain), Data::Number(value)) => match value.natural() {
                Some(coordinate) => Ok(Data::Scalar(Scalar::Index(Index::new(Arc::clone(domain), coordinate)))),
                None => Err(Failure::Natural),
            },
            (Type::Count(domain), Data::Number(value)) => match value.natural() {
                Some(magnitude) => Ok(Data::Scalar(Scalar::Count(Count::new(Arc::clone(domain), magnitude)))),
                None => Err(Failure::Natural),
            },
            (Type::Count(domain), Data::Scalar(Scalar::Quantity(value))) => {
                let dimension = Dimension::tag(Tag::Count(domain.origin().clone()));

                match (value.kind(), value.dimension(), value.magnitude().natural()) {
                    (Kind::Linear, actual, Some(magnitude)) if *actual == dimension => {
                        Ok(Data::Scalar(Scalar::Count(Count::new(Arc::clone(domain), magnitude))))
                    },
                    (Kind::Linear, actual, None) if *actual == dimension => Err(Failure::Natural),
                    _ => Err(Failure::Type),
                }
            },
            (Type::Quantity(signature), Data::Number(value))
                if signature.kind() == Kind::Linear && *signature.dimension() == Dimension::scalar() =>
            {
                Ok(Data::Scalar(Scalar::Quantity(Box::new(Quantity::linear(
                    value,
                    Dimension::scalar(),
                )))))
            },
            (Type::Mask(width), Data::Number(value)) => Mask::new(*width, value, origin.clone())
                .map(Data::Mask)
                .map_err(Failure::Integer),
            (Type::Mask(width), Data::Mask(value)) if value.width() == *width => Ok(Data::Mask(value)),
            (Type::Reference(domain), Data::String(value)) => Reference::new(Arc::clone(domain), value, origin.clone())
                .map(Data::Reference)
                .map_err(Failure::Reference),
            (Type::Reference(domain), Data::Reference(value)) if value.domain() == domain => Ok(Data::Reference(value)),
            (Type::Range(endpoint), Data::Range(value)) if *endpoint == value.endpoint() => Ok(Data::Range(value)),
            (required, Data::Scalar(value)) if required.compatible(&value.endpoint().scalar()) => {
                Ok(Data::Scalar(value))
            },
            (_, _) => Err(Failure::Type),
        };

        result.map_err(|failure| Admission::new(self, origin.clone(), failure))
    }
}

/// A represented value satisfying all attached structural requirements.
#[derive(Debug, Clone)]
// NOTE(invariant): `contracts` is nonempty, and `value` satisfies every stored contract.
pub struct Checked {
    /// Represented semantic value and its original provenance.
    value: Value,

    /// Requirements that every replacement must also satisfy.
    contracts: Vec<Arc<Contract>>,
}

impl Checked {
    /// Inspect the represented value without discarding its attached requirements.
    #[inline]
    pub const fn value(&self) -> &Value {
        let Self { value, .. } = self;

        value
    }

    /// Inspect applicable declarations in their admission order.
    #[inline]
    pub fn contracts(&self) -> Iter<'_, Arc<Contract>> {
        let Self { contracts, .. } = self;

        contracts.iter()
    }

    /// Check a replacement against every attached requirement.
    #[inline]
    pub fn replace(&self, mut value: Value) -> Result<Self, Admission> {
        let Self { contracts, .. } = self;

        for contract in contracts {
            value = contract.convert(value)?;
        }

        let contracts = contracts.clone();

        Ok(Self { value, contracts })
    }

    /// Add a compatible requirement and validate the effective value against it.
    #[inline]
    pub fn constrain(self, required: Arc<Contract>) -> Result<Self, Admission> {
        let Self { value, mut contracts } = self;

        match contracts
            .iter()
            .find(|existing| !existing.ty().compatible(&required.ty()))
        {
            Some(existing) => Err(Admission::new(&required, existing.origin().clone(), Failure::Conflict)),
            None => {
                let value = required.convert(value)?;

                contracts.push(required);

                Ok(Self { value, contracts })
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::{boxed::Box, collections::BTreeSet, sync::Arc, vec, vec::Vec};

    use aldebaran::prelude::Span;
    use num_bigint::{BigInt, BigUint};
    use verte_ir::{
        domain,
        float::{Float, Format as FloatFormat},
        integer::{Format, Integer, Width},
        number::Number,
        record::{Member, Record},
        scalar::Scalar,
        source::{Id, Origin},
        types::{Array, Type, enumeration::Enumeration},
        unit::Unit,
        value::{Data, Value},
    };

    use super::{Contract, admission::Failure};

    #[test]
    fn abstract_numbers_acquire_representation_but_typed_values_are_not_narrowed() {
        let declaration = Origin::new(Id::new("types"), Span::unit(2));
        let origin = Origin::new(Id::new("values"), Span::unit(8));
        let required = Arc::new(Contract::new(
            declaration.clone(),
            Type::Integer(Format::Unsigned(Width::W8)),
        ));
        let value = required
            .admit(Value::new(origin.clone(), Data::Number(Number::integer(255))))
            .expect("u8 maximum");

        assert_eq!(value.value().origin(), &origin);

        match value.value().data() {
            Data::Scalar(Scalar::Integer(value)) => {
                assert_eq!(value.format(), Format::Unsigned(Width::W8));
                assert_eq!(value.value(), &Number::integer(255));
            },
            _ => unreachable!("contextual integer representation"),
        }

        for magnitude in [-1, 256] {
            let failure = required
                .admit(Value::new(origin.clone(), Data::Number(Number::integer(magnitude))))
                .expect_err("outside u8");

            assert!(matches!(failure.failure(), Failure::Integer(_)));
            assert_eq!(failure.origin(), &origin);
            assert_eq!(failure.declaration().origin(), &declaration);
        }

        let wide = Integer::new(Format::Unsigned(Width::W16), Number::integer(1), origin.clone()).expect("wide");
        let failure = required
            .admit(Value::new(origin.clone(), Data::Scalar(Scalar::Integer(wide))))
            .expect_err("no implicit narrowing");
        let signed = Arc::new(Contract::new(declaration, Type::Integer(Format::Signed(Width::W128))));
        let minimum = Number::integer(-(BigInt::from(1_u8) << 127_usize));

        assert!(matches!(failure.failure(), Failure::Type));
        assert!(signed.admit(Value::new(origin, Data::Number(minimum))).is_ok());
    }

    #[test]
    fn nested_array_failures_retain_element_origins_and_exact_length_requirements() {
        let declaration = Origin::new(Id::new("types"), Span::unit(2));
        let element_origin = Origin::new(Id::new("types"), Span::unit(7));
        let origin = Origin::new(Id::new("values"), Span::unit(8));
        let bad_origin = Origin::new(Id::new("included-value"), Span::unit(3));
        let element = Arc::new(Contract::new(
            element_origin.clone(),
            Type::Integer(Format::Unsigned(Width::W8)),
        ));
        let required = Arc::new(Contract::array(
            declaration.clone(),
            Arc::clone(&element),
            Some(2_u8.into()),
        ));
        let good = Value::new(origin.clone(), Data::Number(Number::integer(1)));
        let bad = Value::new(bad_origin.clone(), Data::Bool(false));
        let error = required
            .admit(Value::new(origin.clone(), Data::Array(vec![good.clone(), bad])))
            .expect_err("wrong element type");

        assert_eq!(error.origin(), &bad_origin);
        assert_eq!(error.declaration().origin(), &element_origin);
        assert!(matches!(error.failure(), Failure::Type));

        let checked = required
            .admit(Value::new(
                origin.clone(),
                Data::Array(vec![good.clone(), good.clone()]),
            ))
            .expect("fixed array");
        let error = checked
            .replace(Value::new(
                origin.clone(),
                Data::Array(vec![good.clone(), good.clone(), good]),
            ))
            .expect_err("retained length");
        let length = BigUint::from(1_u8) << 256_usize;
        let huge = Arc::new(Contract::array(declaration, element, Some(length.clone())));
        let impossible = huge
            .admit(Value::new(origin, Data::Array(Vec::new())))
            .expect_err("mathematical length");

        assert!(matches!(error.failure(), Failure::Length { actual: 3, .. }));
        assert!(matches!(impossible.failure(), Failure::Length { expected, actual: 0 } if *expected == length));
    }

    #[test]
    fn closed_records_diagnose_missing_and_extra_members_at_the_owning_locations() {
        let declaration = Origin::new(Id::new("types"), Span::unit(2));
        let member_origin = Origin::new(Id::new("types"), Span::unit(7));
        let origin = Origin::new(Id::new("values"), Span::unit(8));
        let extra_origin = Origin::new(Id::new("values"), Span::unit(12));
        let mut members = Record::new();

        members
            .insert(Member::new(
                "enabled",
                member_origin.clone(),
                Arc::new(Contract::new(member_origin.clone(), Type::Bool)),
            ))
            .expect("member");

        let required = Arc::new(Contract::record(declaration.clone(), members));
        let missing = required
            .admit(Value::new(origin.clone(), Data::Record(Record::new())))
            .expect_err("missing key");
        let mut values = Record::new();

        values
            .insert(Member::new(
                "enabled",
                origin.clone(),
                Value::new(origin.clone(), Data::Bool(true)),
            ))
            .expect("member");
        values
            .insert(Member::new(
                "unexpected",
                extra_origin.clone(),
                Value::new(origin.clone(), Data::Bool(false)),
            ))
            .expect("extra member");

        let extra = required
            .admit(Value::new(origin.clone(), Data::Record(values)))
            .expect_err("closed shape");

        assert!(matches!(missing.failure(), Failure::Missing(name) if name.as_ref() == "enabled"));
        assert_eq!(missing.origin(), &origin);
        assert_eq!(missing.declaration().origin(), &member_origin);
        assert!(matches!(extra.failure(), Failure::Unexpected(name) if name.as_ref() == "unexpected"));
        assert_eq!(extra.origin(), &extra_origin);
        assert_eq!(extra.declaration().origin(), &declaration);
    }

    #[test]
    fn added_requirements_survive_subsequent_replacements() {
        let declaration = Origin::new(Id::new("types"), Span::unit(2));
        let origin = Origin::new(Id::new("values"), Span::unit(8));
        let mut first = Record::new();
        let mut second = Record::new();

        for member in ["a", "b"] {
            first
                .insert(Member::new(member, declaration.clone(), ()))
                .expect("enum member");
        }

        for member in ["b", "c"] {
            second
                .insert(Member::new(member, declaration.clone(), ()))
                .expect("enum member");
        }

        let first = Arc::new(Contract::new(
            declaration.clone(),
            Type::Enumeration(Enumeration::new(first, declaration.clone()).expect("nonempty")),
        ));
        let second = Arc::new(Contract::new(
            declaration.clone(),
            Type::Enumeration(Enumeration::new(second, declaration).expect("nonempty")),
        ));
        let value = first
            .admit(Value::new(origin.clone(), Data::String(Arc::from("b"))))
            .expect("first requirement");
        let value = value.constrain(second).expect("both requirements");

        assert_eq!(value.contracts().len(), 2);
        assert!(
            value
                .replace(Value::new(origin.clone(), Data::String(Arc::from("a"))))
                .is_err()
        );
        assert!(
            value
                .replace(Value::new(origin.clone(), Data::String(Arc::from("c"))))
                .is_err()
        );
        assert!(value.replace(Value::new(origin, Data::String(Arc::from("b")))).is_ok());
    }

    #[test]
    fn empty_arrays_do_not_hide_incompatible_element_types() {
        let declaration = Origin::new(Id::new("types"), Span::unit(2));
        let other = Origin::new(Id::new("other-types"), Span::unit(4));
        let origin = Origin::new(Id::new("values"), Span::unit(8));
        let narrow = Arc::new(Contract::new(
            declaration.clone(),
            Type::Array(Array::new(Type::Integer(Format::Unsigned(Width::W8)), None)),
        ));
        let wide = Arc::new(Contract::new(
            other.clone(),
            Type::Array(Array::new(Type::Integer(Format::Unsigned(Width::W16)), None)),
        ));
        let value = narrow
            .admit(Value::new(origin, Data::Array(Vec::new())))
            .expect("empty list");
        let error = value.constrain(wide).expect_err("different element formats");

        assert!(matches!(error.failure(), Failure::Conflict));
        assert_eq!(error.origin(), &declaration);
        assert_eq!(error.declaration().origin(), &other);
    }

    #[test]
    fn numeric_representation_and_nominal_references_never_convert_implicitly() {
        let declaration = Origin::new(Id::new("types"), Span::unit(2));
        let origin = Origin::new(Id::new("values"), Span::unit(8));
        let floating = Arc::new(Contract::new(declaration.clone(), Type::Float(FloatFormat::Double)));
        let single = Float::new(FloatFormat::Single, &Number::integer(1), origin.clone()).expect("single");
        let members = BTreeSet::from([Arc::from("uart")]);
        let domain = Arc::new(domain::Reference::new(declaration.clone(), Some(members)));
        let required = Arc::new(Contract::new(declaration.clone(), Type::Reference(domain)));

        assert!(
            floating
                .admit(Value::new(origin.clone(), Data::Scalar(Scalar::Float(single))))
                .is_err()
        );
        assert!(
            floating
                .admit(Value::new(origin.clone(), Data::Number(Number::integer(1))))
                .is_ok()
        );
        assert!(
            required
                .admit(Value::new(origin.clone(), Data::Number(Number::integer(1))))
                .is_err()
        );
        assert!(
            required
                .admit(Value::new(origin.clone(), Data::String(Arc::from("uart"))))
                .is_ok()
        );
        assert!(matches!(
            required
                .admit(Value::new(origin.clone(), Data::String(Arc::from("UART"))))
                .expect_err("membership")
                .failure(),
            Failure::Reference(_)
        ));

        let mask = Arc::new(Contract::new(declaration, Type::Mask(Width::W8)));
        let integer = Integer::new(Format::Unsigned(Width::W8), Number::integer(1), origin.clone()).expect("integer");

        assert!(
            mask.admit(Value::new(origin.clone(), Data::Scalar(Scalar::Integer(integer))))
                .is_err()
        );
        assert!(mask.admit(Value::new(origin, Data::Number(Number::integer(1)))).is_ok());
    }

    #[test]
    fn nominal_count_units_require_integral_magnitudes_and_the_same_declaration() {
        let declaration = Origin::new(Id::new("types"), Span::unit(2));
        let origin = Origin::new(Id::new("values"), Span::unit(8));
        let domain = Arc::new(domain::Count::new(declaration.clone(), "page"));
        let other = domain::Count::new(Origin::new(Id::new("other-types"), Span::unit(2)), "page");
        let required = Arc::new(Contract::new(declaration, Type::Count(Arc::clone(&domain))));
        let unit = Unit::lookup("page").expect("intrinsic suffix");
        let half = Number::ratio(1.into(), 2.into(), origin.clone()).expect("fraction");

        let integral = unit
            .apply(&Number::integer(1), Some(&domain), origin.clone())
            .expect("integral quantity");
        let fractional = unit
            .apply(&half, Some(&domain), origin.clone())
            .expect("fractional quantity");
        let different = unit
            .apply(&Number::integer(1), Some(&other), origin.clone())
            .expect("different domain");

        assert!(
            required
                .admit(Value::new(
                    origin.clone(),
                    Data::Scalar(Scalar::Quantity(Box::new(integral)))
                ))
                .is_ok()
        );
        assert!(matches!(
            required
                .admit(Value::new(
                    origin.clone(),
                    Data::Scalar(Scalar::Quantity(Box::new(fractional)))
                ))
                .expect_err("fractional count")
                .failure(),
            Failure::Natural
        ));
        assert!(matches!(
            required
                .admit(Value::new(origin, Data::Scalar(Scalar::Quantity(Box::new(different)))))
                .expect_err("other domain")
                .failure(),
            Failure::Type
        ));
    }
}

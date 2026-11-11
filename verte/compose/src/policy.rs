//! Fixed scalar precedence and array composition for typed configuration values.

use alloc::sync::Arc;

use verte_eval::contract::{
    Checked,
    admission::{Admission, Failure},
};
use verte_ir::value::{Data, Value};

/// Fixed field-level inheritance behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Policy {
    /// Child scalars and arrays win. Records merge recursively.
    Merge,

    /// Parent scalars and arrays win. Records merge recursively.
    Join,

    /// Child scalars win. Arrays concatenate parent then child.
    Admerge,

    /// Parent scalars win. Arrays concatenate parent then child.
    Adjoin,
}

impl Policy {
    /// Compose compatible values and check the result against both sets of requirements.
    #[inline]
    pub fn apply(self, parent: &Checked, child: &Checked) -> Result<Checked, Admission> {
        let conflict = parent
            .contracts()
            .flat_map(|inherited| child.contracts().map(move |incoming| (inherited, incoming)))
            .find(|(inherited, incoming)| !inherited.ty().compatible(&incoming.ty()));

        match conflict {
            Some((inherited, incoming)) => Err(Admission::new(incoming, inherited.origin().clone(), Failure::Conflict)),
            None => {
                let value = self.value(parent.value(), child.value());
                let mut value = child.replace(value)?;

                for inherited in parent.contracts() {
                    value = value.constrain(Arc::clone(inherited))?;
                }

                Ok(value)
            },
        }
    }

    /// Apply precedence recursively after declared-type compatibility is established.
    fn value(self, parent: &Value, child: &Value) -> Value {
        match (parent.data(), child.data(), self) {
            (Data::Record(parent), Data::Record(incoming), _) => {
                let members = parent
                    .clone()
                    .merge(incoming.clone(), |_, parent, child| self.value(&parent, &child));

                Value::new(child.origin().clone(), Data::Record(members))
            },
            (Data::Array(parent), Data::Array(incoming), Self::Admerge | Self::Adjoin) => {
                let mut elements = parent.clone();

                elements.extend(incoming.iter().cloned());

                Value::new(child.origin().clone(), Data::Array(elements))
            },
            (_, _, Self::Merge | Self::Admerge) => child.clone(),
            (_, _, Self::Join | Self::Adjoin) => parent.clone(),
        }
    }
}

impl Default for Policy {
    #[inline]
    fn default() -> Self {
        Self::Merge
    }
}

#[cfg(test)]
mod tests {
    use alloc::{sync::Arc, vec, vec::Vec};

    use aldebaran::prelude::Span;
    use verte_eval::contract::{Contract, admission::Failure};
    use verte_ir::{
        integer::{Format, Width},
        number::Number,
        record::{Member, Record},
        source::{Id, Origin},
        types::{Array, Type},
        value::{Data, Value},
    };

    use super::Policy;

    #[test]
    fn fixed_policies_select_scalars_and_preserve_array_order_and_duplicates() {
        let parent_origin = Origin::new(Id::new("parent"), Span::unit(0));
        let child_origin = Origin::new(Id::new("child"), Span::unit(0));
        let scalar = Arc::new(Contract::new(parent_origin.clone(), Type::String));
        let array = Arc::new(Contract::new(
            parent_origin.clone(),
            Type::Array(Array::new(Type::String, None)),
        ));
        let parent = scalar
            .admit(Value::new(parent_origin.clone(), Data::String(Arc::from("parent"))))
            .expect("parent scalar");
        let child = scalar
            .admit(Value::new(child_origin.clone(), Data::String(Arc::from("child"))))
            .expect("child scalar");
        let old_array = array
            .admit(Value::new(
                parent_origin.clone(),
                Data::Array(vec![Value::new(parent_origin.clone(), Data::String(Arc::from("same")))]),
            ))
            .expect("parent array");
        let new_array = array
            .admit(Value::new(
                child_origin.clone(),
                Data::Array(vec![
                    Value::new(child_origin.clone(), Data::String(Arc::from("same"))),
                    Value::new(child_origin.clone(), Data::String(Arc::from("new"))),
                ]),
            ))
            .expect("child array");

        for (policy, winner, count) in [
            (Policy::Merge, "child", 2),
            (Policy::Join, "parent", 1),
            (Policy::Admerge, "child", 3),
            (Policy::Adjoin, "parent", 3),
        ] {
            let result = policy.apply(&parent, &child).expect("scalar composition");
            let array = policy.apply(&old_array, &new_array).expect("array composition");

            assert_eq!(result.value().data(), &Data::String(Arc::from(winner)));
            assert_eq!(result.contracts().len(), 2);

            match array.value().data() {
                Data::Array(values) => {
                    assert_eq!(values.len(), count);

                    if count == 3 {
                        assert_eq!(values.first().expect("parent element").origin(), &parent_origin);
                        assert_eq!(values.get(1).expect("child duplicate").origin(), &child_origin);
                        assert_eq!(
                            values.get(2).expect("child new element").data(),
                            &Data::String(Arc::from("new"))
                        );
                    }
                },
                _ => unreachable!("array composition"),
            }
        }
    }

    #[test]
    fn precedence_never_hides_incompatible_underlying_types() {
        let parent_origin = Origin::new(Id::new("parent"), Span::unit(0));
        let child_origin = Origin::new(Id::new("child"), Span::unit(0));
        let old_type = Arc::new(Contract::new(
            parent_origin.clone(),
            Type::Integer(Format::Unsigned(Width::W8)),
        ));
        let new_type = Arc::new(Contract::new(
            child_origin.clone(),
            Type::Integer(Format::Unsigned(Width::W16)),
        ));
        let parent = old_type
            .admit(Value::new(parent_origin.clone(), Data::Number(Number::integer(1))))
            .expect("parent");
        let child = new_type
            .admit(Value::new(child_origin.clone(), Data::Number(Number::integer(1))))
            .expect("child");

        for policy in [Policy::Merge, Policy::Join, Policy::Admerge, Policy::Adjoin] {
            let error = policy.apply(&parent, &child).expect_err("different declared widths");

            assert!(matches!(error.failure(), Failure::Conflict));
            assert_eq!(error.origin(), &parent_origin);
            assert_eq!(error.declaration().origin(), &child_origin);
        }
    }

    #[test]
    fn fixed_length_contracts_survive_lists_and_multiple_generations() {
        let parent_origin = Origin::new(Id::new("parent"), Span::unit(0));
        let child_origin = Origin::new(Id::new("child"), Span::unit(0));
        let fixed = Arc::new(Contract::new(
            parent_origin.clone(),
            Type::Array(Array::new(Type::Bool, Some(1_u8.into()))),
        ));
        let list = Arc::new(Contract::new(
            child_origin.clone(),
            Type::Array(Array::new(Type::Bool, None)),
        ));
        let parent = fixed
            .admit(Value::new(
                parent_origin.clone(),
                Data::Array(vec![Value::new(parent_origin, Data::Bool(true))]),
            ))
            .expect("fixed parent");
        let child = list
            .admit(Value::new(
                child_origin.clone(),
                Data::Array(vec![Value::new(child_origin.clone(), Data::Bool(false))]),
            ))
            .expect("child list");
        let merged = Policy::Merge.apply(&parent, &child).expect("same effective length");
        let grandchild = list
            .admit(Value::new(child_origin.clone(), Data::Array(Vec::new())))
            .expect("empty list");

        assert!(Policy::Admerge.apply(&parent, &child).is_err());
        assert!(Policy::Adjoin.apply(&parent, &child).is_err());
        assert!(Policy::Merge.apply(&merged, &grandchild).is_err());
        assert_eq!(merged.contracts().len(), 2);
    }

    #[test]
    fn records_merge_recursively_and_closed_shapes_remain_constraints() {
        let parent_origin = Origin::new(Id::new("parent"), Span::unit(0));
        let child_origin = Origin::new(Id::new("child"), Span::unit(0));
        let mut members = Record::new();

        members
            .insert(Member::new("value", parent_origin.clone(), Type::Bool))
            .expect("type member");

        let required = Arc::new(Contract::new(parent_origin.clone(), Type::Record(members.clone())));
        let mut old_members = Record::new();
        let mut new_members = Record::new();

        old_members
            .insert(Member::new(
                "value",
                parent_origin.clone(),
                Value::new(parent_origin.clone(), Data::Bool(true)),
            ))
            .expect("parent member");
        new_members
            .insert(Member::new(
                "value",
                child_origin.clone(),
                Value::new(child_origin.clone(), Data::Bool(false)),
            ))
            .expect("child member");

        let parent = required
            .admit(Value::new(parent_origin, Data::Record(old_members)))
            .expect("parent record");
        let child = required
            .admit(Value::new(child_origin.clone(), Data::Record(new_members.clone())))
            .expect("child record");

        for (policy, expected) in [
            (Policy::Merge, false),
            (Policy::Join, true),
            (Policy::Admerge, false),
            (Policy::Adjoin, true),
        ] {
            let result = policy.apply(&parent, &child).expect("record composition");

            match result.value().data() {
                Data::Record(members) => assert_eq!(
                    members.get("value").expect("member").value().data(),
                    &Data::Bool(expected)
                ),
                _ => unreachable!("record value"),
            }
        }

        members
            .insert(Member::new("extra", child_origin.clone(), Type::Bool))
            .expect("extra type member");
        new_members
            .insert(Member::new(
                "extra",
                child_origin.clone(),
                Value::new(child_origin.clone(), Data::Bool(true)),
            ))
            .expect("extra member");

        let expanded = Arc::new(Contract::new(child_origin.clone(), Type::Record(members)))
            .admit(Value::new(child_origin, Data::Record(new_members)))
            .expect("expanded child");

        assert!(
            matches!(Policy::Merge.apply(&parent, &expanded).expect_err("parent closed shape").failure(), Failure::Unexpected(name) if name.as_ref() == "extra")
        );
    }
}

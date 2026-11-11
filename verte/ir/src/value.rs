//! Owned evaluated candidates with source locations for every nested value.
//!
//! These values preserve scalar representations and nominal identities. Admission
//! against a declared type and its refinements belongs to the evaluation layer,
//! which carries the resulting contracts into configuration composition.

use alloc::{boxed::Box, sync::Arc, vec::Vec};

use crate::{
    integer::mask::Mask, number::Number, range::Range, record::Record, resource::reference::Reference, scalar::Scalar,
    source::Origin,
};

/// An evaluated candidate whose source survives provider and syntax-tree lifetimes.
#[derive(Debug, Clone, PartialEq, Eq)]
// NOTE(invariant): None.
pub struct Value {
    /// Source-qualified occurrence producing this candidate.
    origin: Origin,

    /// Mathematical or already represented semantic data.
    data: Data,
}

impl Value {
    /// Associate semantic data with the expression that produced it.
    #[inline]
    pub const fn new(origin: Origin, data: Data) -> Self {
        Self { origin, data }
    }

    /// Retrieve the source identity and original expression coordinates.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        let Self { origin, .. } = self;

        origin
    }

    /// Inspect the candidate before or after declared-type admission.
    #[inline]
    pub const fn data(&self) -> &Data {
        let Self { data, .. } = self;

        data
    }

    /// Transfer semantic data into an operation that tracks its own provenance.
    #[inline]
    pub fn into_data(self) -> Data {
        let Self { data, .. } = self;

        data
    }
}

/// Candidate values, including exact numbers awaiting contextual representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Data {
    /// A literal or exact expression before a declared scalar format is required.
    Number(Number),

    /// Boolean truth.
    Bool(bool),

    /// A decoded UTF-8 string, also used by closed string enumerations.
    String(Arc<str>),

    /// An ordered scalar with its dimensions, format, or nominal identity.
    Scalar(Scalar),

    /// An admitted string resource reference.
    Reference(Reference),

    /// A nominal unsigned mask.
    Mask(Mask),

    /// An interval with typed coordinates and an explicit endpoint mode.
    Range(Box<Range>),

    /// Independently source-qualified sequence elements.
    Array(Vec<Value>),

    /// Unique decoded members in declaration order.
    Record(Record<Value>),
}

#[cfg(test)]
mod tests {
    use alloc::{string::String, sync::Arc, vec};

    use aldebaran::prelude::Span;

    use super::{Data, Value};
    use crate::{
        record::{Member, Record},
        source::{Id, Origin},
    };

    #[test]
    fn nested_values_keep_each_source_identity_after_input_buffers_are_dropped() {
        let parent = String::from("host:parent");
        let child = String::from("host:child");
        let member = String::from("decoded member");
        let content = String::from("decoded value");
        let parent_origin = Origin::new(Id::new(parent.as_str()), Span::unit(4));
        let child_origin = Origin::new(Id::new(child.as_str()), Span::unit(4));
        let element = Value::new(parent_origin.clone(), Data::String(Arc::from(content.as_str())));
        let array = Value::new(child_origin.clone(), Data::Array(vec![element]));
        let mut record = Record::new();

        record
            .insert(Member::new(member.as_str(), child_origin.clone(), array))
            .expect("member");

        let value = Value::new(child_origin.clone(), Data::Record(record));

        drop(parent);
        drop(child);
        drop(member);
        drop(content);

        assert_eq!(value.origin(), &child_origin);

        match value.data() {
            Data::Record(record) => {
                let member = record.get("decoded member").expect("owned key");

                assert_eq!(member.origin(), &child_origin);

                match member.value().data() {
                    Data::Array(elements) => {
                        let element = elements.first().expect("element");

                        assert_eq!(element.origin(), &parent_origin);
                        assert_eq!(element.data(), &Data::String(Arc::from("decoded value")));
                    },
                    _ => unreachable!("array value"),
                }
            },
            _ => unreachable!("record value"),
        }
    }
}

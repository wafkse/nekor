//! Closed nonempty string enumerations with member declaration locations.

use aldebaran::{
    prelude::Report,
    report::annotated::{InlineAnnotations, label::Label},
};
use fack::prelude::Error;

use crate::{record::Record, source::Origin};

/// A nonempty set of unique decoded strings in declaration order.
#[derive(Debug, Clone, PartialEq, Eq)]
// NOTE(invariant): The record contains at least one member.
pub struct Enumeration(Record<()>);

impl Enumeration {
    /// Admit a closed set after decoded-name uniqueness has been established.
    #[inline]
    pub fn new(members: Record<()>, origin: Origin) -> Result<Self, Empty> {
        if members.is_empty() {
            let labels =
                InlineAnnotations::new(Label::new("declare at least one enumeration member", origin.span()), []);

            Err(Empty { origin, labels })
        } else {
            Ok(Self(members))
        }
    }

    /// Test membership using exact decoded values.
    #[inline]
    pub fn contains(&self, value: &str) -> bool {
        let Self(members) = self;

        members.get(value).is_some()
    }

    /// Inspect members and their source locations in declaration order.
    #[inline]
    pub const fn members(&self) -> &Record<()> {
        let Self(members) = self;

        members
    }
}

/// An enumeration has no admitted string values.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("an enumeration must declare at least one member")]
#[report(title = "empty enumeration", annotations = labels)]
// NOTE(invariant): The label uses the coordinates qualified by `origin`.
pub struct Empty {
    /// Source-qualified enumeration declaration.
    origin: Origin,

    /// Location of the empty enumeration body.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

impl Empty {
    /// Retrieve the source identity qualifying this report's annotation.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        let Self { origin, .. } = self;

        origin
    }
}

#[cfg(test)]
mod tests {
    use aldebaran::prelude::Span;

    use super::Enumeration;
    use crate::{
        record::{Member, Record},
        source::{Id, Origin},
    };

    #[test]
    fn enumeration_membership_is_exact_and_the_empty_set_is_rejected() {
        let origin = Origin::new(Id::new("enum"), Span::unit(0));
        let mut members = Record::new();

        assert!(Enumeration::new(Record::new(), origin.clone()).is_err());

        members.insert(Member::new("edge", origin.clone(), ())).expect("member");
        members
            .insert(Member::new("level", origin.clone(), ()))
            .expect("member");

        let enumeration = Enumeration::new(members, origin).expect("nonempty");

        assert!(enumeration.contains("edge"));
        assert!(!enumeration.contains("Edge"));
        assert_eq!(enumeration.members().iter().next().expect("first").name(), "edge");
    }
}

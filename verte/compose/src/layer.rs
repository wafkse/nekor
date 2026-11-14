//! Local table declarations carry policies only until inheritance is applied.

use alloc::boxed::Box;

use aldebaran::{
    prelude::Report,
    report::annotated::{InlineAnnotations, label::Label},
};
use fack::prelude::Error;
use verte_eval::contract::{Checked, admission::Admission};
use verte_ir::{
    record::{Member, Record, duplicate::Duplicate},
    source::Origin,
};

use crate::{
    policy::Policy,
    tree::{Node, Tree},
};

/// A local configuration layer with independently checked field initializers.
#[derive(Debug, Clone)]
// NOTE(invariant): None.
pub struct Layer {
    /// Source-qualified configuration or table occurrence.
    origin: Origin,

    /// Unique declarations in source order.
    members: Record<Declaration>,
}

impl Layer {
    /// Start a table scope before inserting its source-ordered declarations.
    #[inline]
    pub const fn new(origin: Origin) -> Self {
        Self {
            origin,
            members: Record::new(),
        }
    }

    /// Insert a uniquely named table or checked field declaration.
    #[inline]
    pub fn insert(&mut self, member: Member<Declaration>) -> Result<(), Duplicate> {
        let Self { members, .. } = self;

        members.insert(member)
    }

    /// Retrieve the local configuration or table's source-qualified occurrence.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        let Self { origin, .. } = self;

        origin
    }

    /// Consume local policies while building an effective typed tree.
    #[inline]
    pub fn inherit(self, parent: Option<&Tree>, inheritance: Inheritance) -> Result<Tree, Composition> {
        let Self { origin, members } = self;

        match (parent, inheritance) {
            (Some(parent), Inheritance::Forbid(directive)) => Err(Composition::Forbidden(Forbidden::new(
                directive,
                parent.origin().clone(),
            ))),
            (parent, inheritance) => {
                let policy = match inheritance {
                    Inheritance::Allow(policy) => policy,
                    Inheritance::Forbid(_) => Policy::Merge,
                };
                let members = members.try_map(|name, _, incoming| {
                    let inherited = parent.and_then(|parent| parent.members().get(name)).map(Member::value);

                    incoming.inherit(inherited, policy)
                })?;
                let members = match parent {
                    Some(parent) => parent.members().clone().merge(members, |_, _, incoming| incoming),
                    None => members,
                };

                Ok(Tree::new(origin, members))
            },
        }
    }
}

/// Document-level permission to compose with an explicit parent.
#[derive(Debug, Clone)]
pub enum Inheritance {
    /// Compose under the selected fixed policy.
    Allow(Policy),

    /// Reject any parent and report the failure at the directive location.
    Forbid(Origin),
}

impl Default for Inheritance {
    #[inline]
    fn default() -> Self {
        Self::Allow(Policy::Merge)
    }
}

/// A local table or a field initializer with an optional incoming policy override.
#[derive(Debug, Clone)]
pub enum Declaration {
    /// A nested table's independent local declarations.
    Table(Layer),

    /// A checked field and its document-local inheritance behavior.
    Field(Field),
}

impl Declaration {
    /// Retrieve the table or initializer's source-qualified occurrence.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        match self {
            Self::Table(layer) => layer.origin(),
            Self::Field(field) => field.value().value().origin(),
        }
    }

    /// Apply inherited structure while consuming any field-local policy.
    fn inherit(self, parent: Option<&Node>, policy: Policy) -> Result<Node, Composition> {
        match (self, parent) {
            (Self::Table(layer), Some(Node::Table(parent))) => {
                layer.inherit(Some(parent), Inheritance::Allow(policy)).map(Node::Table)
            },
            (Self::Table(layer), None) => layer.inherit(None, Inheritance::Allow(policy)).map(Node::Table),
            (Self::Field(field), Some(Node::Field(parent))) => {
                let Field {
                    value,
                    policy: incoming,
                } = field;

                incoming
                    .unwrap_or(policy)
                    .apply(parent, &value)
                    .map(Node::Field)
                    .map_err(Composition::Admission)
            },
            (Self::Field(field), None) => {
                let Field { value, .. } = field;

                Ok(Node::Field(value))
            },
            (incoming, Some(parent)) => Err(Composition::Structure(Structure::new(
                incoming.origin().clone(),
                parent.origin().clone(),
            ))),
        }
    }
}

/// A checked local initializer and its optional incoming field policy.
#[derive(Debug, Clone)]
// NOTE(invariant): None.
pub struct Field {
    /// Initializer satisfying its own declared requirements before inheritance.
    value: Checked,

    /// Override applied only while this field participates as an incoming declaration.
    policy: Option<Policy>,
}

impl Field {
    /// Associate an admitted initializer with its optional local policy.
    #[inline]
    pub const fn new(value: Checked, policy: Option<Policy>) -> Self {
        Self { value, policy }
    }

    /// Inspect the independently checked initializer.
    #[inline]
    pub const fn value(&self) -> &Checked {
        let Self { value, .. } = self;

        value
    }
}

/// A typed inheritance operation failed structural, value, or parent-permission checks.
///
/// Render the concrete variant against its own source. Related type declarations
/// and inherited inputs have separate reports, each rendered against the source
/// text to which its byte coordinates belong.
#[derive(Debug, Clone, Error)]
#[error(inline)]
pub enum Composition {
    /// The effective value violates an applicable declared type requirement.
    #[error(transparent(0))]
    Admission(Admission),

    /// A table and field occupy the same inherited name.
    #[error(transparent(0))]
    Structure(Structure),

    /// A parent was supplied despite an explicit `forbid` directive.
    #[error(transparent(0))]
    Forbidden(Forbidden),
}

/// A table scope conflicts with an inherited field or the reverse.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("incompatible inherited table and field")]
#[report(title = "incompatible inherited structure", annotations = labels)]
// NOTE(invariant): Labels belong to `origin`, independently of `parent`'s canonical source.
pub struct Structure {
    /// Incoming declaration's source-qualified occurrence.
    origin: Origin,

    /// Independently qualified inherited declaration.
    parent: Box<Parent>,

    /// Primary incoming declaration location.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

impl Structure {
    /// Report the conflicting table and field declarations.
    #[inline]
    fn new(origin: Origin, parent: Origin) -> Self {
        let labels = InlineAnnotations::new(
            Label::new(
                "a table and a typed field cannot occupy the same inherited name",
                origin.span(),
            ),
            [],
        );

        let parent = Box::new(Parent::new(parent));

        Self { origin, parent, labels }
    }

    /// Retrieve the source qualifying this report's annotation.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        let Self { origin, .. } = self;

        origin
    }

    /// Retrieve the independently qualified inherited declaration location.
    #[inline]
    pub const fn parent(&self) -> &Origin {
        let Self { parent, .. } = self;

        parent.origin()
    }

    /// Borrow the inherited input report for attachment to its own source text.
    #[inline]
    pub const fn related(&self) -> &Parent {
        let Self { parent, .. } = self;

        parent
    }
}

/// An explicit parent was supplied to a configuration that forbids inheritance.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("this configuration forbids parent inheritance")]
#[report(title = "parent inheritance is forbidden", annotations = labels)]
// NOTE(invariant): Labels belong to `origin`, independently of `parent`'s canonical source.
pub struct Forbidden {
    /// Source-qualified `forbid` directive.
    origin: Origin,

    /// Independently qualified explicit parent configuration.
    parent: Box<Parent>,

    /// Primary directive location.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

impl Forbidden {
    /// Report the prohibiting directive and the explicit parent location.
    #[inline]
    fn new(origin: Origin, parent: Origin) -> Self {
        let labels = InlineAnnotations::new(
            Label::new(
                "this directive rejects an explicit parent, including an empty configuration",
                origin.span(),
            ),
            [],
        );

        let parent = Box::new(Parent::new(parent));

        Self { origin, parent, labels }
    }

    /// Retrieve the source-qualified directive location.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        let Self { origin, .. } = self;

        origin
    }

    /// Retrieve the independently qualified explicit parent location.
    #[inline]
    pub const fn parent(&self) -> &Origin {
        let Self { parent, .. } = self;

        parent.origin()
    }

    /// Borrow the explicit parent's report for attachment to its own source text.
    #[inline]
    pub const fn related(&self) -> &Parent {
        let Self { parent, .. } = self;

        parent
    }
}

/// The independently source-qualified inherited input in a composition failure.
#[derive(Debug, Clone, Report)]
#[report(title = "inherited input", annotations = labels)]
// NOTE(invariant): The label uses the source coordinates qualified by `origin`.
pub struct Parent {
    /// Canonical source and original inherited occurrence.
    origin: Origin,

    /// Parent input location in its own source.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

impl Parent {
    /// Create a parent input report for attachment to its source text.
    #[inline]
    const fn new(origin: Origin) -> Self {
        let labels = InlineAnnotations::new(Label::new("the inherited input originates here", origin.span()), []);

        Self { origin, labels }
    }

    /// Retrieve the canonical source qualifying this report's annotation.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        let Self { origin, .. } = self;

        origin
    }
}

#[cfg(test)]
mod tests {
    use alloc::{sync::Arc, vec::Vec};

    use aldebaran::prelude::Span;
    use verte_eval::contract::Contract;
    use verte_ir::{
        record::{Member, Record},
        source::{Id, Origin},
        types::Type,
        value::{Data, Value},
    };

    use super::{Composition, Declaration, Field, Inheritance, Layer};
    use crate::{
        policy::Policy,
        tree::{Node, Tree},
    };

    #[test]
    fn inheritance_keeps_parent_positions_and_appends_new_child_names_in_source_order() {
        let parent_origin = Origin::new(Id::new("parent"), Span::unit(0));
        let child_origin = Origin::new(Id::new("child"), Span::unit(0));
        let required = Arc::new(Contract::new(parent_origin.clone(), Type::Bool));
        let mut parent = Layer::new(parent_origin.clone());
        let mut child = Layer::new(child_origin.clone());

        for name in ["z", "a"] {
            let value = required
                .admit(Value::new(parent_origin.clone(), Data::Bool(true)))
                .expect("parent value");

            parent
                .insert(Member::new(
                    name,
                    parent_origin.clone(),
                    Declaration::Field(Field::new(value, None)),
                ))
                .expect("parent field");
        }

        for name in ["a", "m", "b"] {
            let value = required
                .admit(Value::new(child_origin.clone(), Data::Bool(false)))
                .expect("child value");

            child
                .insert(Member::new(
                    name,
                    child_origin.clone(),
                    Declaration::Field(Field::new(value, None)),
                ))
                .expect("child field");
        }

        let parent = parent
            .inherit(None, Inheritance::default())
            .expect("root configuration");
        let effective = child
            .inherit(Some(&parent), Inheritance::default())
            .expect("child configuration");

        assert_eq!(
            effective.members().iter().map(Member::name).collect::<Vec<_>>(),
            ["z", "a", "m", "b"]
        );
        assert_eq!(effective.origin(), &child_origin);

        match effective.members().get("a").expect("overridden field").value() {
            Node::Field(value) => {
                assert_eq!(value.value().data(), &Data::Bool(false));
                assert_eq!(value.value().origin(), &child_origin);
                assert_eq!(value.contracts().len(), 2);
            },
            Node::Table(_) => unreachable!("field"),
        }
    }

    #[test]
    fn incoming_field_policies_are_consumed_and_do_not_stick_to_later_generations() {
        let origin = Origin::new(Id::new("policies"), Span::unit(0));
        let required = Arc::new(Contract::new(origin.clone(), Type::Bool));
        let mut parent = Layer::new(origin.clone());
        let mut child = Layer::new(origin.clone());
        let mut grandchild = Layer::new(origin.clone());

        for (layer, value, policy) in [
            (&mut parent, true, None),
            (&mut child, false, Some(Policy::Join)),
            (&mut grandchild, false, None),
        ] {
            let value = required
                .admit(Value::new(origin.clone(), Data::Bool(value)))
                .expect("initializer");

            layer
                .insert(Member::new(
                    "enabled",
                    origin.clone(),
                    Declaration::Field(Field::new(value, policy)),
                ))
                .expect("field");
        }

        let parent = parent.inherit(None, Inheritance::default()).expect("root");
        let child = child
            .inherit(Some(&parent), Inheritance::default())
            .expect("local join");
        let grandchild = grandchild
            .inherit(Some(&child), Inheritance::default())
            .expect("later default merge");

        match (
            child.members().get("enabled").expect("child field").value(),
            grandchild.members().get("enabled").expect("grandchild field").value(),
        ) {
            (Node::Field(child), Node::Field(grandchild)) => {
                assert_eq!(child.value().data(), &Data::Bool(true));
                assert_eq!(grandchild.value().data(), &Data::Bool(false));
                assert_eq!(grandchild.contracts().len(), 3);
            },
            _ => unreachable!("fields"),
        }
    }

    #[test]
    fn forbid_rejects_an_explicit_empty_parent_but_allows_no_parent() {
        let origin = Origin::new(Id::new("child"), Span::unit(4));
        let parent_origin = Origin::new(Id::new("parent"), Span::unit(0));
        let parent = Tree::new(parent_origin.clone(), Record::new());
        let child = Layer::new(origin.clone());
        let error = child
            .clone()
            .inherit(Some(&parent), Inheritance::Forbid(origin.clone()))
            .expect_err("explicit empty parent");

        match error {
            Composition::Forbidden(report) => {
                assert_eq!(report.origin(), &origin);
                assert_eq!(report.parent(), &parent_origin);
                assert_eq!(report.related().origin(), &parent_origin);
            },
            _ => unreachable!("forbidden parent"),
        }

        assert!(
            child
                .inherit(None, Inheritance::Forbid(origin))
                .expect("no parent")
                .members()
                .is_empty()
        );
    }

    #[test]
    fn table_inheritance_is_recursive_and_rejects_table_field_collisions() {
        let parent_origin = Origin::new(Id::new("parent"), Span::unit(0));
        let child_origin = Origin::new(Id::new("child"), Span::unit(0));
        let required = Arc::new(Contract::new(parent_origin.clone(), Type::Bool));
        let value = required
            .admit(Value::new(parent_origin.clone(), Data::Bool(true)))
            .expect("field");
        let mut old_table = Layer::new(parent_origin.clone());
        let mut new_table = Layer::new(child_origin.clone());

        old_table
            .insert(Member::new(
                "retained",
                parent_origin.clone(),
                Declaration::Field(Field::new(value.clone(), None)),
            ))
            .expect("parent member");
        new_table
            .insert(Member::new(
                "added",
                child_origin.clone(),
                Declaration::Field(Field::new(value.clone(), None)),
            ))
            .expect("child member");

        let mut parent = Layer::new(parent_origin.clone());
        let mut child = Layer::new(child_origin.clone());
        let mut invalid = Layer::new(child_origin.clone());

        parent
            .insert(Member::new("table", parent_origin, Declaration::Table(old_table)))
            .expect("parent table");
        child
            .insert(Member::new(
                "table",
                child_origin.clone(),
                Declaration::Table(new_table),
            ))
            .expect("child table");
        invalid
            .insert(Member::new(
                "table",
                child_origin,
                Declaration::Field(Field::new(value, None)),
            ))
            .expect("conflicting field");

        let parent = parent.inherit(None, Inheritance::default()).expect("parent");
        let effective = child
            .inherit(Some(&parent), Inheritance::default())
            .expect("recursive composition");

        match effective.members().get("table").expect("table").value() {
            Node::Table(table) => assert_eq!(
                table.members().iter().map(Member::name).collect::<Vec<_>>(),
                ["retained", "added"]
            ),
            Node::Field(_) => unreachable!("table"),
        }

        assert!(matches!(
            invalid.inherit(Some(&parent), Inheritance::default()),
            Err(Composition::Structure(_))
        ));
    }
}

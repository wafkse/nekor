//! Effective configuration nodes expose checked values in declaration order.

use verte_eval::contract::Checked;
use verte_ir::{record::Record, source::Origin};

/// An effective configuration whose fields carry their type requirements.
#[derive(Debug, Clone)]
// NOTE(invariant): None.
pub struct Tree {
    /// Source-qualified local configuration that produced this effective tree.
    origin: Origin,

    /// Effective members in inherited position and then new declaration order.
    members: Record<Node>,
}

impl Tree {
    /// Establish an effective tree from independently checked nodes.
    #[inline]
    pub const fn new(origin: Origin, members: Record<Node>) -> Self {
        Self { origin, members }
    }

    /// Retrieve the source-qualified configuration occurrence.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        let Self { origin, .. } = self;

        origin
    }

    /// Inspect effective members by name or in declaration order.
    #[inline]
    pub const fn members(&self) -> &Record<Node> {
        let Self { members, .. } = self;

        members
    }
}

/// A table scope or a field checked against its structural requirements.
#[derive(Debug, Clone)]
pub enum Node {
    /// A nested table scope.
    Table(Tree),

    /// An independently admitted field value.
    Field(Checked),
}

impl Node {
    /// Retrieve the table or effective value's source-qualified occurrence.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        match self {
            Self::Table(tree) => tree.origin(),
            Self::Field(value) => value.value().origin(),
        }
    }
}

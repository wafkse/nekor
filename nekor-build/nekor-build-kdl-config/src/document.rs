//! Typed KDL configuration documents.

use indexmap::IndexMap;

use crate::{
    error::LoadError,
    merge::{Merge, MergeError},
    name::Name,
    node::Node,
    overlay::{Overlay, OverlayError},
    parse::document as parse_document,
    source::Source,
};

/// A typed semantic configuration document.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Document {
    // NOTE(invariant): Every semantic path has at most one node. Parsing and
    // merging reject conflicting terminal definitions before construction.
    /// The top-level nodes keyed by their semantic names.
    fields: IndexMap<Name, Node>,
}

impl Document {
    /// Parse one KDL 2 source into a typed semantic document.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid KDL syntax, unsupported node shapes, or
    /// invalid reserved representation annotations.
    #[inline]
    pub fn parse(input: &str, source: &Source) -> Result<Self, LoadError> {
        parse_document(input, source)
    }

    /// Determine a top-level node by name.
    #[inline]
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Node> {
        let &Self { ref fields } = self;

        fields.get(name)
    }

    /// Iterate over top-level nodes in source order.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&Name, &Node)> {
        let &Self { ref fields } = self;

        fields.iter()
    }

    /// Construct a document from a map whose semantic paths are unique.
    #[inline]
    pub(crate) const fn from_fields(fields: IndexMap<Name, Node>) -> Self {
        Self { fields }
    }

    /// Determine the backing map for Serde projection.
    #[inline]
    pub(crate) const fn fields(&self) -> &IndexMap<Name, Node> {
        let &Self { ref fields } = self;

        fields
    }
}

impl Merge for Document {
    type Error = MergeError;

    fn merge(self, incoming: Self) -> Result<Self, Self::Error> {
        let Self { mut fields } = self;
        let Self { fields: incoming } = incoming;

        for (name, node) in incoming {
            if let Some((index, existing_name, existing)) = fields.shift_remove_full(&name) {
                let merged = existing.merge(node).map_err(|error| error.prefixed(name))?;
                _ = fields.shift_insert(index, existing_name, merged);
            } else {
                _ = fields.insert(name, node);
            }
        }

        Ok(Self { fields })
    }
}

impl Overlay for Document {
    type Error = OverlayError;

    fn overlay(self, derived: Self) -> Result<Self, Self::Error> {
        let Self { mut fields } = self;
        let Self { fields: derived } = derived;

        for (name, node) in derived {
            if let Some((index, inherited_name, inherited)) = fields.shift_remove_full(&name) {
                let resolved = inherited.overlay(node).map_err(|error| error.prefixed(name))?;
                _ = fields.shift_insert(index, inherited_name, resolved);
            } else {
                _ = fields.insert(name, node);
            }
        }

        Ok(Self { fields })
    }
}

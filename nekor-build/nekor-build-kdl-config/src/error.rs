//! Errors produced while parsing typed KDL configuration.

use fack::prelude::Error;

use crate::{
    merge::MergeError,
    name::Name,
    scalar::{FloatType, IntegerType, ScalarKind, TypeAnnotation},
    source::{Origin, Source},
};

/// The unsupported structural form observed for a KDL data node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeShape {
    /// The node contains one or more properties.
    Properties,
    /// The node contains both positional values and child nodes.
    ValuesAndChildren,
    /// The node contains more than one positional value.
    MultipleValues,
    /// A child block mixes list entries with named object entries.
    MixedListChildren,
    /// A dashed list entry does not contain exactly one scalar value.
    InvalidListEntry,
}

/// A KDL syntax parsing failure with source identity.
#[derive(Debug, Error)]
#[error("failed to parse KDL source `{source}` with {error}")]
#[error(source(error))]
pub struct ParseError {
    /// The source that failed to parse.
    pub source: Source,
    /// The underlying KDL parser error.
    pub error: kdl::KdlError,
}

/// A KDL document shape failure under the typed configuration profile.
#[derive(Debug, Error)]
pub enum ProfileError {
    /// A data node uses an unsupported structural form.
    #[error("node `{name}` at `{origin}` has unsupported shape `{shape:?}`")]
    UnsupportedNodeShape {
        /// The node name.
        name: Name,
        /// The unsupported structural form.
        shape: NodeShape,
        /// The source location of the node.
        origin: Origin,
    },
}

/// A reserved representation annotation failure.
#[derive(Debug, Error)]
pub enum TypeError {
    /// A reserved annotation was applied to the wrong scalar kind.
    #[error("annotation `{annotation:?}` at `{origin}` requires `{expected:?}` but found `{actual:?}`")]
    ScalarKind {
        /// The classified representation annotation.
        annotation: TypeAnnotation,
        /// The required scalar kind.
        expected: ScalarKind,
        /// The observed scalar kind.
        actual: ScalarKind,
        /// The source location of the scalar.
        origin: Origin,
    },
    /// An integer does not fit the requested fixed-width representation.
    #[error("integer `{value}` at `{origin}` cannot be represented as `{representation:?}`")]
    IntegerRange {
        /// The requested integer representation.
        representation: IntegerType,
        /// The parsed KDL integer value.
        value: i128,
        /// The source location of the scalar.
        origin: Origin,
    },
    /// A floating point number overflows the requested representation.
    #[error(
        "floating point value `{value}` at `{origin}` cannot be represented as \
         `{representation:?}`"
    )]
    FloatRange {
        /// The requested floating point representation.
        representation: FloatType,
        /// The parsed KDL floating point value.
        value: f64,
        /// The source location of the scalar.
        origin: Origin,
    },
}

/// A failure produced while loading one typed KDL source.
#[derive(Debug, Error)]
pub enum LoadError {
    /// KDL syntax could not be parsed.
    #[error(transparent(0))]
    Parse(ParseError),
    /// Parsed KDL violates the typed document profile.
    #[error(transparent(0))]
    Profile(ProfileError),
    /// A reserved representation annotation is invalid.
    #[error(transparent(0))]
    Type(TypeError),
    /// Repeated nodes in one source conflict under strict merging.
    #[error(transparent(0))]
    Merge(Box<MergeError>),
}

impl From<ParseError> for LoadError {
    fn from(error: ParseError) -> Self {
        Self::Parse(error)
    }
}

impl From<ProfileError> for LoadError {
    fn from(error: ProfileError) -> Self {
        Self::Profile(error)
    }
}

impl From<TypeError> for LoadError {
    fn from(error: TypeError) -> Self {
        Self::Type(error)
    }
}

impl From<MergeError> for LoadError {
    fn from(error: MergeError) -> Self {
        Self::Merge(Box::new(error))
    }
}

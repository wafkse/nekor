//! Strict merging for typed configuration values.

use core::error::Error as CoreError;

use fack::prelude::Error;

use crate::{
    annotation::NodeAnnotation,
    name::{Name, Path},
    node::StructureKind,
    source::Origin,
};

/// A value that can absorb a same-precedence input without erasing conflicts.
pub trait Merge<Rhs = Self>: Sized {
    /// The failure produced when the values cannot be merged.
    type Error: CoreError;

    /// Merge `incoming` into this value.
    ///
    /// # Errors
    ///
    /// Returns an error when both values define incompatible information.
    fn merge(self, incoming: Rhs) -> Result<Self, Self::Error>;
}

/// A failure produced by strict configuration merging.
#[derive(Debug, Error)]
pub enum MergeError {
    /// Both inputs define a terminal value at the same semantic path.
    #[error("duplicate value at `{path}` between `{existing}` and `{incoming}`")]
    DuplicateValue {
        /// The semantic path that is defined more than once.
        path: Path,
        /// The source location of the existing value.
        existing: Origin,
        /// The source location of the incoming value.
        incoming: Origin,
    },
    /// Both inputs assign different node annotations to the same path.
    #[error("node annotation `{actual}` conflicts with `{expected}` at `{path}`")]
    NodeAnnotation {
        /// The semantic path whose node annotations conflict.
        path: Path,
        /// The annotation already established at the path.
        expected: NodeAnnotation,
        /// The annotation supplied by the incoming value.
        actual: NodeAnnotation,
        /// The source location of the existing node.
        existing: Origin,
        /// The source location of the incoming node.
        incoming: Origin,
    },
    /// Both inputs use incompatible structural forms at the same path.
    #[error("structure `{actual:?}` conflicts with `{expected:?}` at `{path}`")]
    Structure {
        /// The semantic path whose structures conflict.
        path: Path,
        /// The structural form already established at the path.
        expected: StructureKind,
        /// The structural form supplied by the incoming value.
        actual: StructureKind,
        /// The source location of the existing node.
        existing: Origin,
        /// The source location of the incoming node.
        incoming: Origin,
    },
}

impl MergeError {
    /// Prefix the failing semantic path with one containing node name.
    #[must_use]
    pub fn prefixed(self, name: Name) -> Self {
        match self {
            Self::DuplicateValue {
                path,
                existing,
                incoming,
            } => Self::DuplicateValue {
                path: path.prefixed(name),
                existing,
                incoming,
            },
            Self::NodeAnnotation {
                path,
                expected,
                actual,
                existing,
                incoming,
            } => Self::NodeAnnotation {
                path: path.prefixed(name),
                expected,
                actual,
                existing,
                incoming,
            },
            Self::Structure {
                path,
                expected,
                actual,
                existing,
                incoming,
            } => Self::Structure {
                path: path.prefixed(name),
                expected,
                actual,
                existing,
                incoming,
            },
        }
    }
}

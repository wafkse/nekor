//! Higher-precedence overlay for typed configuration values.

use core::error::Error as CoreError;

use fack::prelude::Error;

use crate::{
    annotation::NodeAnnotation,
    error::TypeError,
    name::{Name, Path},
    node::StructureKind,
    scalar::TypeAnnotation,
    source::Origin,
};

/// A value that can apply a higher-precedence value while preserving
/// invariants.
pub trait Overlay<Rhs = Self>: Sized {
    /// The failure produced when the overlay violates an inherited invariant.
    type Error: CoreError;

    /// Apply `derived` over this value and return the resolved value.
    ///
    /// # Errors
    ///
    /// Returns an error when the derived value violates inherited type or
    /// structural constraints.
    fn overlay(self, derived: Rhs) -> Result<Self, Self::Error>;
}

/// A failure produced while applying a higher-precedence configuration value.
#[derive(Debug, Error)]
pub enum OverlayError {
    /// The derived node changes an inherited node annotation.
    #[error("node annotation `{actual}` conflicts with inherited `{expected}` at `{path}`")]
    NodeAnnotation {
        /// The semantic path whose annotation changed.
        path: Path,
        /// The inherited annotation.
        expected: NodeAnnotation,
        /// The derived annotation.
        actual: NodeAnnotation,
        /// The inherited source location.
        inherited: Origin,
        /// The derived source location.
        derived: Origin,
    },
    /// The derived node changes an inherited structural form.
    #[error("structure `{actual:?}` conflicts with inherited `{expected:?}` at `{path}`")]
    Structure {
        /// The semantic path whose structure changed.
        path: Path,
        /// The inherited structural form.
        expected: StructureKind,
        /// The derived structural form.
        actual: StructureKind,
        /// The inherited source location.
        inherited: Origin,
        /// The derived source location.
        derived: Origin,
    },
    /// The derived scalar changes an inherited representation annotation.
    #[error("value annotation `{actual:?}` conflicts with inherited `{expected:?}` at `{path}`")]
    ValueAnnotation {
        /// The semantic path whose value annotation changed.
        path: Path,
        /// The inherited representation annotation.
        expected: TypeAnnotation,
        /// The derived representation annotation.
        actual: TypeAnnotation,
        /// The inherited source location.
        inherited: Origin,
        /// The derived source location.
        derived: Origin,
    },
    /// The derived scalar cannot be represented by the inherited annotation.
    #[error("derived value at `{path}` cannot satisfy its inherited representation")]
    ValueRepresentation {
        /// The semantic path whose scalar could not be represented.
        path: Path,
        /// The inherited source location.
        inherited: Origin,
        /// The derived source location.
        derived: Origin,
        /// The typed representation failure.
        error: TypeError,
    },
}

impl OverlayError {
    /// Prefix the failing semantic path with one containing node name.
    #[must_use]
    pub fn prefixed(self, name: Name) -> Self {
        match self {
            Self::NodeAnnotation {
                path,
                expected,
                actual,
                inherited,
                derived,
            } => Self::NodeAnnotation {
                path: path.prefixed(name),
                expected,
                actual,
                inherited,
                derived,
            },
            Self::Structure {
                path,
                expected,
                actual,
                inherited,
                derived,
            } => Self::Structure {
                path: path.prefixed(name),
                expected,
                actual,
                inherited,
                derived,
            },
            Self::ValueAnnotation {
                path,
                expected,
                actual,
                inherited,
                derived,
            } => Self::ValueAnnotation {
                path: path.prefixed(name),
                expected,
                actual,
                inherited,
                derived,
            },
            Self::ValueRepresentation {
                path,
                inherited,
                derived,
                error,
            } => Self::ValueRepresentation {
                path: path.prefixed(name),
                inherited,
                derived,
                error,
            },
        }
    }
}

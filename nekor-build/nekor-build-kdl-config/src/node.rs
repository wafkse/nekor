//! Semantic configuration nodes.

use crate::{
    annotation::NodeAnnotation,
    document::Document,
    merge::{Merge, MergeError},
    name::Path,
    overlay::{Overlay, OverlayError},
    scalar::Scalar,
    source::Origin,
};

/// An ordered sequence of scalar configuration values.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScalarList(Vec<Scalar>);

impl ScalarList {
    /// Construct a scalar list while preserving list identity for any length.
    #[inline]
    #[must_use]
    pub const fn new(values: Vec<Scalar>) -> Self {
        Self(values)
    }

    /// Determine the number of scalar elements.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        let &Self(ref value_list) = self;

        value_list.len()
    }

    /// Determine whether the list contains no elements.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Determine the scalar elements as a slice.
    #[inline]
    #[must_use]
    pub const fn as_slice(&self) -> &[Scalar] {
        let &Self(ref values) = self;

        values.as_slice()
    }
}

/// The structural form of a configuration node.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    /// A singular scalar value.
    Scalar(Scalar),
    /// A nested configuration object.
    Object(Document),
    /// An ordered sequence of scalar values.
    List(ScalarList),
}

/// The structural class of a semantic node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructureKind {
    /// A scalar node.
    Scalar,
    /// An object node.
    Object,
    /// A list node.
    List,
}

/// A named semantic node in a typed configuration document.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// The application-defined annotation attached to the node.
    annotation: Option<NodeAnnotation>,
    /// The structural value stored by the node.
    kind: NodeKind,
    /// The source location that supplied the node.
    origin: Origin,
}

impl Node {
    /// Determine the semantic node annotation.
    #[inline]
    #[must_use]
    pub const fn annotation(&self) -> Option<&NodeAnnotation> {
        let &Self { ref annotation, .. } = self;

        annotation.as_ref()
    }

    /// Determine the structural value of this node.
    #[inline]
    #[must_use]
    pub const fn kind(&self) -> &NodeKind {
        let &Self { ref kind, .. } = self;

        kind
    }

    /// Determine the source location that supplied this node.
    #[inline]
    #[must_use]
    pub const fn origin(&self) -> &Origin {
        let &Self { ref origin, .. } = self;

        origin
    }

    /// Construct a node after parsing and type interpretation have succeeded.
    #[inline]
    pub(crate) const fn from_parts(annotation: Option<NodeAnnotation>, kind: NodeKind, origin: Origin) -> Self {
        Self {
            annotation,
            kind,
            origin,
        }
    }
}

impl NodeKind {
    /// Determine the structural class of this node value.
    #[inline]
    #[must_use]
    pub const fn structure(&self) -> StructureKind {
        match self {
            &Self::Scalar(..) => StructureKind::Scalar,
            &Self::Object(..) => StructureKind::Object,
            &Self::List(..) => StructureKind::List,
        }
    }
}

impl Merge for Node {
    type Error = MergeError;

    fn merge(self, incoming: Self) -> Result<Self, Self::Error> {
        let Self {
            annotation,
            kind,
            origin,
        } = self;
        let Self {
            annotation: incoming_annotation,
            kind: incoming_kind,
            origin: incoming_origin,
        } = incoming;

        let annotation = match (annotation, incoming_annotation) {
            (Some(expected), Some(actual)) if expected != actual => {
                return Err(MergeError::NodeAnnotation {
                    path: Path::new(),
                    expected,
                    actual,
                    existing: origin,
                    incoming: incoming_origin,
                });
            },
            (Some(annotation), _) | (None, Some(annotation)) => Some(annotation),
            (None, None) => None,
        };

        let expected = kind.structure();
        let actual = incoming_kind.structure();
        let kind = match (kind, incoming_kind) {
            (NodeKind::Object(existing), NodeKind::Object(incoming)) => NodeKind::Object(existing.merge(incoming)?),
            (left, right) if left.structure() == right.structure() => {
                return Err(MergeError::DuplicateValue {
                    path: Path::new(),
                    existing: origin,
                    incoming: incoming_origin,
                });
            },
            _ => {
                return Err(MergeError::Structure {
                    path: Path::new(),
                    expected,
                    actual,
                    existing: origin,
                    incoming: incoming_origin,
                });
            },
        };

        Ok(Self {
            annotation,
            kind,
            origin,
        })
    }
}

impl Overlay for Node {
    type Error = OverlayError;

    fn overlay(self, derived: Self) -> Result<Self, Self::Error> {
        let Self {
            annotation,
            kind,
            origin,
        } = self;
        let Self {
            annotation: derived_annotation,
            kind: derived_kind,
            origin: derived_origin,
        } = derived;

        let annotation = match (annotation, derived_annotation) {
            (Some(expected), Some(actual)) if expected != actual => {
                return Err(OverlayError::NodeAnnotation {
                    path: Path::new(),
                    expected,
                    actual,
                    inherited: origin,
                    derived: derived_origin,
                });
            },
            (Some(annotation), _) | (None, Some(annotation)) => Some(annotation),
            (None, None) => None,
        };

        let expected = kind.structure();
        let actual = derived_kind.structure();
        let kind = match (kind, derived_kind) {
            (NodeKind::Object(inherited), NodeKind::Object(derived)) => NodeKind::Object(inherited.overlay(derived)?),
            (NodeKind::Scalar(inherited), NodeKind::Scalar(derived)) => NodeKind::Scalar(inherited.overlay(derived)?),
            (NodeKind::List(_), NodeKind::List(derived)) => NodeKind::List(derived),
            _ => {
                return Err(OverlayError::Structure {
                    path: Path::new(),
                    expected,
                    actual,
                    inherited: origin,
                    derived: derived_origin,
                });
            },
        };

        Ok(Self {
            annotation,
            kind,
            origin: derived_origin,
        })
    }
}

//! KDL annotation types.

use core::fmt;

/// An application-defined KDL annotation identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Annotation(String);

impl Annotation {
    /// Construct an annotation from its identifier value.
    #[inline]
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Determine the identifier value of the annotation.
    #[inline]
    #[must_use]
    pub const fn as_str(&self) -> &str {
        let &Self(ref value) = self;

        value.as_str()
    }
}

impl fmt::Display for Annotation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A KDL annotation attached to a node rather than to one of its values.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeAnnotation(Annotation);

impl NodeAnnotation {
    /// Construct a node annotation from its identifier value.
    #[inline]
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(Annotation::new(value))
    }

    /// Determine the underlying annotation identifier.
    #[inline]
    #[must_use]
    pub const fn annotation(&self) -> &Annotation {
        let &Self(ref annotation) = self;

        annotation
    }

    /// Determine the identifier value of the node annotation.
    #[inline]
    #[must_use]
    pub const fn as_str(&self) -> &str {
        self.annotation().as_str()
    }
}

impl fmt::Display for NodeAnnotation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.annotation(), formatter)
    }
}

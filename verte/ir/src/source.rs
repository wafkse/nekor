//! Canonical source identities and source-qualified byte extents.

use alloc::sync::Arc;

use aldebaran::prelude::Span;

/// A host-defined canonical identity without filesystem interpretation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
// NOTE(invariant): None.
pub struct Id(Arc<str>);

impl Id {
    /// Own the host-assigned identity without normalizing its text.
    #[inline]
    pub fn new(identity: impl Into<Arc<str>>) -> Self {
        Self(identity.into())
    }

    /// Borrow the host's canonical source identity as text.
    #[inline]
    pub fn as_str(&self) -> &str {
        let Self(identity) = self;

        identity
    }
}

/// A byte extent qualified by the source to which it belongs.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
// NOTE(invariant): None.
pub struct Origin {
    /// Canonical identity supplied by the source provider.
    source: Id,

    /// Original byte coordinates, including EOF insertion boundaries.
    span: Span,
}

impl Origin {
    /// Associate coordinates with their canonical source.
    #[inline]
    pub const fn new(source: Id, span: Span) -> Self {
        Self { source, span }
    }

    /// Retrieve the identity independently of its source coordinates.
    #[inline]
    pub const fn source(&self) -> &Id {
        let Self { source, .. } = self;

        source
    }

    /// Retrieve the original byte extent.
    #[inline]
    pub const fn span(&self) -> Span {
        let &Self { span, .. } = self;

        span
    }
}

/// Source text owned by a compilation snapshot or completed configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
// NOTE(invariant): None.
pub struct Text {
    /// Canonical source identity.
    identity: Id,

    /// Immutable original UTF-8 text.
    text: Arc<str>,
}

impl Text {
    /// Own the source text so it can outlive the provider's buffers.
    #[inline]
    pub fn new(identity: Id, text: impl Into<Arc<str>>) -> Self {
        Self {
            identity,
            text: text.into(),
        }
    }

    /// Retrieve the source's canonical identity.
    #[inline]
    pub const fn identity(&self) -> &Id {
        let Self { identity, .. } = self;

        identity
    }

    /// Borrow the original text.
    #[inline]
    pub fn text(&self) -> &str {
        let Self { text, .. } = self;

        text
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::String;

    use aldebaran::prelude::Span;

    use super::{Id, Origin, Text};

    #[test]
    fn sources_outlive_provider_buffers_and_distinguish_equal_spans() {
        let identity_text = String::from("host:base");
        let content = String::from("value");
        let source = Text::new(Id::new(identity_text.as_str()), content.as_str());
        let first = Origin::new(source.identity().clone(), Span::unit(0));
        let second = Origin::new(Id::new("host:child"), Span::unit(0));

        drop(identity_text);
        drop(content);

        assert_eq!(source.text(), "value");
        assert_eq!(source.identity().as_str(), "host:base");
        assert_ne!(first, second);
    }
}

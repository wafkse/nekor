//! Borrowed source text paired with the host's canonical source identity.

/// A UTF-8 source with a host-assigned identity.
#[derive(Debug, Clone, Copy)]
// NOTE(invariant): None.
pub struct Source<'source, Id> {
    /// Canonical identity used when attaching diagnostics.
    identity: Id,

    /// Text whose byte coordinates are used by syntax spans.
    text: &'source str,
}

impl<'source, Id> Source<'source, Id> {
    /// Associate text with its canonical identity.
    #[inline]
    pub const fn new(identity: Id, text: &'source str) -> Self {
        Self { identity, text }
    }

    /// Retrieve the host-provided canonical identity.
    #[inline]
    pub const fn identity(&self) -> &Id {
        let Self { identity, .. } = self;

        identity
    }

    /// Retrieve the original, unmodified UTF-8 source.
    #[inline]
    pub const fn text(&self) -> &'source str {
        let &Self { text, .. } = self;

        text
    }
}

//! Source provenance for typed configuration values.

use std::fmt;

use camino::{Utf8Path, Utf8PathBuf};
use miette::SourceSpan;

/// The source that supplied configuration data.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Source {
    /// A source without a filesystem identity.
    Anonymous,

    /// A source loaded from a UTF-8 filesystem path.
    File(Utf8PathBuf),
}

impl Source {
    /// Construct an anonymous source identity.
    #[inline]
    #[must_use]
    pub const fn anonymous() -> Self {
        Self::Anonymous
    }

    /// Construct a filesystem-backed source identity.
    #[inline]
    #[must_use]
    pub fn file(path: impl Into<Utf8PathBuf>) -> Self {
        Self::File(path.into())
    }

    /// Determine the filesystem path for this source when one exists.
    #[inline]
    #[must_use]
    pub fn path(&self) -> Option<&Utf8Path> {
        match self {
            Self::Anonymous => None,
            Self::File(path) => Some(path.as_path()),
        }
    }
}

impl fmt::Display for Source {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Anonymous => formatter.write_str("<anonymous>"),
            Self::File(path) => fmt::Display::fmt(path, formatter),
        }
    }
}

/// The exact source location that supplied a semantic value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Origin {
    source: Source,
    span: SourceSpan,
}

impl Origin {
    /// Construct source provenance from a source identity and span.
    #[inline]
    #[must_use]
    pub const fn new(source: Source, span: SourceSpan) -> Self {
        Self { source, span }
    }

    /// Determine the source identity.
    #[inline]
    #[must_use]
    pub const fn source(&self) -> &Source {
        let Self { source, .. } = self;

        source
    }

    /// Determine the source span.
    #[inline]
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        let Self { span, .. } = self;

        *span
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { source, span } = self;

        write!(formatter, "{}@{}+{}", source, span.offset(), span.len())
    }
}

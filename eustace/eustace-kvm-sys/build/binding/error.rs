//! Classify rejected Bindgen declarations without discarding their diagnostic context.

use core::fmt;

/// Failure to prove that a generated declaration can retain its C layout.
#[derive(Debug)]
pub enum AdaptationError {
    /// Required declarations, accessors, or layout evidence are absent.
    Missing(String),

    /// A declaration or accessor occurs more than once.
    Duplicate(String),

    /// Independently reported signatures, names, or layouts disagree.
    Conflict(String),

    /// Bindgen emitted a representation the adapter does not support.
    Unsupported(String),

    /// A computed size or bit range cannot be represented.
    Overflow(String),

    /// A bit range extends beyond its backing storage.
    OutOfBounds(String),

    /// Distinct named fields occupy intersecting bit ranges.
    Overlap(String),

    /// An expression or declaration is malformed.
    Invalid(String),

    /// A helper remains after its replacement should have been emitted.
    Untranslated(String),
}

impl fmt::Display for AdaptationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (kind, message) = match self {
            Self::Missing(message) => ("missing binding evidence", message),
            Self::Duplicate(message) => ("duplicate binding declaration", message),
            Self::Conflict(message) => ("conflicting binding evidence", message),
            Self::Unsupported(message) => ("unsupported binding representation", message),
            Self::Overflow(message) => ("binding arithmetic overflow", message),
            Self::OutOfBounds(message) => ("binding range out of bounds", message),
            Self::Overlap(message) => ("overlapping binding fields", message),
            Self::Invalid(message) => ("invalid binding declaration", message),
            Self::Untranslated(message) => ("untranslated binding helper", message),
        };

        write!(formatter, "{kind}: {message}")
    }
}

impl core::error::Error for AdaptationError {}

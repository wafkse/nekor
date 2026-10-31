//! Context management for `verte-parse`.

use alloc::string::String;

use aldebaran::interner::storage::Storage;

/// The scanner context in use for source-level scanning via [`Lex`].
#[repr(transparent)]
// NOTE(invariant): None.
pub struct ScanContext<'a> {
    /// The interner storage context within this scanner context.
    storage: Storage<'a, str, String>,
}

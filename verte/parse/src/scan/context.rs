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

impl<'a> ScanContext<'a> {
    /// Retrieve the storage interner.
    #[inline]
    pub const fn storage(&self) -> &Storage<'a, str, String> {
        let Self { storage, .. } = self;

        storage
    }

    /// Mutably retrieve the storage interner.
    #[inline]
    pub const fn storage_mut(&mut self) -> &mut Storage<'a, str, String> {
        let Self { storage, .. } = self;

        storage
    }
}

//! Context management for `verte-parse`.

use alloc::{rc::Rc, string::String};
use core::cell::{BorrowError, BorrowMutError, Ref, RefCell, RefMut};

use aldebaran::interner::storage::Storage;

use super::tree::group::Cache;

/// The scanner context in use for source-level scanning via [`Lex`](aldebaran::prelude::Lex).
#[derive(Debug)]
// NOTE(invariant): None.
pub struct ScanContext<'a> {
    /// The interner storage context within this scanner context.
    storage: Storage<'a, str, String>,

    /// Delimiter boundaries discovered while recognizing groups.
    boundaries: Cache<'a>,
}

impl<'a> ScanContext<'a> {
    /// Create a context with an empty shared string interner.
    #[inline]
    pub fn new() -> Self {
        Self {
            storage: Storage::empty(),
            boundaries: Cache::new(),
        }
    }

    /// Borrow recognized delimiter boundaries in this storage domain.
    #[inline]
    pub const fn boundaries(&self) -> &Cache<'a> {
        let Self { boundaries, .. } = self;

        boundaries
    }

    /// Borrow the cache for recording newly recognized delimiter boundaries.
    #[inline]
    pub const fn boundaries_mut(&mut self) -> &mut Cache<'a> {
        let Self { boundaries, .. } = self;

        boundaries
    }

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

impl Default for ScanContext<'_> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Shared ownership of a lexical storage domain across bounded group streams.
#[derive(Debug, Clone)]
// NOTE(invariant): Every clone denotes the same interning domain.
pub struct Shared<'s>(Rc<RefCell<ScanContext<'s>>>);

impl<'s> Shared<'s> {
    /// Create an empty storage domain.
    #[inline]
    pub fn new() -> Self {
        Self(Rc::new(RefCell::new(ScanContext::new())))
    }

    /// Recover exclusive ownership when no stream holds a reference to the domain.
    #[inline]
    pub fn into_inner(self) -> Result<ScanContext<'s>, Self> {
        let Self(storage) = self;

        Rc::try_unwrap(storage).map(RefCell::into_inner).map_err(Self)
    }

    /// Borrow the domain for inspecting interned content.
    #[inline]
    pub fn borrow(&self) -> Result<Ref<'_, ScanContext<'s>>, BorrowError> {
        let Self(storage) = self;

        storage.try_borrow()
    }

    /// Borrow the domain for lexical construction.
    #[inline]
    pub fn borrow_mut(&self) -> Result<RefMut<'_, ScanContext<'s>>, BorrowMutError> {
        let Self(storage) = self;

        storage.try_borrow_mut()
    }
}

impl Default for Shared<'_> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

//! Source-level scanning for Verte.

use alloc::string::String;
use core::ops;

use aldebaran::interner::storage::Storage;

pub mod trivia;

/// The scanner context in use for source-level scanning via [`Lex`].
#[repr(transparent)]
pub struct ScanContext<'a>(Storage<'a, str, String>);

// TODO: Maybe this shouldn't be dereferenceable?
//
// Additions to the contexta re unlikely, but they could force a huge refactor in the future.

impl<'a> ops::Deref for ScanContext<'a> {
    type Target = Storage<'a, str, String>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        let Self(target_storage) = self;

        target_storage
    }
}

impl<'a> ops::DerefMut for ScanContext<'a> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        let Self(target_storage) = self;

        target_storage
    }
}

#![cfg_attr(not(any(test, miri, usermode)), no_std)]
//! Storage and representation types used by low-level facilities.
//!
//! These modules describe reserved bytes, integer portions, relative pointers and entry
//! tokens without selecting processor instructions. Their callers supply any architectural
//! setup and resource ownership required to use the represented values.

pub mod reserved;

pub mod opaque;

pub mod swap;

pub mod partitioned;

pub mod rel_ptr;

pub mod low_level;

pub mod cache;

pub mod inline_vec;

pub mod prelude {
    //! A prelude module for the `nekor-aal-agnostic` crate.
    //!
    //! This simply re-exports the usual utilities for easier use.

    pub use crate::{
        low_level::{Contextual, LowLevel},
        opaque::Opaque,
        partitioned::Partitioned,
        rel_ptr::{RelPtr, RelPtrMut},
        reserved::Reserved,
        swap::Swap,
    };
}

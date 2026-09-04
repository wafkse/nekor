#![cfg_attr(not(any(test, miri, usermode)), no_std)]
//! Architecture-agnostic utilities that are useful for miscenalleous low-level
//! work
//!
//! The structures contained within this crate do not posess any
//! architecture-specific architecture.

pub mod reserved;

pub mod opaque;

pub mod swap;

pub mod partitioned;

pub mod rel_ptr;

pub mod low_level;

pub mod cache;

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

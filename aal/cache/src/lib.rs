#![cfg_attr(not(any(test, miri, usermode)), no_std)]

//! Architectural Support for cache-adjacent operations.

pub mod arch;

pub mod line;

pub mod padded;

pub mod prelude {
    //! The prelude module of this crate.

    pub use crate::{line::Cacheline, padded::CachePadded};
}

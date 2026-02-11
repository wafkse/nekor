#![cfg_attr(not(any(test, miri)), no_std)]
#![forbid(
    clippy::all,
    clippy::perf,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::panic,
    clippy::pedantic,
    rustdoc::all
)]
//! A memory-mapped register interface for the Nekor unikernel.
//!
//! Note that this is purely for memory-mapped register access, not machine
//! register access.
//!
//! For machine register access, defer to the per-architecture implementations
//! in the `nekor-arch` crate.

pub mod memory;

pub mod mode;

pub mod prelude {
    //! The prelude module provides commonly used types and traits for working
    //! with memory-mapped registers.

    pub use crate::memory::Memory;

    pub use crate::mode::{Ro, Rw, Wo};
}

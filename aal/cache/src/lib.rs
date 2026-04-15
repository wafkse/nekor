#![cfg_attr(not(any(test, miri, usermode)), no_std)]
#![forbid(
    clippy::all,
    clippy::perf,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::panic,
    clippy::pedantic,
    rustdoc::all
)]
//! Architectural Support for cache-adjacent operations.

pub mod arch;

pub mod line;

pub mod padded;

pub mod prelude {
    //! The prelude module of this crate.

    pub use crate::line::Cacheline;

    pub use crate::padded::CachePadded;
}

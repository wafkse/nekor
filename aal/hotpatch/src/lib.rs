#![cfg_attr(not(any(test, miri, usermode)), no_std)]
#![forbid(
    clippy::all,
    clippy::perf,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::panic,
    rustdoc::all
)]
#![deny(clippy::pedantic)]
//! Architecture-level runtime code patching support.

pub mod arch;

pub mod cmc;

pub mod patch;

pub mod prelude {
    //! The prelude module provides commonly used types and traits for
    //! performing runtime code patching.

    pub use crate::patch::{
        choose::Chosen,
        delegate::{Delegated, Delegator, Logical, Single},
        pod::Pod,
        site::Patch,
    };
}

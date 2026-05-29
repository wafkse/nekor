#![no_std]
#![forbid(
    clippy::all,
    clippy::perf,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::panic,
    clippy::pedantic,
    rustdoc::all
)]
//! Utilities for working with per-cpu data.

mod domain;

pub mod limit;

pub mod processor;

pub mod local;

pub mod prelude {
    pub use crate::limit::Cores;

    pub use crate::processor::{CoreId, Handout, Reutilization};

    pub use crate::local::{Local, PerCpu};
}

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
//! The System-on-Chip Abstraction Layer.
//!
//! This provides abstractions and utilities to interact with the peripherals
//! built-in into a `SoC`.

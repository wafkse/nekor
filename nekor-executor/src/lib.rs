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
//! The asynchronous executor for Nekor.
//!
//! See the [`Executor`] struct for more information.

pub mod run_queue;

pub mod task;

pub mod wake;

pub mod schedule;

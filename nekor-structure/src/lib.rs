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
//! Lock-free and allocator-less data structures for the Nekor unikernel.
//!
//! This crate does not have a prelude module. Each data structure must be
//! imported individually.

pub mod slotted;

pub mod linked_list;

pub use slotted::{arena, queue};

#![cfg_attr(not(any(test, miri)), no_std)]
//! The asynchronous executor for Nekor.
//!
//! See the [`Executor`] struct for more information.

#[cfg(test)]
extern crate alloc;

pub mod run_queue;

pub mod task;

pub mod wake;

pub mod schedule;

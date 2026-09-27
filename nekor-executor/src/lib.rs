#![cfg_attr(not(any(test, miri)), no_std)]
//! The asynchronous executor for Nekor.
//!
//! See [`task::Task`] and [`run_queue`] for the task and scheduling interfaces.

#[cfg(test)]
extern crate alloc;

pub mod run_queue;

pub mod task;

pub mod wake;

pub mod schedule;

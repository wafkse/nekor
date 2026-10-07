#![no_std]

//! Memory support for tests running under Miri.
//!
//! The public modules exist only under `cfg(miri)`. An intentional static root
//! keeps an allocation alive through the test process. Callers remain responsible
//! for initializing and synchronizing its contents.

#[cfg(miri)]
extern crate alloc;

#[cfg(miri)]
mod ffi;

#[cfg(miri)]
pub mod alignment;

#[cfg(miri)]
pub mod borrow;

#[cfg(miri)]
pub mod memory;

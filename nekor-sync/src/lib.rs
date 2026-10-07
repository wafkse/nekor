#![cfg_attr(not(any(test, miri, usermode)), no_std)]
#![expect(
    clippy::cfg_not_test,
    reason = "non-`no_std` environments such as `cfg(test)` do not require declaring the existence of the `alloc` crate"
)]
//! Synchronization primitives for atomic state and mutual exclusion.

#[cfg(not(any(test, miri, usermode)))]
extern crate alloc;

pub mod mutex;

pub mod atomic;

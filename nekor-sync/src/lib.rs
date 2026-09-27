#![cfg_attr(not(any(test, miri, usermode)), no_std)]

//! Synchronization primitives for atomic state and mutual exclusion.

extern crate alloc;

pub mod mutex;

pub mod atomic;

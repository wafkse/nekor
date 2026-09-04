#![cfg_attr(not(any(test, miri, usermode)), no_std)]

extern crate alloc;

pub mod mutex;

pub mod atomic;

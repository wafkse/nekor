#![cfg_attr(not(any(test, miri, usermode)), no_std)]

//! Interrupt-related functionality and infrastructure.

pub mod arch;

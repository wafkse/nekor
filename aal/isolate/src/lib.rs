#![cfg_attr(not(any(test, usermode)), no_std)]

//! Processor context transfer and stack storage.
//!
//! The x86-64 transfer saves and restores general-purpose registers, the FS and GS bases,
//! and an interrupt return frame.

pub mod arch;

pub mod job;

pub mod stack;

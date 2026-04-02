//! Architecture-specific code for the `x86` architecture.
//!
//! # Compatibility
//!
//! Nekor does not support either *Real Mode* or *Protected Mode*, but modeling
//! and incomplete support for protected mode `x86` is done due to its
//! similarities with *long mode*.

pub mod mode;

pub mod msr;

pub mod instruction;

pub mod address;

pub mod segmentation;

pub mod privilege;

pub mod descriptor;

pub mod gate;

pub mod serialize;

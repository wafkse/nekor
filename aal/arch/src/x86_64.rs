//! Specialized support module for `x86` architecture's 64-bit mode of
//! execution.
//!
//! This inherits any functionality from the other [`x86`] module.
//!
//! [`x86`]: crate::x86

pub use crate::x86::{mode, msr, segmentation};

pub mod descriptor;

pub mod instruction;

pub mod gate;

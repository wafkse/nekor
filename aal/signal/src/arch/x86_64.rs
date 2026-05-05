//! Specialized support module for `x86` architecture's 64-bit mode of
//! execution.
//!
//! This inherits any functionality from the other [`x86`] module, IPI signaling
//! is agnostic of machine bitness.
//!
//! [`x86`]: crate::arch::x86

pub use crate::arch::x86::*;

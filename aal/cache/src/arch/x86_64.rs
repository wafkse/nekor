//! Cache-related intrinsics for the `x86` architecture 64-bit mode of
//! execution.
//!
//! As there are no Long Mode-specific cache control mechanisms, this simply
//! performs a glob import on [`x86`].
//!
//! [`x86`]: crate::arch::x86

pub use crate::arch::x86::*;

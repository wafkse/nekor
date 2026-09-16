//! Numeric register addresses.
//!
//! A register address is an integer that names a register in some external
//! address space. This module does not assign memory semantics to that integer.

/// Sealing implementation for [`Address`].
mod private {
    /// Marker preventing downstream [`Address`] implementations.
    pub trait Sealed {}
}

/// An integer type usable as a register address.
// TODO: Add architecture-specific address types when a register space needs
// stronger semantics than an unsigned integer.
pub trait Address: Copy + private::Sealed {}

impl private::Sealed for u8 {}
impl Address for u8 {}

impl private::Sealed for u16 {}
impl Address for u16 {}

impl private::Sealed for u32 {}
impl Address for u32 {}

impl private::Sealed for u64 {}
impl Address for u64 {}

impl private::Sealed for usize {}
impl Address for usize {}

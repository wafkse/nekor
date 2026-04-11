//! Interrupt Gates for the `x86-64` architecture.

use crate::x86::gate::GateType32;

/// An interrupt [`GateType`] intended for *Long Mode* code execution.
///
/// This *Gate Type* is exclusively supported in a *Long Mode* context, any
/// other type is disallowed.
///
/// # Effects on Interrupt Service Routine Dispatch
///
/// The only effects depend on whether it is an *Interrupt Gate* or *Trap Gate*.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
#[non_exhaustive]
pub enum GateType64 {
    /// A *Long Mode* 64-bit interrupt mode.
    ///
    /// # Remarks
    ///
    /// This value is inherited from a [`GateType32::Interrupt`].
    Interrupt = GateType32::Interrupt as u8,

    /// A *Long Mode* 64-bit interrupt trap mode.
    ///
    /// # Remarks
    ///
    /// This value is inherited from a [`GateType32::Trap`].
    Trap = GateType32::Trap as u8,
}

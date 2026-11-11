//! Typed guest exits returned by KVM vCPU execution.
//!
//! Port-I/O and MMIO exits describe guest device accesses. Exits without a dedicated variant
//! retain their original KVM reason code.

use nekor_virt_ioport::access::IoPortAccess;
use nekor_virt_mmio::access::MmioAccess;

/// Original exit reason for an exit without a dedicated Nekor run kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): None.
pub struct ExitReason(u32);

impl ExitReason {
    /// Preserve a raw exit reason returned by KVM.
    #[inline]
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Return the original exit reason supplied by KVM.
    #[inline]
    #[must_use]
    pub const fn value(self) -> u32 {
        let Self(value) = self;

        value
    }
}

/// Classified result of inspecting a KVM run exit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RunExit {
    /// Shared semantic description of the guest port-I/O operation.
    Io(IoPortAccess),

    /// Shared semantic description of the guest MMIO operation.
    Mmio(MmioAccess),

    /// The guest executed a halt instruction.
    Halt,

    /// Original KVM reason for an exit without a Nekor run record.
    Other(ExitReason),
}

//! Guest memory-mapped I/O access descriptions.

use nekor_virt_address::gpa::Gpa;

/// Number of bytes transferred by a guest MMIO access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The value is in the inclusive range 1 through 8.
pub struct MmioByteCount(u8);

impl MmioByteCount {
    /// Create a byte count accepted by the shared x86 KVM and WHP access contracts.
    #[inline]
    #[must_use]
    pub const fn new(value: u8) -> Option<Self> {
        match value {
            1..=8 => Some(Self(value)),
            _ => None,
        }
    }

    /// Return the number of transferred bytes.
    #[inline]
    #[must_use]
    pub const fn value(self) -> u8 {
        let Self(value) = self;

        value
    }
}

/// Guest MMIO read request that awaits response bytes from userspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MmioRead {
    /// Guest physical address read by the operation.
    pub address: Gpa,

    /// Number of response bytes requested by the guest.
    pub byte_count: MmioByteCount,
}

/// Guest MMIO write payload copied from the active backend access record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MmioWrite {
    /// Guest physical address written by the operation.
    pub address: Gpa,

    /// Access bytes, of which only the prefix selected by `byte_count` is meaningful.
    pub data: [u8; 8],

    /// Number of meaningful bytes in `data`.
    pub byte_count: MmioByteCount,
}

/// Direction-classified guest MMIO access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MmioAccess {
    /// The guest read requires a userspace response.
    Read(MmioRead),

    /// The guest wrote the returned payload to the device.
    Write(MmioWrite),
}

//! Guest I/O port addresses.

use core::num::NonZero;

/// A 16-bit port address used by x86 guest I/O instructions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): None.
pub struct IoPort(u16);

impl IoPort {
    /// Preserve a raw port address.
    #[inline]
    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Return the 16-bit port address.
    #[inline]
    #[must_use]
    pub const fn value(self) -> u16 {
        let Self(target_port) = self;

        target_port
    }
}

/// Direction of a guest port-I/O operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IoDirection {
    /// The guest reads from the port and userspace supplies the value.
    In,

    /// The guest writes to the port and userspace consumes the value.
    Out,
}

/// Width of each element in a guest port-I/O operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IoElementSize {
    /// Each element occupies one byte.
    Byte,

    /// Each element occupies two bytes.
    Word,

    /// Each element occupies four bytes.
    Dword,
}

impl IoElementSize {
    /// Return the number of bytes transferred by each element.
    #[inline]
    #[must_use]
    pub const fn byte_count(self) -> usize {
        match self {
            Self::Byte => 1,
            Self::Word => 2,
            Self::Dword => 4,
        }
    }
}

/// A positive number of port-I/O elements requested by the guest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): None.
pub struct IoElementCount(NonZero<u64>);

impl IoElementCount {
    /// Create an element count when `value` is nonzero.
    #[inline]
    #[must_use]
    pub const fn new(value: u64) -> Option<Self> {
        match NonZero::<u64>::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Return the number of requested elements.
    #[inline]
    #[must_use]
    pub const fn value(self) -> u64 {
        let Self(value) = self;

        value.get()
    }
}

/// Semantic details of a guest port-I/O operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IoPortAccess {
    /// Direction in which the guest transfers values to or from the port.
    pub direction: IoDirection,

    /// Width of each transferred element.
    pub element_size: IoElementSize,

    /// Port addressed by the guest operation.
    pub port: IoPort,

    /// Positive number of elements transferred by the operation.
    pub element_count: IoElementCount,
}

//! Guest I/O port addresses.

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

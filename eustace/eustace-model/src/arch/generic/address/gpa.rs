//! Guest physical addresses.

/// A guest physical address, distinct from guest virtual and host virtual addresses.
///
/// The value identifies a byte in the physical address space exposed to a guest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): None.
pub struct Gpa(u64);

impl Gpa {
    /// Preserve a raw guest physical address value.
    #[inline]
    #[must_use]
    pub const fn new(target_value: u64) -> Self {
        Self(target_value)
    }

    /// Return the address value in bytes.
    #[inline]
    #[must_use]
    pub const fn value(self) -> u64 {
        let Self(target_value) = self;

        target_value
    }
}

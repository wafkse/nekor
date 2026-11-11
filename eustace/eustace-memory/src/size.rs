//! Guest memory sizes measured in bytes.

use core::num::NonZeroU64;

/// A nonzero guest memory extent measured in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): None.
pub struct GuestMemorySize(NonZeroU64);

impl GuestMemorySize {
    /// Create a guest memory size when `byte_count` is nonzero.
    #[inline]
    #[must_use]
    pub const fn new(byte_count: u64) -> Option<Self> {
        match NonZeroU64::new(byte_count) {
            Some(byte_count) => Some(Self(byte_count)),
            None => None,
        }
    }

    /// Return the number of guest memory bytes.
    #[inline]
    #[must_use]
    pub const fn value(self) -> u64 {
        let Self(byte_count) = self;

        byte_count.get()
    }
}

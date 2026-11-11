//! Guest physical address extents.

use nekor_virt_address::gpa::Gpa;

use crate::size::GuestMemorySize;

/// A nonempty guest physical address extent with a byte-sized length.
///
/// Construction verifies that the final byte is representable by `Gpa`. It does not check the
/// physical-address width configured for a particular machine, nor any host page alignment rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
// NOTE(invariant): The final byte of the extent is representable as a `Gpa`.
pub struct GuestMemoryRegion {
    /// First guest physical address in the extent.
    start: Gpa,

    /// Number of guest memory bytes beginning at `start`.
    size: GuestMemorySize,
}

impl GuestMemoryRegion {
    /// Create a region when its final guest byte is representable.
    #[inline]
    #[must_use]
    pub const fn new(start: Gpa, size: GuestMemorySize) -> Option<Self> {
        let last_byte_offset = size.value() - 1;

        match start.value().checked_add(last_byte_offset) {
            Some(_last_byte) => Some(Self { start, size }),
            None => None,
        }
    }

    /// Return the first guest physical address in the extent.
    #[inline]
    #[must_use]
    pub const fn start(&self) -> Gpa {
        let Self { start, .. } = self;

        *start
    }

    /// Return the guest memory size in bytes.
    #[inline]
    #[must_use]
    pub const fn size(&self) -> GuestMemorySize {
        let Self { size, .. } = self;

        *size
    }

    /// Return the byte offset of a range when it lies within the region.
    ///
    /// The requested length is measured in bytes. An empty range may begin at the exclusive end
    /// of the region, which supports zero-length host slice operations without extending the
    /// guest address extent.
    #[inline]
    #[must_use]
    pub const fn offset_of(&self, guest_start: Gpa, byte_count: u64) -> Option<u64> {
        let Self { start, size } = self;

        match guest_start.value().checked_sub(start.value()) {
            Some(offset) => match size.value().checked_sub(offset) {
                Some(remaining) => match byte_count <= remaining {
                    true => Some(offset),
                    false => None,
                },
                None => None,
            },
            None => None,
        }
    }

    /// Return whether the region contains the specified guest physical byte.
    #[inline]
    #[must_use]
    pub const fn contains(&self, guest_address: Gpa) -> bool {
        match self.offset_of(guest_address, 1) {
            Some(_offset) => true,
            None => false,
        }
    }

    /// Return whether this extent overlaps another guest memory region.
    #[inline]
    #[must_use]
    pub const fn overlaps(&self, other: Self) -> bool {
        let Self {
            start: left_start,
            size: left_size,
        } = self;

        let Self {
            start: right_start,
            size: right_size,
        } = other;

        match left_start.value().checked_sub(right_start.value()) {
            Some(0) => true,
            Some(distance) => distance < right_size.value(),
            None => match right_start.value().checked_sub(left_start.value()) {
                Some(distance) => distance < left_size.value(),
                None => false,
            },
        }
    }
}

//! Flexible array members in raw C records.
//!
//! [`Fam`] marks trailing storage supplied by the enclosing allocation.

use core::{fmt, slice};

/// A zero-sized flexible array member aligned for its element type.
///
/// Place it last in a `repr(C)` record. The enclosing allocation owns the trailing
/// storage, which begins at the member offset rather than after any record tail
/// padding. The marker neither allocates nor drops trailing elements.
#[repr(C)]
// NOTE(invariant): None.
pub struct Fam<Element>([Element; 0]);

impl<Element> Fam<Element> {
    /// Construct the marker without allocating or initializing its trailing array.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self([])
    }

    /// Return the first trailing element pointer, preserving address and provenance.
    #[inline]
    #[must_use]
    pub const fn pointer(member_pointer: *const Self) -> *const Element {
        member_pointer.cast()
    }

    /// Return the mutable trailing element pointer, preserving address and provenance.
    #[inline]
    #[must_use]
    pub const fn pointer_mut(member_pointer: *mut Self) -> *mut Element {
        member_pointer.cast()
    }

    /// Borrow the trailing array.
    ///
    /// # Safety
    ///
    /// The safety requirements of [`slice::from_raw_parts()`] apply to the
    /// trailing region for the requested element count and returned lifetime.
    #[inline]
    #[must_use]
    pub const unsafe fn slice<'value>(member_pointer: *const Self, element_count: usize) -> &'value [Element] {
        // SAFETY: The cast preserves provenance. The caller guarantees the
        // storage and shared borrowing requirements of `from_raw_parts`.
        unsafe { slice::from_raw_parts(Self::pointer(member_pointer), element_count) }
    }

    /// Exclusively borrow the trailing array.
    ///
    /// # Safety
    ///
    /// The safety requirements of [`slice::from_raw_parts_mut()`] apply to the
    /// trailing region for the requested element count and returned lifetime.
    #[inline]
    #[must_use]
    pub const unsafe fn slice_mut<'value>(member_pointer: *mut Self, element_count: usize) -> &'value mut [Element] {
        // SAFETY: The cast preserves provenance. The caller guarantees the
        // storage and exclusive borrowing requirements of `from_raw_parts_mut`.
        unsafe { slice::from_raw_parts_mut(Self::pointer_mut(member_pointer), element_count) }
    }
}

impl<Element> Default for Fam<Element> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<Element> fmt::Debug for Fam<Element> {
    #[inline]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("Fam").finish_non_exhaustive()
    }
}

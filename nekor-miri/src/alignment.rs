//! Checked pointer alignment promises.

use core::mem;

use crate::ffi;

/// A nonzero power-of-two alignment.
#[derive(Clone, Copy, PartialEq, Eq)]
// NOTE(invariant): The stored byte count is a nonzero power of two.
pub struct Alignment(usize);

impl Alignment {
    /// Return the alignment of `T`.
    #[inline]
    pub const fn of<T>() -> Self {
        // A Rust type's alignment is a nonzero power of two.
        Self(mem::align_of::<T>())
    }

    /// Check that a byte alignment is a nonzero power of two.
    #[inline]
    pub const fn new(target_alignment: usize) -> Option<Self> {
        if target_alignment.is_power_of_two() {
            Some(Self(target_alignment))
        } else {
            None
        }
    }

    /// Return the alignment in bytes.
    #[inline]
    pub const fn value(&self) -> usize {
        let &Self(target_alignment) = self;

        target_alignment
    }
}

/// Promise that a pointer meets `alignment`.
///
/// A false promise stops interpretation. Any pointer type can be passed directly.
#[inline]
pub fn promise<T>(target_pointee: *const T, promise_alignment: Alignment)
where
    T: ?Sized,
{
    let align = promise_alignment.value();
    let ptr = target_pointee.cast::<()>();

    // SAFETY: Alignment has a valid shape. Miri checks the pointer's actual address.
    unsafe { ffi::miri_promise_symbolic_alignment(ptr, align) };
}

#[cfg(test)]
mod tests {
    use core::ptr;

    use super::{Alignment, promise};

    #[test]
    fn checked_alignment_promise() {
        assert!(Alignment::new(0).is_none());
        assert!(Alignment::new(3).is_none());

        let value = 7_u64;
        let alignment = Alignment::of::<u64>();
        promise(ptr::from_ref(&value), alignment);

        let values = [7_u64, 8];
        promise(ptr::from_ref(values.as_slice()), alignment);
    }
}

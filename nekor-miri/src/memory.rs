//! Owned allocations and intentional static roots.

use alloc::boxed::Box;
use core::{
    alloc::Layout,
    mem::{self, ManuallyDrop},
    num::NonZero,
    ptr::NonNull,
};

use crate::ffi;

/// A live allocation paired with its deallocation layout.
// NOTE(invariant): `pointer` is the base of a live Miri allocation made with `layout`.
pub struct Allocation {
    /// Base of the owned allocation.
    pointer: NonNull<u8>,

    /// Layout passed to the interpreter allocator.
    layout: Layout,
}

impl Allocation {
    /// Allocate uninitialized storage for a value of type `T`.
    ///
    /// Return `None` when `T` is zero-sized or allocation fails.
    #[inline]
    pub fn new<T>() -> Option<Self> {
        match NonZero::new(mem::size_of::<T>()) {
            Some(size) => {
                let layout = Layout::new::<T>();

                // SAFETY: Layout guarantees a valid alignment and nonzero size was checked.
                let pointer = NonNull::new(unsafe { ffi::miri_alloc(size.get(), layout.align()) }.cast::<u8>())?;

                Some(Self { pointer, layout })
            },
            None => None,
        }
    }

    /// Return the allocation base without granting access to its contents.
    #[inline]
    pub const fn pointer(&self) -> NonNull<u8> {
        let &Self { pointer, .. } = self;

        pointer
    }

    /// Return the layout used to allocate this block.
    #[inline]
    pub const fn layout(&self) -> Layout {
        let &Self { layout, .. } = self;

        layout
    }

    /// Register this allocation as a static root and consume its owner.
    ///
    /// The returned pointer remains live for the test process. Its contents
    /// still require initialization before they can be read.
    #[inline]
    pub fn leak(self) -> NonNull<u8> {
        let &Self { pointer, .. } = &self;

        // SAFETY: The owner holds the allocation base until ownership is consumed.
        unsafe { ffi::miri_static_root(pointer.as_ptr().cast_const().cast::<()>()) };

        let _owner = ManuallyDrop::new(self);

        pointer
    }

    /// Report allocation events for this block.
    #[inline]
    pub fn track(&self) {
        let &Self { pointer, .. } = self;

        // SAFETY: This handle owns a live allocation.
        unsafe { ffi::miri_track_alloc(pointer.as_ptr().cast_const().cast::<()>()) };
    }
}

impl Drop for Allocation {
    #[inline]
    fn drop(&mut self) {
        let Self { pointer, layout } = self;

        // SAFETY: The pointer and layout are the original live allocation pair.
        unsafe { ffi::miri_dealloc(pointer.as_ptr().cast::<()>(), layout.size(), layout.align()) };
    }
}

/// Register a boxed allocation as a static root and leak the box.
#[inline]
pub fn leak<T>(target_boxed: Box<T>) -> &'static mut T
where
    T: ?Sized + 'static,
{
    // T may be unsized, so check the size of the value.
    let is_nonzero_size = mem::size_of_val(&*target_boxed) != usize::MIN;

    if is_nonzero_size {
        // SAFETY: A nonzero boxed value starts at the allocation base.
        unsafe { ffi::miri_static_root(Box::as_ptr(&target_boxed).cast::<()>()) };
    }

    Box::leak(target_boxed)
}

#[cfg(test)]
mod tests {
    use alloc::{boxed::Box, vec};
    use core::{alloc::Layout, ptr::NonNull};

    use super::{Allocation, leak};

    #[test]
    fn allocation_ownership_and_box_roots() {
        assert!(Allocation::new::<()>().is_none());

        let layout = Layout::new::<u64>();
        let allocation = Allocation::new::<u64>().expect("nonzero allocation");
        assert_eq!(allocation.layout(), layout);
        assert_eq!(allocation.pointer().cast::<u64>().as_ptr().align_offset(8), 0);
        allocation.track();
        drop(allocation);

        let rooted = Allocation::new::<u64>().expect("nonzero allocation");
        let _rooted: NonNull<u8> = rooted.leak();

        assert_eq!(*leak(Box::new(42_u64)), 42);
        assert_eq!(*leak(Box::new(())), ());

        let slice: Box<[u8]> = vec![1, 2, 3].into_boxed_slice();
        assert_eq!(leak(slice), &[1, 2, 3]);

        let empty: Box<[u8]> = vec![].into_boxed_slice();
        assert!(leak(empty).is_empty());
    }
}

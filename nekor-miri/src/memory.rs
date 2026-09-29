//! Owned Miri memory and intentional static roots.

use alloc::boxed::Box;
use core::{
    alloc::Layout,
    mem::{self, ManuallyDrop},
    ptr::NonNull,
};

use crate::ffi;

/// A live Miri allocation with its original deallocation layout.
// NOTE(invariant): `pointer` is the base of a live Miri allocation made with `layout`.
pub struct Allocation {
    /// Base of the owned allocation.
    pointer: NonNull<u8>,

    /// Layout passed to the interpreter allocator.
    layout: Layout,
}

impl Allocation {
    /// Allocate uninitialized memory. Zero-sized layouts have no allocation.
    #[inline]
    pub fn new(layout: Layout) -> Option<Self> {
        if layout.size() == 0 {
            return None;
        }

        // SAFETY: Layout guarantees a valid alignment and nonzero size was checked.
        let pointer = NonNull::new(unsafe { ffi::miri_alloc(layout.size(), layout.align()) }.cast::<u8>());
        let Some(pointer) = pointer else {
            unreachable!("Miri allocation returned null for a nonzero layout")
        };
        Some(Self { pointer, layout })
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

    /// Register this allocation as a process lifetime root and consume its owner.
    #[inline]
    pub fn leak(self) -> NonNull<u8> {
        let &Self { pointer, .. } = &self;

        // SAFETY: The owner holds the allocation base until ownership is consumed.
        unsafe { ffi::miri_static_root(pointer.as_ptr().cast_const().cast::<()>()) };
        let _owner = ManuallyDrop::new(self);
        pointer
    }

    /// Ask Miri to report allocation events for this block.
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
pub fn leak<T>(value: Box<T>) -> &'static mut T
where
    T: ?Sized + 'static,
{
    let size = mem::size_of_val(&*value);
    if size != 0 {
        // SAFETY: A nonzero boxed value starts at the allocation base.
        unsafe { ffi::miri_static_root((&*value as *const T).cast::<()>()) };
    }
    Box::leak(value)
}

//! Allocation identities and interpreter borrow diagnostics.

use core::{mem, ptr};

use crate::ffi;

/// Opaque allocation identity used only for diagnostics.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AllocationId(u64);

/// Which pointer tags to display.
#[derive(Clone, Copy)]
pub enum Visibility {
    /// Show every tag.
    All,

    /// Show only named tags where supported.
    Named,
}

impl AllocationId {
    /// Capture the identity of a nonzero-sized referenced value.
    #[inline]
    pub fn of<T>(value: &T) -> Option<Self>
    where
        T: ?Sized,
    {
        if mem::size_of_val(value) == 0 {
            None
        } else {
            // SAFETY: A reference to a nonzero-sized value identifies a live allocation.
            Some(unsafe { Self::raw(ptr::from_ref(value).cast::<()>()) })
        }
    }

    /// Capture an allocation identity from a raw pointer.
    ///
    /// # Safety
    ///
    /// `pointer` must carry provenance for a live allocation. Miri aborts on invalid input.
    #[inline]
    pub unsafe fn raw(pointer: *const ()) -> Self {
        // SAFETY: The caller supplies live allocation provenance.
        Self(unsafe { ffi::miri_get_alloc_id(pointer) })
    }

    /// Print the current borrow state for this captured allocation.
    #[inline]
    pub fn print(&self, visibility: Visibility) {
        let &Self(id) = self;

        let show_unnamed = matches!(visibility, Visibility::All);
        // SAFETY: The ID was obtained from the interpreter.
        unsafe { ffi::miri_print_borrow_state(id, show_unnamed) };
    }
}

/// Track allocation events for a referenced value.
#[inline]
pub fn track<T>(value: &T) -> Option<AllocationId>
where
    T: ?Sized,
{
    let id = AllocationId::of(value)?;
    // SAFETY: A nonzero-sized live reference identifies an allocation.
    unsafe { ffi::miri_track_alloc(ptr::from_ref(value).cast::<()>()) };
    Some(id)
}

/// Name a pointer tag or selected parent of a referenced value.
#[inline]
pub fn name<T>(value: &T, parent: u8, name: &str) -> bool
where
    T: ?Sized,
{
    if mem::size_of_val(value) == 0 {
        return false;
    }
    // SAFETY: A live reference supplies a valid tag, and the name is borrowed for the call.
    unsafe { ffi::miri_pointer_name(ptr::from_ref(value).cast::<()>(), parent, name.as_bytes()) };
    true
}

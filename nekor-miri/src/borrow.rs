//! Borrow diagnostics for live allocations.

use core::{mem, ptr};

use crate::ffi;

/// An allocation identity for later borrow inspection.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AllocationId(u64);

impl AllocationId {
    /// Capture the allocation that holds a nonzero-sized value.
    #[inline]
    pub fn of<T>(target_value: &T) -> Option<Self>
    where
        T: ?Sized,
    {
        if mem::size_of_val(target_value) == usize::MIN {
            None
        } else {
            let id = ffi::miri_get_alloc_id(ptr::from_ref(target_value).cast::<()>());

            Some(Self(id))
        }
    }

    /// Print this allocation's current borrow state.
    #[inline]
    pub fn print(&self) {
        let &Self(id) = self;

        ffi::miri_print_borrow_state(id, true);
    }
}

/// Report allocation events for the allocation holding a value.
#[inline]
pub fn track<T>(target_value: &T) -> Option<AllocationId>
where
    T: ?Sized,
{
    let id = AllocationId::of(target_value)?;

    ffi::miri_track_alloc(ptr::from_ref(target_value).cast::<()>());

    Some(id)
}

#[cfg(test)]
mod tests {
    use super::{AllocationId, track};

    #[test]
    fn borrow_diagnostics() {
        let value = 7_u64;
        let id = AllocationId::of(&value).expect("nonzero value");
        id.print();

        assert!(track(&value).is_some());
        assert!(AllocationId::of(&()).is_none());
    }
}

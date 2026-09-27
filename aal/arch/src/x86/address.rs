//! Address sizes.

use core::marker;

use crate::x86::mode::{Mode, Native};

/// A memory address for the target processor [`Mode`].
///
/// # Remarks
///
/// This is a [`Mode`]-agnostic way to represent an address. This type is not
/// meant to be a general-purpose pointer type
#[repr(transparent)]
#[derive(Debug)]
// NOTE(invariant): The address uses `B`'s representation and the marker prevents `Send` and `Sync`.
pub struct Address<T, B = Native>(
    // NOTE(cheri): This uses exposed provenance when a native pointer is
    // converted to an `Address`.
    B::Address,
    marker::PhantomData<fn() -> *mut T>,
)
where
    B: Mode;

impl<T> Address<T, Native> {
    /// Convert a native pointer address into an [`Address`].
    #[inline]
    pub fn native(target_address: *mut T) -> Self {
        Self(
            target_address.expose_provenance() as <Native as Mode>::Address,
            marker::PhantomData,
        )
    }
}

impl<T, B> Address<T, B>
where
    B: Mode,
{
    /// Create a new [`Address`] from a [`Mode`]-dependant address.
    #[inline]
    pub const fn to(target_address: B::Address) -> Self {
        Self(target_address, marker::PhantomData)
    }
}

//! Relative-addressing pointers.

use core::{marker, mem::MaybeUninit, pin::Pin, ptr};

/// A base-relative pointer to an instance of `T`.
#[derive(Debug, Hash, Eq, PartialEq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct RelPtr<T>(*const T, marker::PhantomPinned);

impl<T> RelPtr<T> {
    /// Construct a new base-relative pointer from a regular pointer to an
    /// instance of `T`.
    ///
    /// The final value is written to a maybe-unintialized [`RelPtr`].
    #[inline]
    pub fn new(target_base: Pin<&mut MaybeUninit<Self>>, target_value: *const T) -> Pin<&mut Self> {
        let base_address = target_base.as_ptr().addr();

        let target_relative = target_value.wrapping_byte_sub(base_address);

        // SAFETY: We initialize the pinned value and retrieve a mutable reference to
        // it, which we immediately pin. The provenance of the pointer provided is kept
        // verbatim in the pointer stored inside the structure.
        unsafe {
            let target_value = MaybeUninit::write(
                Pin::into_inner_unchecked(target_base),
                Self(target_relative, marker::PhantomPinned),
            );

            Pin::new_unchecked(target_value)
        }
    }

    /// Translate a base-relative pointer to a `T` to a regular pointer.
    ///
    /// # Safety
    ///
    /// This operation is always safe, but using the resulting pointer is not.
    #[inline]
    #[must_use]
    pub fn base(self: Pin<&Self>) -> *const T {
        let base_address = ptr::from_ref::<Self>(self.get_ref());

        let base_address = base_address.addr();

        let &Self(relative_pointer, ..) = self.get_ref();

        relative_pointer.wrapping_byte_add(base_address)
    }
}

/// A base-relative mutable pointer to an instance of `T`.
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct RelPtrMut<T>(*mut T, marker::PhantomPinned);

impl<T> RelPtrMut<T> {
    /// Construct a new base-relative pointer from a regular pointer to an
    /// instance of `T`.
    ///
    /// The final value is written to a maybe-unintialized [`RelPtrMut`].
    #[inline]
    pub fn new(target_base: Pin<&mut MaybeUninit<Self>>, target_value: *mut T) -> Pin<&mut Self> {
        let base_address = target_base.as_ptr().addr();

        let target_relative = target_value.wrapping_byte_sub(base_address);

        // SAFETY: We initialize the pinned value and retrieve a mutable reference to
        // it, which we immediately pin. The provenance of the pointer provided is kept
        // verbatim in the pointer stored inside the structure.
        unsafe {
            let target_value = MaybeUninit::write(
                Pin::into_inner_unchecked(target_base),
                Self(target_relative, marker::PhantomPinned),
            );

            Pin::new_unchecked(target_value)
        }
    }

    /// Translate a base-relative pointer to a `T` to a regular pointer.
    ///
    /// # Safety
    ///
    /// This operation is always safe, but using the resulting pointer is not.
    #[inline]
    #[must_use]
    pub fn base(self: Pin<&Self>) -> *mut T {
        let base_address = ptr::from_ref::<Self>(self.get_ref());

        let base_address = base_address.addr();

        let &Self(relative_pointer, ..) = self.get_ref();

        relative_pointer.wrapping_byte_add(base_address)
    }
}

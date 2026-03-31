//! A memory-level swapchain for modifying critical system structures safely.
//!
//! This is intended for structures like:
//!
//! - Interrupt Tables
//! - Descriptor Tables (x86-specific)

use core::{mem::MaybeUninit, ops::Deref, ptr::NonNull};

use nekor_primitive::scalar::Scalar;

/// A swap mechanism between two distinct structures of the same type.
#[derive(Debug)]
pub struct Swap<S>(
    // NOTE(invariant): The `bool`, when casted to an integer, points to the
    // currently-active and initialized `S`.
    [MaybeUninit<S>; 2],
    bool,
)
where
    S: Scalar;

impl<S> Swap<S>
where
    S: Scalar,
{
    /// Create a new chain for the target structure.
    #[inline]
    pub const fn chain(target_value: S) -> Self {
        Self(
            [MaybeUninit::new(target_value), MaybeUninit::uninit()],
            false,
        )
    }
}

impl<S> Swap<S>
where
    S: Scalar,
{
    /// Determine the currently-active structure in this [`Swap`] mechanism.
    #[inline]
    pub const fn active<'a>(&'a self) -> &'a S {
        let &Self(ref structure_list, target_structure) = self;

        // SAFETY: A `bool` is always in-bound for a 2-element array.
        //
        // The underlying structure is initialized due to type invariants,
        // particularly, the boolean always indicates a valid structure.
        unsafe {
            NonNull::new_unchecked(structure_list.as_ptr() as *mut MaybeUninit<S>)
                .offset(target_structure as isize)
                .as_ref()
                .assume_init_ref()
        }
    }

    /// Determine the currently-active structure in this [`Swap`] mechanism, but
    /// in a mutable manner.
    #[inline]
    pub const fn active_mut<'a>(&'a mut self) -> &'a mut S {
        let &mut Self(ref mut structure_list, target_structure) = self;

        // SAFETY: A `bool` is always in-bound for a 2-element array.
        //
        // The underlying structure is initialized due to type invariants,
        // particularly, the boolean always indicates a valid structure.
        unsafe {
            NonNull::new_unchecked(structure_list.as_mut_ptr())
                .offset(target_structure as isize)
                .as_mut()
                .assume_init_mut()
        }
    }

    /// Determine the currently-inactive structure in this [`Swap`] mechanism.
    #[inline]
    pub const fn inactive<'a>(&'a self) -> &'a MaybeUninit<S> {
        let &Self(ref structure_list, target_structure) = self;

        // SAFETY: A `bool` is always in-bound for a 2-element array.
        unsafe {
            NonNull::new_unchecked(structure_list.as_ptr() as *mut MaybeUninit<S>)
                .offset(!target_structure as isize)
                .as_ref()
        }
    }

    /// Determine the currently-inactive structure in this [`Swap`] mechanism,
    /// but in a mutable manner.
    #[inline]
    pub const fn inactive_mut<'a>(&'a mut self) -> &'a mut MaybeUninit<S> {
        let &mut Self(ref mut structure_list, target_structure) = self;

        // SAFETY: A `bool` is always in-bound for a 2-element array.
        unsafe {
            NonNull::new_unchecked(structure_list.as_mut_ptr())
                .offset(!target_structure as isize)
                .as_mut()
        }
    }

    /// Access the leftwards-facing structure.
    #[inline]
    pub const fn left<'a>(&'a self) -> &'a MaybeUninit<S> {
        let &Self([ref left_structure, ..], ..) = self;

        left_structure
    }

    /// Access the rightwards-facing structure.
    #[inline]
    pub const fn right<'a>(&'a self) -> &'a MaybeUninit<S> {
        let &Self([.., ref right_structure], ..) = self;

        right_structure
    }

    /// Access the leftwards-facing structure, but in a mutable manner.
    #[inline]
    pub const fn left_mut<'a>(&'a mut self) -> &'a mut MaybeUninit<S> {
        let &mut Self([ref mut left_structure, ..], ..) = self;

        left_structure
    }

    /// Access the rightwards-facing structure, but in a mutable manner
    #[inline]
    pub const fn right_mut<'a>(&'a mut self) -> &'a mut MaybeUninit<S> {
        let &mut Self([.., ref mut right_structure], ..) = self;

        right_structure
    }

    /// Swap the currently-active structure with the inactive one.
    ///
    /// # Safety
    ///
    /// The currently-inactive structure must be correctly initialized.
    #[inline]
    pub const unsafe fn now<'a>(&'a mut self) -> &'a mut S {
        let &mut Self(.., ref mut target_state) = self;

        *target_state = !*target_state;

        Self::active_mut(self)
    }
}

impl<S> Deref for Swap<S>
where
    S: Scalar,
{
    type Target = S;

    #[inline]
    fn deref(&self) -> &Self::Target {
        Self::active(self)
    }
}

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
        Self([MaybeUninit::new(target_value), MaybeUninit::uninit()], false)
    }
}

impl<S> Swap<S>
where
    S: Scalar,
{
    /// Determine the currently-active structure in this [`Swap`] mechanism.
    #[inline]
    pub const fn active(&self) -> &S {
        let &Self(ref structure_list, target_structure) = self;

        // SAFETY: A `bool` is always in-bound for a 2-element array.
        //
        // The underlying structure is initialized due to type invariants,
        // particularly, the boolean always indicates a valid structure.
        unsafe {
            // FIXME(const): Make this use `get_unchecked_mut` when const-fn-stable.
            NonNull::new_unchecked(structure_list.as_ptr().cast_mut())
                .offset(target_structure as isize)
                .as_ref()
                .assume_init_ref()
        }
    }

    /// Determine the currently-active structure in this [`Swap`] mechanism, but
    /// in a mutable manner.
    #[inline]
    pub const fn active_mut(&mut self) -> &mut S {
        let &mut Self(ref mut structure_list, target_structure) = self;

        // SAFETY: A `bool` is always in-bound for a 2-element array.
        //
        // The underlying structure is initialized due to type invariants,
        // particularly, the boolean always indicates a valid structure.
        unsafe {
            // FIXME(const): Make this use `get_unchecked_mut` when const-fn-stable.
            NonNull::new_unchecked(structure_list.as_mut_ptr())
                .offset(target_structure as isize)
                .as_mut()
                .assume_init_mut()
        }
    }

    /// Determine the currently-inactive structure in this [`Swap`] mechanism.
    #[inline]
    pub const fn inactive(&self) -> &MaybeUninit<S> {
        let &Self(ref structure_list, target_structure) = self;

        // SAFETY: A `bool` is always in-bound for a 2-element array.
        unsafe {
            // FIXME(const): Make this use `get_unchecked_mut` when const-fn-stable.
            NonNull::new_unchecked(structure_list.as_ptr().cast_mut())
                .offset(!target_structure as isize)
                .as_ref()
        }
    }

    /// Determine the currently-inactive structure in this [`Swap`] mechanism,
    /// but in a mutable manner.
    #[inline]
    pub const fn inactive_mut(&mut self) -> &mut MaybeUninit<S> {
        let &mut Self(ref mut structure_list, target_structure) = self;

        // SAFETY: A `bool` is always in-bound for a 2-element array.
        unsafe {
            // FIXME(const): Make this use `get_unchecked_mut` when const-fn-stable.
            NonNull::new_unchecked(structure_list.as_mut_ptr())
                .offset(!target_structure as isize)
                .as_mut()
        }
    }

    /// Access the leftwards-facing structure.
    #[inline]
    pub const fn left(&self) -> &MaybeUninit<S> {
        let &Self([ref left_structure, ..], ..) = self;

        left_structure
    }

    /// Access the rightwards-facing structure.
    #[inline]
    pub const fn right(&self) -> &MaybeUninit<S> {
        let &Self([.., ref right_structure], ..) = self;

        right_structure
    }

    /// Access the leftwards-facing structure, but in a mutable manner.
    ///
    /// # Safety
    ///
    /// The returned [`MaybeUninit`] may refer to the currently-active
    /// structure. If it does, the caller must ensure that it contains a
    /// fully initialized, valid `S` before the returned mutable reference
    /// ceases to be used.
    ///
    /// Leaving the active structure uninitialized or containing an invalid
    /// value violates [`Swap`]'s invariants and may cause undefined
    /// behavior when the active structure is subsequently accessed.
    #[inline]
    pub const unsafe fn left_mut(&mut self) -> &mut MaybeUninit<S> {
        let &mut Self([ref mut left_structure, ..], ..) = self;

        left_structure
    }

    /// Access the rightwards-facing structure, but in a mutable manner.
    ///
    /// # Safety
    ///
    /// The returned [`MaybeUninit`] may refer to the currently-active
    /// structure. If it does, the caller must ensure that it contains a
    /// fully initialized, valid `S` before the returned mutable reference
    /// ceases to be used.
    ///
    /// Leaving the active structure uninitialized or containing an invalid
    /// value violates [`Swap`]'s invariants and may cause undefined
    /// behavior when the active structure is subsequently accessed.
    #[inline]
    pub const unsafe fn right_mut(&mut self) -> &mut MaybeUninit<S> {
        let &mut Self([.., ref mut right_structure], ..) = self;

        right_structure
    }

    /// Swap the currently-active structure with the inactive structure.
    ///
    /// # Safety
    ///
    /// The currently-inactive structure must be correctly initialized.
    #[inline]
    pub const unsafe fn now(&mut self) -> &mut S {
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

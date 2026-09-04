//! Mutual exclusion RAII guards for distinct types of access.

use core::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use crate::mutex::{
    Mutex,
    lock_state::{Observed, Waited},
};

/// A raw [`Mutex`] lock guard, which encompasses all the required information
/// for unlocking the lock afterwards.
///
/// # Remarks
///
/// This structure does **not** release the underlying lock after its scope has
/// ended, therefore, if not manually unlocked via [`RawMutexGuard::unlock`],
/// the associated [`Mutex`] will remain locked forever.
pub struct RawMutexGuard<'a, T>(&'a Mutex<T>, NonNull<T>, Waited);

impl<'a, T> RawMutexGuard<'a, T> {
    /// Construct a new [`RawMutexGuard`] for the target [`Mutex`] and
    /// [`Occupied`] slot.
    ///
    /// # Safety
    ///
    /// The [`Occupied`] slot must have been acquired from the same [`Mutex`].
    #[inline]
    pub const unsafe fn new(target_mutex: &'a Mutex<T>, waited_state: Waited) -> Self {
        let &Mutex { ref target_value, .. } = target_mutex;

        // SAFETY: The guard has been acquired for the mutex.
        let target_pointer = unsafe { NonNull::new(UnsafeCell::get(target_value)).unwrap_unchecked() };

        Self(target_mutex, target_pointer, waited_state)
    }

    /// Retrieve the guarded `T` through an immutable reference.
    #[inline]
    #[must_use]
    pub const fn value(&self) -> &T {
        let &Self(_, target_value, ..) = self;

        // SAFETY:
        //  Pointer has originally been sourced from a reference, therefore,
        // this is safe.
        unsafe { target_value.as_ref() }
    }

    /// Retrieve the guarded `T` through a mutable reference.
    #[inline]
    #[must_use]
    pub const fn value_mut(&mut self) -> &mut T {
        let &mut Self(_, mut target_value, ..) = self;

        // SAFETY:
        //  Pointer has originally been sourced from a reference, therefore,
        // this is safe.
        unsafe { target_value.as_mut() }
    }

    /// Unlock the underlying value, effectively consuming this
    /// [`RawMutexGuard`].
    #[inline]
    #[must_use]
    pub fn unlock(self) -> Observed {
        let Self(&Mutex { ref lock_state, .. }, .., target_occupied) = self;

        // SAFETY: The `Waited` was sourced from the same `LockState`.
        unsafe { lock_state.release(target_occupied) }
    }
}

/// A RAII-style guard type for [`Mutex`] acquisition.
#[repr(transparent)]
#[derive(Debug)]
pub struct Guard<'a, T>(
    // NOTE(invariant): Always is initialized, deinitialized for unlock at
    // `Drop`.
    MaybeUninit<RawMutexGuard<'a, T>>,
);

impl<'a, T> Guard<'a, T> {
    /// Construct a new mutex guard.
    #[inline]
    #[must_use]
    pub const fn new(target_value: RawMutexGuard<'a, T>) -> Self {
        Self(MaybeUninit::new(target_value))
    }

    /// Determine the inner [`RawMutexGuard`] employed by this regular
    /// [`Guard`].
    #[inline]
    #[must_use]
    pub const fn raw(&self) -> &RawMutexGuard<'a, T> {
        let &Self(ref target_value) = self;

        // SAFETY: [invariant] Always initialized before an unlock.
        unsafe { target_value.assume_init_ref() }
    }

    /// Determine the inner [`RawMutexGuard`] employed by this regular
    /// [`Guard`], but in a mutable manner.
    #[inline]
    #[must_use]
    pub const fn raw_mut(&mut self) -> &mut RawMutexGuard<'a, T> {
        let &mut Self(ref mut target_value) = self;

        // SAFETY: [invariant] Always initialized before an unlock.
        unsafe { target_value.assume_init_mut() }
    }
}

impl<T> Guard<'_, T> {
    /// Retrieve the guarded `T` value as an immutable reference.
    ///
    /// This is required in cases where one desires to prevent [`Deref`]'s
    /// artificial shortening of the `'a` lifetime.
    #[inline]
    #[must_use]
    pub const fn value(&self) -> &T {
        RawMutexGuard::value(Self::raw(self))
    }

    /// Retrieve the guarded `T` value as a mutable reference.
    ///
    /// This is required in cases where one desires to prevent [`DerefMut`]'s
    /// artificial shortening of the `'a` lifetime.
    #[inline]
    #[must_use]
    pub const fn value_mut(&mut self) -> &mut T {
        RawMutexGuard::value_mut(Self::raw_mut(self))
    }
}

impl<T> Deref for Guard<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        Self::value(self)
    }
}

impl<T> DerefMut for Guard<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        Self::value_mut(self)
    }
}

impl<T> Drop for Guard<'_, T> {
    #[inline]
    fn drop(&mut self) {
        let &mut Self(ref mut target_value) = self;

        // SAFETY: The underlying raw guard is always initialized previous to a
        // `Drop`.
        let target_guard = unsafe { target_value.assume_init_read() };

        let _: Observed = RawMutexGuard::unlock(target_guard);
    }
}

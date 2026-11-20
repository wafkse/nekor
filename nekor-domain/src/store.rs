//! Initialize shared static storage once for each value type and domain.
//!
//! [`Static`] coordinates access through the storage header. The thread that changes
//! `Uninitialized` to `Pending` owns initialization. It writes the value before publishing
//! `Initialized` with release ordering, and readers use acquire ordering before borrowing it.
//! If initialization unwinds, a guard resets the header so another attempt can proceed.
//!
//! ```text
//! Uninitialized --claim--> Pending --write and publish--> Initialized
//!       ^                    |
//!       +------unwind--------+
//! ```

use core::{cell::UnsafeCell, hint, mem::MaybeUninit, ptr::NonNull};

use crate::{
    domain::Domain,
    lowlevel::{Container, Header, InitializationStage},
    prelude::Preset,
};

/// Values that can be shared through static storage.
///
/// The blanket implementation accepts sized, `'static` types implementing [`Sync`].
/// [`Send`] is not required because the stored value is accessed through shared references.
pub trait Store: 'static + Sync {}

impl<T> Store for T where T: 'static + Sync {}

/// The result of a [`Static`]-based initialization operation
///
/// This represents two distinct outcomes of the operation: Defer to each
/// distinct variant for additional information.
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum Initialized<T>
where
    T: Store,
{
    /// The underlying [`Container`] was already initialized or was pending
    /// initialization.
    Already(&'static T, T),

    /// The underlying [`Container`] had to be initialized by the current thread
    /// of execution.
    Did(&'static T),
}

impl<T> Initialized<T>
where
    T: Store,
{
    /// Extract the reference to the value of type `T` contained in the
    /// [`Container`], regardless of whether it was already initialized or not.
    #[inline]
    pub const fn anyhow(&self) -> &'static T {
        let (&Self::Already(target_value, _) | &Self::Did(target_value)) = self;

        target_value
    }
}

/// Access static storage selected by a value type and [`Domain`].
///
/// [`Self::value`] uses a supplied value, [`Self::value_with`] runs a constructor on demand,
/// and [`Self::value_default`] uses [`Default`]. The `_in` forms select a domain explicitly.
/// Calls for the same type and domain share the initialized value, so the first successful
/// constructor determines its contents.
#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum Static {}

impl Static {
    /// Determine the maybe-uninitialized value of `T` pertaining to the default
    /// ([`Preset`]) [`Domain`].
    #[inline]
    #[must_use]
    pub fn raw_value<T>() -> &'static UnsafeCell<MaybeUninit<T>>
    where
        T: Store,
    {
        Self::raw_value_in::<T, Preset>()
    }

    /// Determine the maybe-uninitialized value of `T` pertaining to the
    /// [`Domain`] `D`.
    #[inline]
    #[must_use]
    pub fn raw_value_in<T, D>() -> &'static UnsafeCell<MaybeUninit<T>>
    where
        T: Store,
        D: Domain,
    {
        // SAFETY: `Container` is safe to share between threads.
        let container = unsafe { Container::<T>::address_in::<D>().as_ref() };

        let value_ptr = &raw const container.value_storage;

        // SAFETY: The pointer is guaranteed to be non-null, as it has been
        // derived from a valid reference.
        let value_ptr = unsafe { NonNull::new_unchecked(value_ptr.cast_mut()) };

        // SAFETY: The struct layout has been proven correct. Therefore, this
        // pointer to reference conversion is safe.
        unsafe {
            // FIXME: Use `as_uninit_ref` instead of casting directly to
            // `MaybeUninit` when its stable.
            value_ptr.as_ref()
        }
    }

    /// Determine the mutable maybe-uninitialized value of `T` pertaining to the
    /// default ([`Preset`]) [`Domain`].
    ///
    /// # Safety
    ///
    /// This method has the same safety considerations as
    /// [`Self::raw_value_mut_in`].
    #[inline]
    #[must_use]
    pub unsafe fn raw_value_mut<T>() -> &'static mut MaybeUninit<T>
    where
        T: Store,
    {
        // SAFETY: The safety considerations for this call have been guaranteed
        // by the caller.
        unsafe { Self::raw_value_mut_in::<T, Preset>() }
    }

    /// Determine the mutable maybe-uninitialized value of `T` pertaining to the
    /// [`Domain`] `D`.
    ///
    /// # Safety
    ///
    /// - The caller of this method must ensure that they **actually have** exclusive access to the
    ///   static variable of type `T`.
    #[inline]
    #[must_use]
    pub unsafe fn raw_value_mut_in<T, D>() -> &'static mut MaybeUninit<T>
    where
        T: Store,
        D: Domain,
    {
        // SAFETY: `Container` is safe to share between threads.
        let container = unsafe { Container::<T>::address_in::<D>().as_ref() };

        let value_ptr = &raw const container.value_storage;

        // SAFETY: The pointer is guaranteed to be non-null, as it has been
        // derived from a valid reference.
        let mut value_ptr = unsafe { NonNull::new_unchecked(value_ptr.cast_mut()) };

        // SAFETY: The struct layout has been proven correct. Therefore, this
        // pointer to reference conversion is safe.
        unsafe {
            // FIXME: Use `as_uninit_ref` instead of casting directly to
            // `MaybeUninit` when its stable.
            value_ptr.as_mut().get_mut()
        }
    }

    /// Determine the initialization [`Header`] of `T` pertaining to the default
    /// ([`Preset`]) [`Domain`].
    ///
    /// # Safety
    ///
    /// This method has the same safety compromises as [`Self::raw_header_in`].
    #[inline]
    #[must_use]
    pub unsafe fn raw_header<T>() -> &'static Header
    where
        T: Store,
    {
        // SAFETY: All safety compromises are met by the caller of this
        // function.
        unsafe { Self::raw_header_in::<T, Preset>() }
    }

    /// Determine the initialization [`Header`] of `T` pertaining to the
    /// [`Domain`] `D`.
    ///
    /// # Safety
    ///
    ///  - The yielded [`Header`] must only be subjected to atomic loads, i.e, never mutated.
    #[inline]
    #[must_use]
    pub unsafe fn raw_header_in<T, D>() -> &'static Header
    where
        T: Store,
        D: Domain,
    {
        // SAFETY: The reference is valid and is correct to share between
        // threads.
        let &Container { ref value_header, .. } = unsafe { Container::<T>::address_in::<D>().as_ref() };

        value_header
    }

    /// Determine the initialization [`InitializationStage`] of `T` pertaining to the default
    /// ([`Preset`]) [`Domain`].
    #[inline]
    #[must_use]
    pub fn raw_stage<T>() -> InitializationStage
    where
        T: Store,
    {
        Self::raw_stage_in::<T, Preset>()
    }

    /// Determine the initialization [`InitializationStage`] of `T` pertaining to the
    /// [`Domain`] `D`.
    #[inline]
    #[must_use]
    pub fn raw_stage_in<T, D>() -> InitializationStage
    where
        T: Store,
        D: Domain,
    {
        // SAFETY: The reference is valid and is correct to share between
        // threads.
        let &Container { ref value_header, .. } = unsafe { Container::<T>::address_in::<D>().as_ref() };

        value_header.load()
    }
}

impl Static {
    /// Retrieve the value of type `T` contained in the default ([`Preset`])
    /// [`Domain`].
    ///
    /// This initializes the value using the specified value.
    ///
    /// This yields the provided `T` back if the static variable happened to be
    /// already initialized.
    #[inline]
    pub fn value<T>(target_value: T) -> Initialized<T>
    where
        T: Store,
    {
        Self::value_in::<T, Preset>(target_value)
    }

    /// Retrieve the value of type `T` contained in the [`Domain`] `D`.
    ///
    /// This initializes the value using the specified value.
    ///
    /// This yields the provided `T` back if the static variable happened to be
    /// already initialized.
    #[inline]
    pub fn value_in<T, D>(target_value: T) -> Initialized<T>
    where
        T: Store,
        D: Domain,
    {
        let mut target_value = Some(target_value);

        let target_closure = || {
            // SAFETY:
            //  The `Option` is always guaranteed to be `Some` at this point.
            //  This won't be called multiple times, as `value_with_in` takes a
            // `FnOnce`.
            unsafe { target_value.take().unwrap_unchecked() }
        };

        let target_ref = Self::value_with_in::<T, D, _>(target_closure);

        match target_value {
            Some(target_value) => Initialized::Already(target_ref, target_value),
            None => Initialized::Did(target_ref),
        }
    }

    /// Retrieve the value of type `T` contained in the default ([`Preset`])
    /// [`Domain`].
    ///
    /// This method will initialize the static as needed using the provided
    /// closure.
    ///
    /// # Panics
    ///
    /// A constructor panic propagates to the caller. If it unwinds, the initialization guard
    /// resets the header to `Uninitialized`, allowing another thread to try again.
    ///
    /// # Notes
    ///
    /// Recursively requesting the same value type and domain waits for the current
    /// initialization to finish, so such recursion deadlocks. A constructor must not depend
    /// on its own pending static value.
    #[inline]
    pub fn value_with<T, F>(target_predicate: F) -> &'static T
    where
        T: Store,
        F: FnOnce() -> T,
    {
        Self::value_with_in::<T, Preset, F>(target_predicate)
    }

    /// Retrieve the value of type `T` contained in the [`Domain`] `D`.
    ///
    /// This method will initialize the static as needed using the provided
    /// closure.
    ///
    /// # Panics
    ///
    /// A constructor panic propagates to the caller. If it unwinds, the initialization guard
    /// resets the header to `Uninitialized`, allowing another thread to try again.
    ///
    /// # Notes
    ///
    /// Recursively requesting the same value type and domain waits for the current
    /// initialization to finish, so such recursion deadlocks. A constructor must not depend
    /// on its own pending static value.
    pub fn value_with_in<T, D, F>(target_predicate: F) -> &'static T
    where
        T: Store,
        D: Domain,
        F: FnOnce() -> T,
    {
        /// Reset a pending initialization if the constructor unwinds.
        /// Successful initialization disarms the guard before publishing the value.
        // NOTE(invariant): A false disarm flag means dropping the guard must restore the header to
        // the uninitialized stage.
        struct Guard<'a>(&'a Header, bool);

        impl Drop for Guard<'_> {
            #[inline]
            fn drop(&mut self) {
                let &mut Self(target_header, guard_disable) = self;

                if !guard_disable {
                    // SAFETY: The stored stage is not the `Initialized` variant.
                    unsafe { target_header.store(InitializationStage::Uninitialized) };
                }
            }
        }

        // SAFETY: The reference is valid and is correct to share between
        // threads.
        let &Container { ref value_header, .. } = unsafe { Container::<T>::address_in::<D>().as_ref() };

        // SAFETY: The to-migrate stage is not the `Initialized` variant.
        let target_migrate =
            unsafe { value_header.migrate(InitializationStage::Uninitialized, InitializationStage::Pending) };

        match target_migrate {
            None => {
                // SAFETY: We have ensured exclusive access to the underlying
                // storage due to the `compare_exchange`'s success.
                let target_value = unsafe { Self::raw_value_mut_in::<T, D>() };

                let mut target_guard: Guard<'_> = Guard(value_header, false);

                let target_value: &T = target_value.write(target_predicate());

                // NOTE(drop): No failure point remains, so explicitly disarm
                // the `Guard` before it is dropped.
                let &mut Guard(.., ref mut guard_disable) = &mut target_guard;

                *guard_disable = true;

                // SAFETY: The value associated has been initialized.
                unsafe { value_header.store(InitializationStage::Initialized) };

                target_value
            },
            Some(target_stage) => match target_stage {
                InitializationStage::Pending => {
                    loop {
                        match value_header.load() {
                            InitializationStage::Initialized => {
                                let target_value = Self::raw_value_in::<T, D>();

                                // SAFETY: The value is guaranteed to have been
                                // initialized due to an initialization signal.
                                let target_value =
                                    unsafe { target_value.get().as_ref().unwrap_unchecked().assume_init_ref() };

                                break target_value;
                            },
                            InitializationStage::Uninitialized => {
                                // NOTE: The initializing thread has possibly
                                // panicked, so we just retry.
                                break Self::value_with_in::<T, D, F>(target_predicate);
                            },
                            InitializationStage::Pending => hint::spin_loop(),
                        }
                    }
                },
                InitializationStage::Initialized => {
                    let target_value = Self::raw_value_in::<T, D>();

                    // SAFETY: The value is guaranteed to have been initialized
                    // due to an initialization signal.
                    unsafe { target_value.get().as_ref().unwrap_unchecked().assume_init_ref() }
                },
                InitializationStage::Uninitialized => unreachable!(),
            },
        }
    }

    /// Retrieve the value of type `T` contained in the default ([`Preset`])
    /// [`Domain`], using [`Default`] for lazy initialization.
    #[inline]
    #[must_use]
    pub fn value_default<T>() -> &'static T
    where
        T: Store + Default,
    {
        Self::value_default_in::<T, Preset>()
    }

    /// Retrieve the value of type `T` contained in the [`Domain`] `D`, using
    /// [`Default`] for lazy initialization.
    #[inline]
    #[must_use]
    pub fn value_default_in<T, D>() -> &'static T
    where
        T: Store + Default,
        D: Domain,
    {
        Self::value_with_in::<T, D, _>(|| T::default())
    }
}

#[cfg(test)]
mod tests {
    use core::ptr;

    use crate::store::{Initialized, Static};

    #[test]
    fn testme() {
        let st: &'static usize = Static::value(1337).anyhow();
        let st2: &'static usize = Initialized::anyhow(&Static::value(6767)); // haha funny, kms

        println!("{:?}, {:?}, {st2:?}", ptr::from_ref(st), ptr::from_ref(st2));
    }
}

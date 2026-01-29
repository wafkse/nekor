//! An Application Programming Interface (API) to instantiate generic static
//! values in a sound manner.
//!
//! The type of utmost importance in this module is the [`Static`] type. Defer
//! to its documentation for further guidance.

use core::{
    cell::UnsafeCell,
    hint,
    mem::{self, MaybeUninit},
    ptr::NonNull,
};

use crate::{
    arch::{Container, Header, InitializationStage},
    domain::Domain,
    prelude::Preset,
};

/// A trait that expresses the bare minimum a static variable (be it `non-mut`
/// or `mut`) must implement for it being sound to store it statically.
///
/// This is implemented automatically, therefore, if your type is both `'static`
/// and [`Send`], it should be good to go.
///
/// Note that this denotes an implicit [`Sized`] requirement.
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

/// An uninhabited umbrella type for static intialization.
///
/// A brief description of the associated functions present in this type are the
/// following:
///
/// - [`Static::value<T>`]: Retrieve a [`Default`]-initialized `&'static T`
///   stored in the default ([`Preset`]) domain.
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
    /// - The caller of this method must ensure that they **actually have**
    ///   exclusive access to the static variable of type `T`.
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
    ///  - The yielded [`Header`] must only be subjected to atomic loads, i.e,
    ///    never mutated.
    #[inline]
    #[must_use]
    pub unsafe fn raw_header_in<T, D>() -> &'static Header
    where
        T: Store,
        D: Domain,
    {
        // SAFETY: The reference is valid and is correct to share between
        // threads.
        let Container { value_header, .. } = unsafe { Container::<T>::address_in::<D>().as_ref() };

        value_header
    }

    /// Determine the initialization [`Stage`] of `T` pertaining to the default
    /// ([`Preset`]) [`Domain`].
    #[inline]
    #[must_use]
    pub fn raw_stage<T>() -> InitializationStage
    where
        T: Store,
    {
        Self::raw_stage_in::<T, Preset>()
    }

    /// Determine the initialization [`Stage`] of `T` pertaining to the
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
        let Container { value_header, .. } = unsafe { Container::<T>::address_in::<D>().as_ref() };

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
    /// # Behavior
    ///
    /// This function will only panic if the provided closure `F` panics.
    ///
    /// If a panic is encountered during initialization, the state machine will
    /// be reset to the uninitialized state.
    ///
    /// Additionally, there is risk of deadlock if the closure attempts to fetch
    /// a `T` in the same [`Domain`].
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
    /// # Behavior
    ///
    /// This function will only panic if the provided closure `F` panics.
    ///
    /// If a panic is encountered during initialization, the state machine will
    /// be reset to the uninitialized state.
    ///
    /// Additionally, there is risk of deadlock if the closure attempts to fetch
    /// a `T` in the same [`Domain`].
    pub fn value_with_in<T, D, F>(target_predicate: F) -> &'static T
    where
        T: Store,
        D: Domain,
        F: FnOnce() -> T,
    {
        /// A guard type to signal an initialization failure on panic.
        ///
        /// # Remarks
        ///
        /// Remember to [`forget`] this guard once the risk of failure has been
        /// superceded.
        ///
        /// [`forget`]: core::mem::forget
        #[repr(transparent)]
        struct Guard<'a>(&'a Header);

        impl Drop for Guard<'_> {
            #[inline]
            fn drop(&mut self) {
                let &mut Self(target_header) = self;

                // SAFETY: The stored stage is not the `Initialized` variant.
                unsafe { target_header.store(InitializationStage::Uninitialized) };
            }
        }

        // SAFETY: The reference is valid and is correct to share between
        // threads.
        let Container { value_header, .. } = unsafe { Container::<T>::address_in::<D>().as_ref() };

        let target_migrate =
            // SAFETY: The to-migrate stage is not the `Initialized` variant.
            unsafe {
            value_header.migrate(
                InitializationStage::Uninitialized,
                InitializationStage::Pending,
            )
        };

        match target_migrate {
            None => {
                // SAFETY: We have ensured exclusive access to the underlying
                // storage due to the `compare_exchange`'s success.
                let target_value = unsafe { Self::raw_value_mut_in::<T, D>() };

                let target_guard: Guard<'_> = Guard(value_header);

                let target_value: &T = target_value.write(target_predicate());

                // NOTE(drop): No failure point remains, so explicitly prohibit
                // the `Guard` from being dropped.
                mem::forget(target_guard);

                // SAFETY: The value associated has been initialized.
                unsafe { value_header.store(InitializationStage::Initialized) };

                target_value
            }
            Some(target_stage) => match target_stage {
                InitializationStage::Pending => {
                    loop {
                        match value_header.load() {
                            InitializationStage::Initialized => {
                                let target_value = Self::raw_value_in::<T, D>();

                                // SAFETY: The value is guaranteed to have been
                                // initialized due to an initialization signal.
                                let target_value = unsafe {
                                    target_value
                                        .get()
                                        .as_ref()
                                        .unwrap_unchecked()
                                        .assume_init_ref()
                                };

                                break target_value;
                            }
                            InitializationStage::Uninitialized => {
                                // NOTE: The initializing thread has possibly
                                // panicked, so we just retry.
                                break Self::value_with_in::<T, D, F>(target_predicate);
                            }
                            InitializationStage::Pending => hint::spin_loop(),
                        }
                    }
                }
                InitializationStage::Initialized => {
                    let target_value = Self::raw_value_in::<T, D>();

                    // SAFETY: The value is guaranteed to have been initialized
                    // due to an initialization signal.
                    unsafe {
                        target_value
                            .get()
                            .as_ref()
                            .unwrap_unchecked()
                            .assume_init_ref()
                    }
                }
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
    use crate::store::{Initialized, Static};

    #[test]
    fn testme() {
        let st: &'static usize = Static::value(18273).anyhow();
        let st2: &'static usize = Initialized::anyhow(&Static::value(18273));

        println!("{:?}, {:?}", st as *const _, st2 as *const _);
    }
}

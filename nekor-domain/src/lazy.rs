//! Smart pointers for lazy static storage initialization.

use core::{
    marker,
    ops::Deref,
    ptr::{self, NonNull},
    sync::atomic::{AtomicPtr, Ordering},
};

use crate::{
    domain::Domain,
    prelude::{Preset, Static, Store},
};

/// A lazy smart pointer to the static item `T` under the `D` [`Domain`].
///
/// This will initialize the underlying reference automatically using `T`'s
/// [`Default`] implementation.
///
/// Even though this has a `'static` reference to `T`, the lifetime is erased in
/// the [`Deref`] implementation.
///
/// To access the underlying value, see [`Lazy::value`].
///
/// # Layout
///
/// This has the same exact layout as [`AtomicPtr`].
#[repr(transparent)]
pub struct Lazy<T, D = Preset>(
    // NOTE(invariant): The `AtomicPtr` must be a valid pointer with a lifetime
    // of `'static`.
    AtomicPtr<T>,
    // NOTE(variance): This is explicitly invariant for `D`.
    marker::PhantomData<fn() -> D>,
)
where
    T: Store + Default,
    D: Domain;

impl<T, D> Lazy<T, D>
where
    T: Store + Default,
    D: Domain,
{
    /// Construct a new [`Lazy`] pointer to a `T`.
    #[inline]
    #[must_use]
    pub const fn pointer() -> Self {
        Self(AtomicPtr::new(ptr::null_mut()), marker::PhantomData)
    }

    /// Access the pointer to the underlying value, lazily initializing it as
    /// needed.
    #[inline]
    pub fn value(&self) -> &'static T {
        let &Self(ref ptr, ..) = self;

        if let Some(target_value) = NonNull::new(ptr.load(Ordering::Relaxed)) {
            // SAFETY: The pointer is valid due to type invariants.
            unsafe { target_value.as_ref() }
        } else {
            let target_value = Static::value_default_in::<T, D>();

            match ptr.compare_exchange(
                ptr::null_mut::<T>(),
                ptr::from_ref::<T>(target_value).cast_mut(),
                // NOTE(atomic): No ordering requirements, any posterior
                // thread that reads a stale (null, there is no
                // re-initialization) value causes no harm.
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                // NOTE(atomic): We do not care whether we failed to update
                // the `AtomicPtr` or not, as we already have our `&'static
                // T` reference.
                Ok(..) | Err(..) => target_value,
            }
        }
    }

    /// Determine the cached pointer to the static storage.
    ///
    /// This will not eagerly initialize the variable, hence the [`Option`]
    /// output.
    #[inline]
    pub fn cache(&self) -> Option<&'static T> {
        let &Self(ref ptr, ..) = self;

        match NonNull::new(ptr.load(Ordering::Relaxed)) {
            // SAFETY: The pointer is valid due to type invariants.
            Some(ptr) => Some(unsafe { ptr.as_ref() }),
            None => None,
        }
    }

    /// Duplicate this [`Lazy`] smart pointer, effectively sharing the
    /// downstream static value.
    ///
    /// # Remarks
    ///
    /// This tries to load an already-existing reference to the target value.
    ///
    /// For a `const`-compatible alternative, simply instantiate another
    /// [`Lazy`] through the use of [`Lazy::pointer`] for the same `(T, D)`
    /// pair.
    #[inline]
    #[must_use]
    pub fn duplicate(&self) -> Self {
        let &Self(ref ptr, ..) = self;

        Self(
            // NOTE(atomic): Prefer to not require stronger ordering here, as a
            // stale pointer would be initialized regardless.
            AtomicPtr::new(ptr.load(Ordering::Relaxed)),
            marker::PhantomData,
        )
    }
}

impl<T, D> Deref for Lazy<T, D>
where
    T: Store + Default,
    D: Domain,
{
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        Self::value(self)
    }
}

impl<T, D> Clone for Lazy<T, D>
where
    T: Store + Default,
    D: Domain,
{
    #[inline]
    fn clone(&self) -> Self {
        Lazy::duplicate(self)
    }
}

impl<T, D> Default for Lazy<T, D>
where
    T: Store + Default,
    D: Domain,
{
    #[inline]
    fn default() -> Self {
        Self::pointer()
    }
}

// SAFETY: Access to `T` is thread-safe, therefore, access to `Erased` is too.
unsafe impl<T, D> Sync for Lazy<T, D>
where
    T: Store + Default,
    D: Domain,
{
}

/// A lazy smart pointer for a type `T` whose [`Domain`] has been type-erased.
#[derive(Debug)]
pub struct Erased<T>(
    // NOTE(invariant): `fn pointer` is a monomorphization of
    // `Static::value_default_in` for pair `(T, D)`
    fn() -> &'static T,
    AtomicPtr<T>,
)
where
    T: Store + Default;

impl<T> Erased<T>
where
    T: Store + Default,
{
    #[inline]
    pub const fn pointer<D>() -> Self
    where
        D: Domain,
    {
        Self(Static::value_default_in::<T, D>, AtomicPtr::new(ptr::null_mut()))
    }

    /// Access the pointer to the underlying `T`, lazily initializing it as
    /// needed.
    #[inline]
    pub fn value(&self) -> &'static T {
        let &Self(value_default_in, ref ptr) = self;

        if let Some(target_value) = NonNull::new(ptr.load(Ordering::Relaxed)) {
            // SAFETY: The pointer is valid due to type invariants.
            unsafe { target_value.as_ref() }
        } else {
            let target_value = value_default_in();

            match ptr.compare_exchange(
                ptr::null_mut::<T>(),
                ptr::from_ref::<T>(target_value).cast_mut(),
                // NOTE(atomic): No ordering requirements, any posterior
                // thread that reads a stale (null, there is no
                // re-initialization) value causes no harm.
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                // NOTE(atomic): We do not care whether we failed to update
                // the `AtomicPtr` or not, as we already have our `&'static
                // T` reference.
                Ok(..) | Err(..) => target_value,
            }
        }
    }

    /// Determine the cached pointer to the static storage.
    ///
    /// This will not eagerly initialize the variable, hence the [`Option`]
    /// output.
    #[inline]
    pub fn cache(&self) -> Option<&'static T> {
        let &Self(.., ref ptr) = self;

        match NonNull::new(ptr.load(Ordering::Relaxed)) {
            // SAFETY: The pointer is valid due to type invariants.
            Some(ptr) => Some(unsafe { ptr.as_ref() }),
            None => None,
        }
    }

    /// Duplicate this [`Lazy`] smart pointer, effectively sharing the
    /// downstream static value.
    ///
    /// # Remarks
    ///
    /// This tries to load an already-existing reference to the target value.
    ///
    /// For a `const`-compatible alternative, simply instantiate another
    /// [`Lazy`] through the use of [`Lazy::pointer`] for the same `(T, D)`
    /// pair.
    #[inline]
    #[must_use]
    pub fn duplicate(&self) -> Self {
        let &Self(value_default_in, ref ptr) = self;

        Self(
            value_default_in,
            // NOTE(atomic): Prefer to not require stronger ordering here, as a
            // stale pointer would be initialized regardless.
            AtomicPtr::new(ptr.load(Ordering::Relaxed)),
        )
    }

    /// Determine the constructor function associated to this [`Erased`].
    #[inline]
    pub const fn constructor(&self) -> fn() -> &'static T {
        let &Self(target_constructor, ..) = self;

        target_constructor
    }
}

impl<T> Deref for Erased<T>
where
    T: Store + Default,
{
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        Self::value(self)
    }
}

impl<T> Clone for Erased<T>
where
    T: Store + Default,
{
    #[inline]
    fn clone(&self) -> Self {
        Erased::duplicate(self)
    }
}

// SAFETY: Access to `T` is thread-safe, therefore, access to `Erased` is too.
unsafe impl<T> Sync for Erased<T> where T: Store + Default {}

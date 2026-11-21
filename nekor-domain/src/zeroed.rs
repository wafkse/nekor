//! Static storage for types that can be zero-initialized.

use core::{
    cell::UnsafeCell,
    hint, marker,
    mem::MaybeUninit,
    ptr::NonNull,
    sync::atomic::{
        AtomicI8, AtomicI16, AtomicI32, AtomicI64, AtomicIsize, AtomicU8, AtomicU16, AtomicU32, AtomicU64, AtomicUsize,
    },
};

use crate::{
    domain::{Adapter, Domain},
    lowlevel::{Container, InitializationStage},
    prelude::Preset,
    store::{Static, Store},
};

/// A marker trait that indicates that a type is zero-initializable.
///
/// # Safety
///
/// An all-zero byte representation must be an initialized value of the implementing type.
/// Any additional invariants of the type must also hold for that value.
pub unsafe trait Zeroable: Store {}

/// Automatically implement [`Zeroable`] for a list of compatible types.
macro_rules! zeroable {
    ($($target_type: ty),+) => {
        $(
            // SAFETY: Every listed integer and atomic integer has an initialized zero value.
            // Zeroing its storage therefore satisfies the `Zeroable` contract.
            unsafe impl Zeroable for $target_type {}
        )+
    };
}

zeroable!(u8, u16, u32, u64, u128, usize);
zeroable!(i8, i16, i32, i64, i128, isize);

zeroable!(AtomicU8, AtomicU16, AtomicU32, AtomicU64, AtomicUsize);
zeroable!(AtomicI8, AtomicI16, AtomicI32, AtomicI64, AtomicIsize);

// SAFETY: If a type `Z` can be zero-initialized, a fixed-size array of `Z` also
// can be, as any inter-element padding being zeroed out causes no harm.
unsafe impl<Z, const N: usize> Zeroable for [Z; N] where Z: Zeroable {}

/// Access static values whose all-zero representation is initialized.
///
/// [`Self::value`] and [`Self::value_in`] use the same storage and initialization header as
/// [`Static`]. The `explicit` forms select separate storage through an internal domain, so
/// their zeroed values cannot be replaced by a `Static` constructor for the original domain.
pub enum Zeroed {}

impl Zeroed {
    /// Retrieve the value of type `T` contained in the default ([`Preset`])
    /// [`Domain`].
    ///
    /// Claim the initially zeroed storage and publish it through the shared initialization header.
    #[inline]
    #[must_use]
    pub fn value<T>() -> &'static T
    where
        T: Zeroable,
    {
        Self::value_in::<T, Preset>()
    }

    /// Retrieve the value of type `T` contained in the [`Domain`] `D`.
    ///
    /// Claim the initially zeroed storage and publish it through the shared initialization header.
    #[inline]
    #[must_use]
    pub fn value_in<T, D>() -> &'static T
    where
        T: Zeroable,
        D: Domain,
    {
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
                let target_value = unsafe { Static::raw_value_mut_in::<T, D>() };

                // SAFETY: All memory reserved by the `arch` module is
                // guaranteed to be zero-initialized, and since `T` implements
                // the `Zeroable` trait, this is safe.
                let target_value: &T = unsafe { target_value.assume_init_ref() };

                // SAFETY: The value associated has been initialized.
                unsafe { value_header.store(InitializationStage::Initialized) };

                target_value
            },
            Some(target_stage) => match target_stage {
                InitializationStage::Pending => {
                    loop {
                        match value_header.load() {
                            InitializationStage::Initialized => {
                                let target_value = Static::raw_value_in::<T, D>();

                                // SAFETY: The value is guaranteed to have been
                                // initialized due to an initialization signal.
                                let target_value =
                                    unsafe { target_value.get().as_ref().unwrap_unchecked().assume_init_ref() };

                                break target_value;
                            },
                            // TODO: Add Backoff type here.
                            InitializationStage::Pending => hint::spin_loop(),
                            // NOTE: There is no initializing thread panic risk
                            // for default zero-initialization.
                            InitializationStage::Uninitialized => unreachable!(),
                        }
                    }
                },
                InitializationStage::Initialized => {
                    let target_value = Static::raw_value_in::<T, D>();

                    // SAFETY: The value is guaranteed to have been initialized
                    // due to an initialization signal.
                    unsafe { target_value.get().as_ref().unwrap_unchecked().assume_init_ref() }
                },
                InitializationStage::Uninitialized => unreachable!(),
            },
        }
    }
}

/// A separate storage identity for values initialized by their zero representation.
///
/// The explicit accessors use this private domain without updating an initialization header.
/// `Static` calls for the original domain select different storage and cannot overwrite it.
enum AlwaysZeroed<D>
where
    D: Domain,
{
    /// A variant to preserve the uninhabited property of the type due to the
    /// inclusion of another uninhabited type.
    #[doc(hidden)]
    __Variant(Preset, marker::PhantomData<fn() -> D>),
}

/// The [`Adapter`] transparent type for the [`AlwaysZeroed`] domain.
#[repr(transparent)]
// NOTE(invariant): The adapter is transparent over `T` while `D` contributes only domain identity.
struct ForZeroed<T, D>(T, marker::PhantomData<fn() -> D>)
where
    T: Store,
    D: Domain;

// SAFETY: `ForZeroed` is transparent over `T`.
unsafe impl<T, D> Adapter for ForZeroed<T, D>
where
    T: Store,
    D: Domain,
{
    type Target = T;
}

impl<D> Domain for AlwaysZeroed<D>
where
    D: Domain,
{
    type Adapter<T>
        = ForZeroed<T, D>
    where
        T: Store;
}

impl Zeroed {
    /// Retrieve an explicitly-zeroed `T` contained in the default ([`Preset`])
    /// [`Domain`].
    ///
    /// # Remarks
    ///
    /// Note that any value stored here is disjoint from any other storage type,
    /// even if contained within the same domain. In other words, other
    /// access patterns such as [`Static`] will not be able to access the same
    /// storage this method provides.
    #[inline]
    #[must_use]
    pub fn explicit<T>() -> &'static T
    where
        T: Zeroable,
    {
        Self::explicit_in::<T, Preset>()
    }

    /// Retrieve an explicitly-zeroed `T` contained in the [`Domain`] `D`.
    ///
    /// # Remarks
    ///
    /// Note that any value stored here is disjoint from any other storage type,
    /// even if contained within the same domain. In other words, other
    /// access patterns such as [`Static`] will not be able to access the same
    /// storage this method provides.
    #[inline]
    #[must_use]
    pub fn explicit_in<T, D>() -> &'static T
    where
        T: Zeroable,
        D: Domain,
    {
        // SAFETY: This private domain selects storage that starts zeroed and is never
        // initialized through the ordinary `Static` API. `Zeroable` makes those bytes an
        // initialized `T`. `UnsafeCell` and `MaybeUninit` preserve its layout, and the cast
        // creates only a shared reference, so mutable access still requires `T`'s own rules.
        unsafe {
            NonNull::<UnsafeCell<MaybeUninit<T>>>::from_ref(Static::raw_value_in::<T, AlwaysZeroed<D>>())
                .cast::<T>()
                .as_ref()
        }
    }
}

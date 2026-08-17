//! Static storage for types that can be zero-initialized.

use core::{
    cell::UnsafeCell,
    hint, marker,
    mem::MaybeUninit,
    ptr::NonNull,
    sync::atomic::{
        AtomicI8, AtomicI16, AtomicI32, AtomicI64, AtomicIsize, AtomicU8, AtomicU16, AtomicU32,
        AtomicU64, AtomicUsize,
    },
};

use crate::{
    arch::{Container, InitializationStage},
    domain::{Adapter, Domain},
    prelude::Preset,
    store::{Static, Store},
};

/// A marker trait that indicates that a type is zero-initializable.
///
/// # Safety
///
/// As aforementioned, the type must be able to initialize from raw zeroed
/// bytes.
pub unsafe trait Zeroable: Store {}

/// Automatically implement [`Zeroable`] for a list of compatible types.
macro_rules! zeroable {
    ($($target_type: ty),+) => {
        $(
            // SAFETY: Not a concern, as the macro is private to this module.
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

/// An uninhabited umbrella type for static intialization that requires no
/// type-specific initialization.
///
/// This is exposed as an alternative to regular [`Static`] usage when the
/// static type is able to initialize from zeroed memory. However, since
/// [`Store`] itself is a supertrait of [`Zeroable`], [`Static`] may as well be
/// used for accessing the same `(T, D)` pair.
pub enum Zeroed {}

impl Zeroed {
    /// Retrieve the value of type `T` contained in the default ([`Preset`])
    /// [`Domain`].
    ///
    /// This lazily initializes the value using the specified value.
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
    /// This lazily initializes the value using the specified value.
    #[inline]
    #[must_use]
    pub fn value_in<T, D>() -> &'static T
    where
        T: Zeroable,
        D: Domain,
    {
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
                let target_value = unsafe { Static::raw_value_mut_in::<T, D>() };

                // SAFETY: All memory reserved by the `arch` module is
                // guaranteed to be zero-initialized, and since `T` implements
                // the `Zeroable` trait, this is safe.
                let target_value: &T = unsafe { target_value.assume_init_ref() };

                // SAFETY: The value associated has been initialized.
                unsafe { value_header.store(InitializationStage::Initialized) };

                target_value
            }
            Some(target_stage) => match target_stage {
                InitializationStage::Pending => {
                    loop {
                        match value_header.load() {
                            InitializationStage::Initialized => {
                                let target_value = Static::raw_value_in::<T, D>();

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
                            // TODO: Add Backoff type here.
                            InitializationStage::Pending => hint::spin_loop(),
                            // NOTE: There is no initializing thread panic risk
                            // for default zero-initialization.
                            InitializationStage::Uninitialized => unreachable!(),
                        }
                    }
                }
                InitializationStage::Initialized => {
                    let target_value = Static::raw_value_in::<T, D>();

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
}

/// A **super** [`Domain`] over `D` where the underlying static value is
/// zero-cost to initialize.
///
/// In other words, all values stored under this [`Domain`] are always
/// intrinsically initialized.
///
/// Albeit this may seem unsound at first, behavior can be:
///
/// - The value is zero-initialized at program start. As known, this is enforced
///   by the [`Zeroable`] unsafe trait.
/// - If the value is non-[`Freeze`], it can be mutated, but the value **always
///   remains initialized** during the lifetime of the program.
///
/// Furthermore, all static values stored through this [`Domain`] will not be
/// able to be accessed by the regular [`Static`] interface. Access to values is
/// granted through [`Zeroed::explicit`] and [`Zeroed::explicit_in`].
///
/// # Internal Remarks
///
/// `explicit{,_in}` variants do not properly mark the header as initialized as
/// a small optimization.
///
/// [`Freeze`]: marker::Freeze
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
struct ForZeroed<T, D>(
    // NOTE(invariant): Must be `transparent` over the `T` field.
    T,
    marker::PhantomData<fn() -> D>,
)
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
        // SAFETY:
        //
        // Both `UnsafeCell` and `MaybeUninit` are `repr(transparent)`.
        // No aliasing issues occur due to the fact that the reference
        // mutability is unchanged.
        //
        // Safe to wrap out of the `MaybeUninit` due to the fact that the
        // underlying storage is zero-initialized and `T` implements `Zeroable`.
        unsafe {
            NonNull::<UnsafeCell<MaybeUninit<T>>>::from_ref(Static::raw_value_in::<
                T,
                AlwaysZeroed<D>,
            >())
            .cast::<T>()
            .as_ref()
        }
    }
}

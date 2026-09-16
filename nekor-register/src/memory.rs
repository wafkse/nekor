#![expect(unsafe_code, reason = "this module implements raw volatile register memory access")]
//! Memory-mapped register addresses and volatile access.

use core::{mem, ptr};

use nekor_primitive::scalar::Scalar;

/// Private sealing implementation for [`Volatile`].
mod private {
    /// A trait to act as a seal supertrait for the [`Volatile`] trait.
    ///
    /// [`Volatile`]: super::Volatile
    pub trait Sealed {}
}

/// A trait for types that can be read in a volatile manner.
///
/// # Remarks
///
/// This trait is sealed and is only implemented for a select set of types.
pub trait Volatile: Scalar + private::Sealed {}

/// A macro to implement the [`Volatile`] trait for a given type.
macro_rules! volatile {
    () => {};
    (
        $($target_type:ty),+
    ) => {
        $(
            impl private::Sealed for $target_type {}

            impl Volatile for $target_type {}
        )+
    };
}

volatile!(u8, u16, u32, u64, usize);

volatile!(i8, i16, i32, i64, isize);

/// A compile-time wrapper over an address used for memory-mapped I/O.
///
/// This type is uninhabited and only exist to indicate volatility of the target
/// memory location.
pub enum Memory<const A: usize> {}

impl<const A: usize> Memory<A> {
    /// Read a `T` from the target memory location.
    ///
    /// # Safety
    ///
    /// The const parameter `A` must be a [`valid pointer`] to `T`, i.e, it must
    /// be non-null and aligned for `T`.
    ///
    /// Furthermore, volatile and non-volatile reads must not coexist.
    ///
    /// [`valid pointer`]: https://doc.rust-lang.org/stable/core/ptr/index.html#safety
    #[inline]
    #[must_use]
    pub unsafe fn read<T>() -> T
    where
        T: Volatile,
    {
        // NOTE(undefined behaviour):
        //
        // Albeit alignment is part of the safety contract, a hardcoded check is
        // performed to ensure it is correct at compile-time for the natural
        // alignment of the type.
        //
        // Furthermore, it is not non-idiomatic to do this, as `libstd` and
        // `libcore` are plagued with `ub_check` invocations.
        () = const {
            let is_aligned = A.is_multiple_of(mem::align_of::<T>());

            assert!(is_aligned, "Address for type is not aligned");
        };

        // SAFETY: The target address `A` is a valid pointer to `T` due to the
        // safety contract.
        unsafe { ptr::read_volatile(ptr::with_exposed_provenance::<T>(A)) }
    }

    /// Write a `T` to the target memory location.
    ///
    /// # Safety
    ///
    /// The const parameter `A` must be a [`valid pointer`] to `T`, i.e., it
    /// must be non-null and aligned for `T`.
    ///
    /// Furthermore, volatile and non-volatile writes must not coexist.
    ///
    /// [`valid pointer`]: https://doc.rust-lang.org/stable/core/ptr/index.html#safety
    #[inline]
    pub unsafe fn write<T>(target_value: T)
    where
        T: Volatile,
    {
        // NOTE(undefined behaviour): See the other `undefined behaviour` note
        // on the `Memory::read` associated function.
        () = const {
            let is_aligned = A.is_multiple_of(mem::align_of::<T>());

            assert!(is_aligned, "Address for type is not aligned");
        };

        // SAFETY: The target address `A` is a valid pointer to `T` due to the
        // safety contract.
        unsafe { ptr::write_volatile(ptr::with_exposed_provenance_mut::<T>(A), target_value) }
    }
}

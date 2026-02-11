//! A module that provides functionality for the different memory-mapped
//! register modes.
//!
//! Generally, memory-mapped registers can only be accessed in a specific mode.
//!
//! The modes that are supported are the following:
//!
//! - Read-only mode: [`Ro`]
//! - Write-only mode: [`Wo`]
//! - Read-write mode: [`Rw`]

use core::marker;

use crate::memory::{Memory, Volatile};

/// A read-only register in a hardware peripheral.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ro<T, const A: usize>(
    // NOTE(variance): invariant over `T`, but could be affected volatile
    // non-primitive types.
    marker::PhantomData<fn() -> T>,
)
where
    T: Volatile;

impl<T, const A: usize> Ro<T, A>
where
    T: Volatile,
{
    /// Construct a new read-only memory-mapped register at the address `A`.
    #[inline]
    #[must_use]
    pub const fn register() -> Self {
        Self(marker::PhantomData)
    }
}

impl<T, const A: usize> Ro<T, A>
where
    T: Volatile,
{
    /// Read the value of the register.
    ///
    /// # Safety
    ///
    /// This associated function has the same safety contract as
    /// [`Memory::read`].
    #[inline]
    #[must_use]
    pub unsafe fn read() -> T {
        // SAFETY: This read operation is safe due to the caller safety
        // contract.
        unsafe { Memory::<A>::read::<T>() }
    }
}

/// A read-write register in a hardware peripheral.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rw<T, const A: usize>(
    // NOTE(variance): invariant over `T`, but could be affected volatile
    // non-primitive types.
    marker::PhantomData<fn() -> T>,
)
where
    T: Volatile;

impl<T, const A: usize> Rw<T, A>
where
    T: Volatile,
{
    /// Construct a new read-write memory-mapped register at the address `A`.
    #[inline]
    #[must_use]
    pub const fn register() -> Self {
        Self(marker::PhantomData)
    }
}

impl<T, const A: usize> Rw<T, A>
where
    T: Volatile,
{
    /// Read the value of the register.
    ///
    /// # Safety
    ///
    /// This associated function has the same safety contract as
    /// [`Memory::read`].
    #[inline]
    #[must_use]
    pub unsafe fn read() -> T {
        // SAFETY: This read operation is safe due to the caller safety
        // contract.
        unsafe { Memory::<A>::read::<T>() }
    }

    /// Write the target value to the register.
    ///
    /// # Safety
    ///
    /// This associated function has the same safety contract as
    /// [`Memory::write`].
    #[inline]
    pub unsafe fn write(target_value: T) {
        // SAFETY: This write operation is safe due to the caller safety
        // contract.
        unsafe { Memory::<A>::write::<T>(target_value) }
    }
}

/// A write-only register in a hardware peripheral.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Wo<T, const A: usize>(
    // NOTE(variance): invariant over `T`, but could be affected volatile
    // non-primitive types.
    marker::PhantomData<fn() -> T>,
)
where
    T: Volatile;

impl<T, const A: usize> Wo<T, A>
where
    T: Volatile,
{
    /// Construct a new write-only memory-mapped register at the address `A`.
    #[inline]
    #[must_use]
    pub const fn register() -> Self {
        Self(marker::PhantomData)
    }
}

impl<T, const A: usize> Wo<T, A>
where
    T: Volatile,
{
    /// Write the target value to the register.
    ///
    /// # Safety
    ///
    /// This associated function has the same safety contract as
    /// [`Memory::write`].
    #[inline]
    pub unsafe fn write(target_value: T) {
        // SAFETY: This write operation is safe due to the caller safety
        // contract.
        unsafe { Memory::<A>::write::<T>(target_value) }
    }
}

// TODO: Add Dyn- registers for MMIO addresses not known at compile time. This
// is largely thought of for ARM.

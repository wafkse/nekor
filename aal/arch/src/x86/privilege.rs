//! Privilege management for `x86`.

use core::{fmt::Debug, marker};

/// A `x86` privilege level.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
#[repr(u8)]
pub enum PrivilegeLevel {
    /// A privilege level of `0`.
    ///
    /// This is the maximum privilege level. also recognized as "kernelspace".
    Ring0 = 0b00,

    /// A privilege level of `1`.
    Ring1 = 0b01,

    /// A privilege level of `2`.
    Ring2 = 0b10,

    /// A privilege level of `3`.
    ///
    /// This is the least privileged level, also recognized as "userspace".
    Ring3 = 0b11,
}

impl PrivilegeLevel {
    /// Convert a raw byte value into a privilege level.
    ///
    /// # Remarks
    ///
    /// This will truncate the most significant `6` bits.
    #[inline]
    pub const fn raw(target_value: u8) -> Self {
        match target_value & 0b11 {
            0b00 => PrivilegeLevel::Ring0,
            0b01 => PrivilegeLevel::Ring1,
            0b10 => PrivilegeLevel::Ring2,
            0b11 => PrivilegeLevel::Ring3,
            _ => unreachable!(),
        }
    }
}

/// An *I/O* *Privilege Level*.
///
/// This is not a privilege level by itself, but rather a constraint that
/// determine whether a [`PrivilegeLevel`] can perform *I/O*.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
#[repr(u8)]
pub enum IoPrivilegeLevel {
    /// A privilege level of `0`.
    ///
    /// This is the maximum privilege level.
    Ring0 = 0b00,

    /// A privilege level of `1`.
    Ring1 = 0b01,

    /// A privilege level of `2`.
    Ring2 = 0b10,

    /// A privilege level of `3`.
    ///
    /// This is the least privileged level.
    Ring3 = 0b11,
}

mod detail {
    //! Implementation etails for the `privilege` module.

    /// A trait to act as a supertrait seal for the
    /// [`AssertPrivilegeLevelIsValid`] trait.
    ///
    /// [`AssertPrivilegeLevelIsValid`]: super::AssertPrivilegeLevelIsValid
    pub trait Sealed {}
}

/// A trait that determines whether a privilege level `N` is valid.
pub trait AssertPrivilegeLevelIsValid<const N: u8>: detail::Sealed {}

/// An uninhabited type used to determine whether a privilege level is valid.
pub enum AssertPrivilegeLevel {}

impl detail::Sealed for AssertPrivilegeLevel {}

impl AssertPrivilegeLevelIsValid<0> for AssertPrivilegeLevel {}
impl AssertPrivilegeLevelIsValid<1> for AssertPrivilegeLevel {}
impl AssertPrivilegeLevelIsValid<2> for AssertPrivilegeLevel {}
impl AssertPrivilegeLevelIsValid<3> for AssertPrivilegeLevel {}

/// A transparent struct over a `T` that proves that the *Current Privilege
/// Level* is `N`.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cpl<const N: u8, T>(
    pub T,
    // NOTE(variance): Guarantee that `Cpl` is covariant over `T`.
    marker::PhantomData<*mut ()>,
)
where
    AssertPrivilegeLevel: AssertPrivilegeLevelIsValid<N>;

impl<const N: u8, T> Cpl<N, T>
where
    AssertPrivilegeLevel: AssertPrivilegeLevelIsValid<N>,
{
    /// Assert that the *Current Privilege Level* is `N`.
    ///
    /// # Safety
    ///
    /// By instantiating this token, the caller guarantees that:
    /// * The token will **only ever be accessed or used** in an execution
    ///   context where the *Current Privilege Level* exactly matches `N`.
    /// * The token must never be sent, leaked, or accessed across privilege
    ///   boundaries (e.g., accessed by Ring 3 code if `N` is 0).
    #[inline]
    pub const unsafe fn assert(target_value: T) -> Self {
        Self(target_value, marker::PhantomData)
    }
}

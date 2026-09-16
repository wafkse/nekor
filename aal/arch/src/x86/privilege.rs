//! Privilege management for `x86`.

use core::{fmt::Debug, marker};

enumerate![
    /// An x86 privilege level.
    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
    #[non_exhaustive]
    pub enum PrivilegeLevel {
        /// Privilege level zero.
        ///
        /// This is the maximum privilege level and the supervisor level used by Nekor.
        Ring0 = 0,

        /// Privilege level one.
        Ring1 = 1,

        /// Privilege level two.
        Ring2 = 2,

        /// Privilege level three.
        ///
        /// This is the least privileged architectural level and the user level used by Nekor.
        Ring3 = 3,
    } as u8
];

enumerate![
    /// An I/O privilege level.
    ///
    /// This is a constraint on I/O instruction execution rather than the current privilege level.
    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
    #[non_exhaustive]
    pub enum IoPrivilegeLevel {
        /// I/O privilege level zero.
        Ring0 = 0,

        /// I/O privilege level one.
        Ring1,

        /// I/O privilege level two.
        Ring2,

        /// I/O privilege level three.
        Ring3,
    } as u8
];

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
    /// * The token will **only ever be accessed or used** in an execution context where the
    ///   *Current Privilege Level* exactly matches `N`.
    /// * The token must never be sent, leaked, or accessed across privilege boundaries (e.g.,
    ///   accessed by Ring 3 code if `N` is 0).
    #[inline]
    pub const unsafe fn assert(target_value: T) -> Self {
        Self(target_value, marker::PhantomData)
    }
}

#[cfg(test)]
mod tests {
    use super::{IoPrivilegeLevel, PrivilegeLevel};

    enumerate![
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum MixedIdentity {
            #[doc = "An explicitly assigned first identity."]
            Zero = 0,

            #[doc = "An implicitly assigned successor."]
            One,

            #[doc = "An explicitly assigned sparse identity."]
            Four = 4,

            #[doc = "An implicitly assigned successor to the sparse identity."]
            Five,
        } as u8
    ];

    #[test]
    fn privilege_level_lifts_and_erases_stable_repr() {
        assert_eq!(PrivilegeLevel::Ring0.raw(), 0);
        assert_eq!(PrivilegeLevel::Ring1.raw(), 1);
        assert_eq!(PrivilegeLevel::Ring2.raw(), 2);
        assert_eq!(PrivilegeLevel::Ring3.raw(), 3);
        assert_eq!(PrivilegeLevel::lift(0), Some(PrivilegeLevel::Ring0));
        assert_eq!(PrivilegeLevel::lift(3), Some(PrivilegeLevel::Ring3));
        assert_eq!(PrivilegeLevel::lift(4), None);
    }

    #[test]
    fn optional_discriminants_follow_rust_enum_rules() {
        assert_eq!(MixedIdentity::Zero.raw(), 0);
        assert_eq!(MixedIdentity::One.raw(), 1);
        assert_eq!(MixedIdentity::Four.raw(), 4);
        assert_eq!(MixedIdentity::Five.raw(), 5);
        assert_eq!(MixedIdentity::lift(2), None);
        assert_eq!(MixedIdentity::lift(5), Some(MixedIdentity::Five));
    }

    #[test]
    fn io_privilege_level_uses_generated_repr_wrappers() {
        assert_eq!(IoPrivilegeLevel::Ring0.raw(), 0);
        assert_eq!(IoPrivilegeLevel::Ring3.raw(), 3);
        assert_eq!(IoPrivilegeLevel::lift(4), None);
    }
}

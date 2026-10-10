//! Extended-state model-specific register representations and capability-aware access.
//!
//! [`XstateComponents`] provides the
//! component vocabulary. The wrappers in this module retain model-specific register identity and
//! preserve every raw bit supplied by hardware.

use super::{XfdAccess, XssAccess};
use crate::x86::xstate::XstateComponents;

/// Implementation details for component-backed model-specific register snapshots.
mod detail {
    /// Defines a component-backed model-specific register snapshot with its index and authority.
    macro_rules! component {
        () => {};
        (
            $(
                #[$target_meta:meta]
            )*

            $target_vis:vis struct $target_name:ident for ($target_address:expr) use $target_authority:ty
        ) => {
            tokel::stream!(
                $(
                    #[$target_meta]
                )*
                #[repr(transparent)]
                $target_vis struct $target_name(XstateComponents);

                impl $target_name {
                    /// An empty XSTATE component bitmap.
                    $target_vis const EMPTY: Self = Self(XstateComponents::empty());

                    /// Constructs a raw register snapshot.
                    #[inline]
                    #[must_use]
                    $target_vis const fn new(target_value: u64) -> Self {
                        Self(XstateComponents::from_bits_retain(target_value))
                    }

                    /// Returns the raw register snapshot.
                    #[inline]
                    #[must_use]
                    $target_vis const fn raw(self) -> u64 {
                        let Self(component) = self;

                        component.bits()
                    }

                    /// Borrows the register's XSTATE component bitmap.
                    #[inline]
                    #[must_use]
                    $target_vis const fn components(&self) -> &XstateComponents {
                        let &Self(ref component) = self;

                        component
                    }

                    /// Mutably borrows the register's XSTATE component bitmap.
                    #[inline]
                    $target_vis const fn components_mut(&mut self) -> &mut XstateComponents {
                        let &mut Self(ref mut component) = self;

                        component
                    }
                }

                // SAFETY: The snapshot has the size and alignment of u64 through transparent
                // XstateComponents storage, and every u64 bit pattern is valid.
                // The declared authority supplies the register's privilege and feature proof.
                unsafe impl $crate::x86::msr::Msr for $target_name {
                    type Access = nekor_register::mode::Rw<Self, u32>;
                    type Authority = $target_authority;

                    const ADDRESS: u32 = $target_address;
                }

                impl $crate::x86::msr::detail::Sealed for $target_name {}
            );
        };
    }

    pub(in crate::x86::msr::xstate) use component;
}

detail::component!(
    /// `IA32_XFD` disabled-component bitmap.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct RawXfd for (0x0000_01C4) use XfdAccess
);

detail::component!(
    /// `IA32_XFD_ERR` component bitmap.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct RawXfdErr for (0x0000_01C5) use XfdAccess
);

detail::component!(
    /// `IA32_XSS` supervisor component bitmap.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct RawXss for (0x0000_0DA0) use XssAccess
);

#[cfg(test)]
mod tests {
    use core::mem;

    use super::{RawXfd, RawXfdErr, RawXss, XstateComponents};
    use crate::x86::msr::Msr;

    #[test]
    fn xfd_registers_share_component_vocabulary_without_losing_identity() {
        let mut xfd = RawXfd::EMPTY;
        let mut error = RawXfdErr::EMPTY;

        xfd.components_mut().insert(XstateComponents::TILE_DATA);
        error.components_mut().insert(XstateComponents::TILE_DATA);

        assert!(xfd.components().contains(XstateComponents::TILE_DATA));
        assert!(error.components().contains(XstateComponents::TILE_DATA));
        assert_eq!(xfd.raw(), 1 << 18);
        assert_eq!(error.raw(), 1 << 18);
    }

    #[test]
    fn xss_named_flags_preserve_reserved_components() {
        let mut xss = RawXss::new(1 << 63);

        xss.components_mut()
            .insert(XstateComponents::CET_SUPERVISOR | XstateComponents::LBR);

        assert!(xss.components().contains(XstateComponents::CET_SUPERVISOR));
        assert!(xss.components().contains(XstateComponents::LBR));
        assert!(!xss.components().contains(XstateComponents::HDC));
        assert_eq!(xss.raw(), (1 << 63) | (1 << 12) | (1 << 15));
    }

    #[test]
    fn xstate_msr_layout_and_indices_match_architecture() {
        assert_eq!(mem::size_of::<RawXfd>(), mem::size_of::<u64>());
        assert_eq!(mem::align_of::<RawXfd>(), mem::align_of::<u64>());
        assert_eq!(mem::size_of::<RawXfdErr>(), mem::size_of::<u64>());
        assert_eq!(mem::align_of::<RawXfdErr>(), mem::align_of::<u64>());
        assert_eq!(mem::size_of::<RawXss>(), mem::size_of::<u64>());
        assert_eq!(mem::align_of::<RawXss>(), mem::align_of::<u64>());

        assert_eq!(RawXfd::REGISTER.address(), 0x1C4);
        assert_eq!(RawXfdErr::REGISTER.address(), 0x1C5);
        assert_eq!(RawXss::REGISTER.address(), 0xDA0);
    }
}

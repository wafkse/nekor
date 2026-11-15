//! XSTATE component bitmaps in `IA32_XFD`, `IA32_XFD_ERR` and `IA32_XSS`.

use crate::x86::xstate::XstateComponents;

/// The declarations shared by the XSTATE register snapshots.
mod detail {
    /// Define a register snapshot and its access requirements.
    macro_rules! component {
        () => {};
        (
            $(
                #[$target_meta:meta]
            )*

            $target_vis:vis struct $target_name:ident for ($target_address:expr)
            for<$target_lifetime:lifetime> $target_authorization:ty
        ) => {
            tokel::stream!(
                $(
                    #[$target_meta]
                )*
                #[repr(transparent)]
                // NOTE(invariant): None.
                $target_vis struct $target_name(XstateComponents);

                impl $target_name {
                    /// An empty XSTATE component bitmap.
                    $target_vis const EMPTY: Self = Self(XstateComponents::empty());

                    /// Construct a snapshot, preserving reserved bits.
                    #[inline]
                    #[must_use]
                    $target_vis const fn new(target_value: u64) -> Self {
                        Self(XstateComponents::from_bits_retain(target_value))
                    }

                    /// The raw register bits.
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
                        let &Self(ref target_component) = self;

                        target_component
                    }

                    /// Mutably borrows the register's XSTATE component bitmap.
                    #[inline]
                    $target_vis const fn components_mut(&mut self) -> &mut XstateComponents {
                        let &mut Self(ref mut target_component) = self;

                        target_component
                    }
                }

                impl $crate::x86::msr::Msr for $target_name {
                    type Access = nekor_register::mode::Rw<Self, u32>;

                    const ADDRESS: u32 = $target_address;

                    #[inline]
                    fn new(target_value: u64) -> Self { Self::new(target_value) }

                    #[inline]
                    fn raw(self) -> u64 { Self::raw(self) }
                }

                impl $crate::x86::msr::detail::Sealed for $target_name {}

                // SAFETY: The borrowed CPL0 and XSTATE capabilities establish the privilege
                // and processor support required by this register throughout the access.
                unsafe impl<$target_lifetime, T: $target_lifetime> $crate::x86::msr::Authorize<$target_name>
                    for $target_authorization {}
            );
        };
    }

    pub(in crate::x86::msr::xstate) use component;
}

detail::component!(
    /// The `IA32_XFD` extended feature disable model-specific register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MsrXfd for (0x0000_01C4)
    for<'a> (&'a crate::x86::privilege::Cpl<0, T>, &'a crate::x86::xstate::XfdCapability)
);

detail::component!(
    /// The `IA32_XFD_ERR` extended feature disable error model-specific register snapshot.
    ///
    /// Its bitmap identifies the components that caused an XFD fault.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MsrXfdErr for (0x0000_01C5)
    for<'a> (&'a crate::x86::privilege::Cpl<0, T>, &'a crate::x86::xstate::XfdCapability)
);

detail::component!(
    /// The `IA32_XSS` supervisor-state component mask model-specific register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MsrXss for (0x0000_0DA0)
    for<'a> (&'a crate::x86::privilege::Cpl<0, T>, &'a crate::x86::xstate::XsaveSupervisorCapability)
);

#[cfg(test)]
mod tests {
    use core::mem;

    use crate::x86::{
        msr::{
            Msr,
            xstate::{MsrXfd, MsrXfdErr, MsrXss},
        },
        xstate::XstateComponents,
    };

    #[test]
    fn xfd_registers_share_component_vocabulary_without_losing_identity() {
        let mut xfd = MsrXfd::EMPTY;
        let mut error = MsrXfdErr::EMPTY;

        xfd.components_mut().insert(XstateComponents::TILE_DATA);
        error.components_mut().insert(XstateComponents::TILE_DATA);

        assert!(xfd.components().contains(XstateComponents::TILE_DATA));
        assert!(error.components().contains(XstateComponents::TILE_DATA));
        assert_eq!(xfd.raw(), 1 << 18);
        assert_eq!(error.raw(), 1 << 18);
    }

    #[test]
    fn xss_named_flags_preserve_reserved_components() {
        let mut xss = MsrXss::new(1 << 63);

        xss.components_mut()
            .insert(XstateComponents::CET_SUPERVISOR | XstateComponents::LBR);

        assert!(xss.components().contains(XstateComponents::CET_SUPERVISOR));
        assert!(xss.components().contains(XstateComponents::LBR));
        assert!(!xss.components().contains(XstateComponents::HDC));
        assert_eq!(xss.raw(), (1 << 63) | (1 << 12) | (1 << 15));
    }

    #[test]
    fn xstate_msr_layout_and_indices_match_architecture() {
        assert_eq!(mem::size_of::<MsrXfd>(), mem::size_of::<u64>());
        assert_eq!(mem::align_of::<MsrXfd>(), mem::align_of::<u64>());
        assert_eq!(mem::size_of::<MsrXfdErr>(), mem::size_of::<u64>());
        assert_eq!(mem::align_of::<MsrXfdErr>(), mem::align_of::<u64>());
        assert_eq!(mem::size_of::<MsrXss>(), mem::size_of::<u64>());
        assert_eq!(mem::align_of::<MsrXss>(), mem::align_of::<u64>());

        assert_eq!(MsrXfd::REGISTER.address(), 0x1C4);
        assert_eq!(MsrXfdErr::REGISTER.address(), 0x1C5);
        assert_eq!(MsrXss::REGISTER.address(), 0xDA0);
    }
}

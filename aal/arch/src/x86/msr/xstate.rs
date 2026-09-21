//! Extended-state MSR representations and capability-aware access.
//!
//! [`XstateComponents`](crate::x86::xstate::XstateComponents) provides the
//! component vocabulary. The wrappers in this module retain MSR identity and
//! preserve every raw bit supplied by hardware.

use super::{Msr, ReadWrite, XfdAccess, XssAccess};
use crate::x86::xstate::XstateComponents;

/// `IA32_XFD` disabled-component bitmap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The flags value preserves every IA32_XFD bit. Safe hardware
// access additionally requires an XFD capability proof.
pub struct RawXfd(XstateComponents);

impl RawXfd {
    /// Empty disabled-component bitmap.
    pub const EMPTY: Self = Self(XstateComponents::empty());

    /// Constructs a raw XFD image.
    #[inline]
    #[must_use]
    pub const fn new(target_value: u64) -> Self {
        Self(XstateComponents::from_bits_retain(target_value))
    }

    /// Returns the raw XFD image.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u64 {
        let Self(components) = self;

        components.bits()
    }

    /// Borrows the disabled XSTATE components.
    #[inline]
    #[must_use]
    pub const fn components(&self) -> &XstateComponents {
        let &Self(ref components) = self;

        components
    }

    /// Mutably borrows the disabled XSTATE components.
    #[inline]
    pub const fn components_mut(&mut self) -> &mut XstateComponents {
        let &mut Self(ref mut components) = self;

        components
    }
}

// SAFETY: RawXfd has the size and alignment of u64 through transparent
// XstateComponents storage, and every u64 bit pattern is valid.
unsafe impl Msr for RawXfd {
    type Access = ReadWrite;
    type Authority = XfdAccess;

    const ADDRESS: u32 = 0x0000_01C4;
}

impl super::private::Sealed for RawXfd {}

/// `IA32_XFD_ERR` component bitmap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The flags value preserves every IA32_XFD_ERR bit. Safe
// hardware access additionally requires an XFD capability proof.
pub struct RawXfdErr(XstateComponents);

impl RawXfdErr {
    /// Empty error-component bitmap.
    pub const EMPTY: Self = Self(XstateComponents::empty());

    /// Constructs a raw XFD error image.
    #[inline]
    #[must_use]
    pub const fn new(target_value: u64) -> Self {
        Self(XstateComponents::from_bits_retain(target_value))
    }

    /// Returns the raw XFD error image.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u64 {
        let Self(components) = self;

        components.bits()
    }

    /// Borrows the faulting XSTATE components.
    #[inline]
    #[must_use]
    pub const fn components(&self) -> &XstateComponents {
        let &Self(ref components) = self;

        components
    }

    /// Mutably borrows the faulting XSTATE components.
    #[inline]
    pub const fn components_mut(&mut self) -> &mut XstateComponents {
        let &mut Self(ref mut components) = self;

        components
    }
}

// SAFETY: RawXfdErr has the size and alignment of u64 through transparent
// XstateComponents storage, and every u64 bit pattern is valid.
unsafe impl Msr for RawXfdErr {
    type Access = ReadWrite;
    type Authority = XfdAccess;

    const ADDRESS: u32 = 0x0000_01C5;
}

impl super::private::Sealed for RawXfdErr {}

/// `IA32_XSS` supervisor component bitmap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The flags value preserves every IA32_XSS bit. Processor
// support and XSAVE policy are checked at the hardware access boundary.
pub struct RawXss(XstateComponents);

impl RawXss {
    /// Empty supervisor component bitmap.
    pub const EMPTY: Self = Self(XstateComponents::empty());

    /// Constructs a raw XSS image.
    #[inline]
    #[must_use]
    pub const fn new(target_value: u64) -> Self {
        Self(XstateComponents::from_bits_retain(target_value))
    }

    /// Returns the raw XSS image.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u64 {
        let Self(components) = self;

        components.bits()
    }

    /// Borrows the supervisor XSTATE components.
    #[inline]
    #[must_use]
    pub const fn components(&self) -> &XstateComponents {
        let &Self(ref components) = self;

        components
    }

    /// Mutably borrows the supervisor XSTATE components.
    #[inline]
    pub const fn components_mut(&mut self) -> &mut XstateComponents {
        let &mut Self(ref mut components) = self;

        components
    }
}

// SAFETY: RawXss has the size and alignment of u64 through transparent
// XstateComponents storage, and every u64 bit pattern is valid.
unsafe impl Msr for RawXss {
    type Access = ReadWrite;
    type Authority = XssAccess;

    const ADDRESS: u32 = 0x0000_0DA0;
}

impl super::private::Sealed for RawXss {}

#[cfg(test)]
mod tests {
    use core::mem;

    use super::{Msr, RawXfd, RawXfdErr, RawXss, XstateComponents};

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

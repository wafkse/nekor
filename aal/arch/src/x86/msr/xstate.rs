//! Extended-state MSR representations and capability-aware access.
//!
//! [`XstateComponents`](crate::x86::xstate::XstateComponents) owns the bitmap
//! representation shared by XSTATE controls. The raw MSR types preserve
//! register identity and [`RawXss`](crate::x86::msr::xstate::RawXss) provides
//! named views for supervisor-state components.

use nekor_bitwise::prelude::{Bit, Counterpart};
use zerocopy::{Immutable, IntoBytes};

use super::{Msr, ReadWrite, read, write};
use crate::x86::{
    privilege::Cpl,
    xstate::{Xfd as XfdCapability, XstateComponents},
};

/// `IA32_XFD` disabled-component bitmap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, IntoBytes, Immutable)]
#[repr(transparent)]
// NOTE(invariant): The shared bitmap preserves every IA32_XFD bit. Safe
// hardware access additionally requires an XFD capability proof.
pub struct RawXfd(XstateComponents);

impl RawXfd {
    /// Empty disabled-component bitmap.
    pub const EMPTY: Self = Self(XstateComponents::EMPTY);

    /// Constructs a raw XFD image.
    #[inline]
    #[must_use]
    pub const fn new(target_value: u64) -> Self {
        Self(XstateComponents::new(target_value))
    }

    /// Returns the raw XFD image.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u64 {
        let Self(components) = self;

        components.raw()
    }

    /// Borrows the XSTATE component bitmap.
    #[inline]
    #[must_use]
    pub const fn components(&self) -> &XstateComponents {
        let &Self(ref components) = self;

        components
    }

    /// Mutably borrows the XSTATE component bitmap.
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

    const ADDRESS: u32 = 0x0000_01C4;
}

impl super::private::Sealed for RawXfd {}

/// `IA32_XFD_ERR` component bitmap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, IntoBytes, Immutable)]
#[repr(transparent)]
// NOTE(invariant): The shared bitmap preserves every IA32_XFD_ERR bit. Safe
// hardware access additionally requires an XFD capability proof.
pub struct RawXfdErr(XstateComponents);

impl RawXfdErr {
    /// Empty error-component bitmap.
    pub const EMPTY: Self = Self(XstateComponents::EMPTY);

    /// Constructs a raw XFD error image.
    #[inline]
    #[must_use]
    pub const fn new(target_value: u64) -> Self {
        Self(XstateComponents::new(target_value))
    }

    /// Returns the raw XFD error image.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u64 {
        let Self(components) = self;

        components.raw()
    }

    /// Borrows the XSTATE component bitmap.
    #[inline]
    #[must_use]
    pub const fn components(&self) -> &XstateComponents {
        let &Self(ref components) = self;

        components
    }

    /// Mutably borrows the XSTATE component bitmap.
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

    const ADDRESS: u32 = 0x0000_01C5;
}

impl super::private::Sealed for RawXfdErr {}

/// Read the live `IA32_XFD` bitmap after proving that the processor exposes XFD.
#[inline]
#[must_use]
pub fn read_xfd<T>(_cpl0: &Cpl<0, T>, _xfd: &XfdCapability) -> RawXfd {
    // SAFETY: The CPL proof supplies privilege and the XFD proof supplies architectural MSR
    // availability for this processor.
    unsafe { read::<RawXfd, false>() }
}

/// Read the live `IA32_XFD_ERR` bitmap after proving that the processor exposes XFD.
#[inline]
#[must_use]
pub fn read_xfd_err<T>(_cpl0: &Cpl<0, T>, _xfd: &XfdCapability) -> RawXfdErr {
    // SAFETY: The CPL proof supplies privilege and the XFD proof supplies architectural MSR
    // availability for this processor.
    unsafe { read::<RawXfdErr, false>() }
}

/// Clear the live `IA32_XFD` bitmap after proving that the processor exposes XFD.
#[inline]
pub fn clear_xfd<T>(_cpl0: &Cpl<0, T>, _xfd: &XfdCapability) {
    // SAFETY: The XFD proof establishes register availability and the all-zero bitmap is
    // architecturally valid regardless of supported state-component details.
    unsafe { write::<RawXfd, false>(RawXfd::EMPTY) };
}

/// Clear the live `IA32_XFD_ERR` bitmap after proving that the processor exposes XFD.
#[inline]
pub fn clear_xfd_err<T>(_cpl0: &Cpl<0, T>, _xfd: &XfdCapability) {
    // SAFETY: The XFD proof establishes register availability and the all-zero bitmap is
    // architecturally valid regardless of supported state-component details.
    unsafe { write::<RawXfdErr, false>(RawXfdErr::EMPTY) };
}

/// Read the live `IA32_XSS` supervisor component bitmap under a CPL0 proof.
#[inline]
#[must_use]
pub fn read_xss<T>(_cpl0: &Cpl<0, T>) -> RawXss {
    // SAFETY: The proof token supplies the RDMSR privilege requirement. The
    // caller already executes on x86 hardware where the modeled MSR is used.
    unsafe { read::<RawXss, false>() }
}

/// Write a `IA32_XSS` supervisor component bitmap under a CPL0 proof.
///
/// # Safety
///
/// Every set component must be supported by the processor and enabled through
/// the architectural XSAVE supervisor-state mechanism required by that bit.
#[inline]
pub unsafe fn write_xss<T>(_cpl0: &Cpl<0, T>, target_value: RawXss) {
    // SAFETY: The caller supplies the component-support and XSAVE-state
    // requirements. The proof token supplies WRMSR privilege.
    unsafe { write::<RawXss, false>(target_value) };
}

/// Processor Trace supervisor state in [`RawXss`].
pub type XssPt<'value> = Bit<'value, u64, 8>;

/// Process-address-space identifier supervisor state in [`RawXss`].
pub type XssPasid<'value> = Bit<'value, u64, 10>;

/// User CET supervisor-managed state in [`RawXss`].
pub type XssCetUser<'value> = Bit<'value, u64, 11>;

/// Supervisor CET state in [`RawXss`].
pub type XssCetSupervisor<'value> = Bit<'value, u64, 12>;

/// Hardware duty-cycle supervisor state in [`RawXss`].
pub type XssHdc<'value> = Bit<'value, u64, 13>;

/// User-interrupt supervisor state in [`RawXss`].
pub type XssUintr<'value> = Bit<'value, u64, 14>;

/// Architectural last-branch-record state in [`RawXss`].
pub type XssLbr<'value> = Bit<'value, u64, 15>;

/// Hardware-managed performance supervisor state in [`RawXss`].
pub type XssHwp<'value> = Bit<'value, u64, 16>;

/// Mutable Processor Trace supervisor state.
pub type XssPtMut<'value> = <XssPt<'value> as Counterpart>::Mut;

/// Mutable process-address-space identifier supervisor state.
pub type XssPasidMut<'value> = <XssPasid<'value> as Counterpart>::Mut;

/// Mutable user CET supervisor-managed state.
pub type XssCetUserMut<'value> = <XssCetUser<'value> as Counterpart>::Mut;

/// Mutable supervisor CET state.
pub type XssCetSupervisorMut<'value> = <XssCetSupervisor<'value> as Counterpart>::Mut;

/// Mutable hardware duty-cycle supervisor state.
pub type XssHdcMut<'value> = <XssHdc<'value> as Counterpart>::Mut;

/// Mutable user-interrupt supervisor state.
pub type XssUintrMut<'value> = <XssUintr<'value> as Counterpart>::Mut;

/// Mutable architectural last-branch-record state.
pub type XssLbrMut<'value> = <XssLbr<'value> as Counterpart>::Mut;

/// Mutable hardware-managed performance supervisor state.
pub type XssHwpMut<'value> = <XssHwp<'value> as Counterpart>::Mut;

/// `IA32_XSS` supervisor component bitmap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, IntoBytes, Immutable)]
#[repr(transparent)]
// NOTE(invariant): The shared bitmap preserves every IA32_XSS bit. Named field
// views never discard unknown component identities.
pub struct RawXss(XstateComponents);

impl RawXss {
    /// Empty supervisor component bitmap.
    pub const EMPTY: Self = Self(XstateComponents::EMPTY);

    /// Constructs a raw XSS image.
    #[inline]
    #[must_use]
    pub const fn new(target_value: u64) -> Self {
        Self(XstateComponents::new(target_value))
    }

    /// Returns the raw XSS image.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u64 {
        let Self(components) = self;

        components.raw()
    }

    /// Borrows the XSTATE component bitmap.
    #[inline]
    #[must_use]
    pub const fn components(&self) -> &XstateComponents {
        let &Self(ref components) = self;

        components
    }

    /// Mutably borrows the XSTATE component bitmap.
    #[inline]
    pub const fn components_mut(&mut self) -> &mut XstateComponents {
        let &mut Self(ref mut components) = self;

        components
    }

    /// Borrow Processor Trace supervisor state.
    #[inline]
    #[must_use]
    pub const fn pt(&self) -> XssPt<'_> {
        let &Self(ref components) = self;

        components.component::<8>()
    }

    /// Mutably borrow Processor Trace supervisor state.
    #[inline]
    pub const fn pt_mut(&mut self) -> XssPtMut<'_> {
        let &mut Self(ref mut components) = self;

        components.component_mut::<8>()
    }

    /// Borrow PASID supervisor state.
    #[inline]
    #[must_use]
    pub const fn pasid(&self) -> XssPasid<'_> {
        let &Self(ref components) = self;

        components.component::<10>()
    }

    /// Mutably borrow PASID supervisor state.
    #[inline]
    pub const fn pasid_mut(&mut self) -> XssPasidMut<'_> {
        let &mut Self(ref mut components) = self;

        components.component_mut::<10>()
    }

    /// Borrow user CET supervisor-managed state.
    #[inline]
    #[must_use]
    pub const fn cet_user(&self) -> XssCetUser<'_> {
        let &Self(ref components) = self;

        components.component::<11>()
    }

    /// Mutably borrow user CET supervisor-managed state.
    #[inline]
    pub const fn cet_user_mut(&mut self) -> XssCetUserMut<'_> {
        let &mut Self(ref mut components) = self;

        components.component_mut::<11>()
    }

    /// Borrow supervisor CET state.
    #[inline]
    #[must_use]
    pub const fn cet_supervisor(&self) -> XssCetSupervisor<'_> {
        let &Self(ref components) = self;

        components.component::<12>()
    }

    /// Mutably borrow supervisor CET state.
    #[inline]
    pub const fn cet_supervisor_mut(&mut self) -> XssCetSupervisorMut<'_> {
        let &mut Self(ref mut components) = self;

        components.component_mut::<12>()
    }

    /// Borrow hardware duty-cycle supervisor state.
    #[inline]
    #[must_use]
    pub const fn hdc(&self) -> XssHdc<'_> {
        let &Self(ref components) = self;

        components.component::<13>()
    }

    /// Mutably borrow hardware duty-cycle supervisor state.
    #[inline]
    pub const fn hdc_mut(&mut self) -> XssHdcMut<'_> {
        let &mut Self(ref mut components) = self;

        components.component_mut::<13>()
    }

    /// Borrow user-interrupt supervisor state.
    #[inline]
    #[must_use]
    pub const fn uintr(&self) -> XssUintr<'_> {
        let &Self(ref components) = self;

        components.component::<14>()
    }

    /// Mutably borrow user-interrupt supervisor state.
    #[inline]
    pub const fn uintr_mut(&mut self) -> XssUintrMut<'_> {
        let &mut Self(ref mut components) = self;

        components.component_mut::<14>()
    }

    /// Borrow architectural last-branch-record state.
    #[inline]
    #[must_use]
    pub const fn lbr(&self) -> XssLbr<'_> {
        let &Self(ref components) = self;

        components.component::<15>()
    }

    /// Mutably borrow architectural last-branch-record state.
    #[inline]
    pub const fn lbr_mut(&mut self) -> XssLbrMut<'_> {
        let &mut Self(ref mut components) = self;

        components.component_mut::<15>()
    }

    /// Borrow hardware-managed performance supervisor state.
    #[inline]
    #[must_use]
    pub const fn hwp(&self) -> XssHwp<'_> {
        let &Self(ref components) = self;

        components.component::<16>()
    }

    /// Mutably borrow hardware-managed performance supervisor state.
    #[inline]
    pub const fn hwp_mut(&mut self) -> XssHwpMut<'_> {
        let &mut Self(ref mut components) = self;

        components.component_mut::<16>()
    }
}

// SAFETY: RawXss has the size and alignment of u64 through transparent
// XstateComponents storage, and every u64 bit pattern is valid.
unsafe impl Msr for RawXss {
    type Access = ReadWrite;

    const ADDRESS: u32 = 0x0000_0DA0;
}

impl super::private::Sealed for RawXss {}

#[cfg(test)]
mod tests {
    use core::mem;

    use nekor_bitwise::prelude::State;

    use super::{Msr, RawXfd, RawXfdErr, RawXss};

    #[test]
    fn xfd_registers_share_component_storage_without_losing_identity() {
        let mut xfd = RawXfd::EMPTY;
        let mut error = RawXfdErr::EMPTY;

        xfd.components_mut().component_mut::<18>().const_set(State::Set);
        error.components_mut().component_mut::<18>().const_set(State::Set);

        assert_eq!(xfd.components().component::<18>().const_state(), State::Set);
        assert_eq!(error.components().component::<18>().const_state(), State::Set);
        assert_eq!(xfd.raw(), 1 << 18);
        assert_eq!(error.raw(), 1 << 18);
    }

    #[test]
    fn xss_named_views_preserve_unknown_components() {
        let mut xss = RawXss::new(1 << 63);

        xss.cet_supervisor_mut().const_set(State::Set);
        xss.lbr_mut().const_set(State::Set);

        assert_eq!(xss.cet_supervisor().const_state(), State::Set);
        assert_eq!(xss.lbr().const_state(), State::Set);
        assert_eq!(xss.hdc().const_state(), State::Cleared);
        assert_eq!(xss.components().component::<63>().const_state(), State::Set);
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

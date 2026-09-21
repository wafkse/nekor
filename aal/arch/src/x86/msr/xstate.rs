//! Extended-state MSR representations and capability-aware access.
//!
//! RawXfd and RawXfdErr require an XFD capability for hardware access.
//! RawXss models the supervisor-state bitmap through named and const-generic
//! component views without discarding unknown architectural bits.

use nekor_bitwise::prelude::{Bit, BitAt, Counterpart};

use super::{Msr, ReadWrite, read, write};
use crate::x86::{privilege::Cpl, xstate::Xfd as XfdCapability};

/// Implements the shared raw component-bitmap operations without changing
/// each MSR type's transparent one-word representation.
macro_rules! impl_raw_component_bitmap {
    ($target:ident) => {
        impl $target {
            /// Empty component bitmap.
            pub const EMPTY: Self = Self(0);

            /// Preserves one complete component bitmap.
            #[inline]
            #[must_use]
            pub const fn new(target_value: u64) -> Self {
                Self(target_value)
            }

            /// Returns the complete component bitmap.
            #[inline]
            #[must_use]
            pub const fn raw(self) -> u64 {
                let Self(target_value) = self;

                target_value
            }

            /// Returns whether every required component is present.
            #[inline]
            #[must_use]
            pub const fn contains(self, required: Self) -> bool {
                let Self(bits) = self;
                let Self(required) = required;

                bits & required == required
            }

            /// Combines every component present in either bitmap.
            #[inline]
            #[must_use]
            pub const fn union(self, other: Self) -> Self {
                let Self(bits) = self;
                let Self(other) = other;

                Self(bits | other)
            }

            /// Retains only components present in both bitmaps.
            #[inline]
            #[must_use]
            pub const fn intersection(self, other: Self) -> Self {
                let Self(bits) = self;
                let Self(other) = other;

                Self(bits & other)
            }

            /// Returns whether no component is selected.
            #[inline]
            #[must_use]
            pub const fn is_empty(self) -> bool {
                let Self(bits) = self;

                bits == 0
            }

            /// Borrows one component selected by its architectural index.
            #[inline]
            #[must_use]
            pub const fn component<const N: usize>(&self) -> Bit<'_, u64, N>
            where
                u64: BitAt<N>,
            {
                let &Self(ref bits) = self;

                Bit::wrap(bits)
            }

            /// Mutably borrows one component selected by its architectural index.
            #[inline]
            pub const fn component_mut<const N: usize>(&mut self) -> <Bit<'_, u64, N> as Counterpart>::Mut
            where
                u64: BitAt<N>,
            {
                let &mut Self(ref mut bits) = self;

                <Bit<'_, u64, N> as Counterpart>::Mut::wrap(bits)
            }
        }
    };
}

/// Complete `IA32_XFD` disabled-component bitmap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar preserves every architectural IA32_XFD component bit.
// Safe hardware access additionally requires an XFD capability proof.
pub struct RawXfd(u64);

impl_raw_component_bitmap!(RawXfd);

// SAFETY: `RawXfd` is transparent over `u64` and every hardware bitmap remains representable.
unsafe impl Msr for RawXfd {
    type Access = ReadWrite;

    const ADDRESS: u32 = 0x0000_01C4;
}

impl super::private::Sealed for RawXfd {}

/// Complete `IA32_XFD_ERR` component bitmap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar preserves every architectural IA32_XFD_ERR component bit.
// Safe hardware access additionally requires an XFD capability proof.
pub struct RawXfdErr(u64);

impl_raw_component_bitmap!(RawXfdErr);

// SAFETY: `RawXfdErr` is transparent over `u64` and every hardware bitmap remains representable.
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

/// Write one `IA32_XSS` supervisor component bitmap under a CPL0 proof.
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

/// Complete `IA32_XSS` supervisor component bitmap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar preserves every `IA32_XSS` component bit. Safe field
// inspection never discards unknown component identities.
pub struct RawXss(u64);

impl_raw_component_bitmap!(RawXss);

impl RawXss {
    /// Borrow Processor Trace supervisor state.
    #[inline]
    #[must_use]
    pub const fn pt(&self) -> XssPt<'_> {
        let &Self(ref bits) = self;

        XssPt::wrap(bits)
    }

    /// Mutably borrow Processor Trace supervisor state.
    #[inline]
    pub const fn pt_mut(&mut self) -> XssPtMut<'_> {
        let &mut Self(ref mut bits) = self;

        <XssPt<'_> as Counterpart>::Mut::wrap(bits)
    }

    /// Borrow PASID supervisor state.
    #[inline]
    #[must_use]
    pub const fn pasid(&self) -> XssPasid<'_> {
        let &Self(ref bits) = self;

        XssPasid::wrap(bits)
    }

    /// Mutably borrow PASID supervisor state.
    #[inline]
    pub const fn pasid_mut(&mut self) -> XssPasidMut<'_> {
        let &mut Self(ref mut bits) = self;

        <XssPasid<'_> as Counterpart>::Mut::wrap(bits)
    }

    /// Borrow user CET supervisor-managed state.
    #[inline]
    #[must_use]
    pub const fn cet_user(&self) -> XssCetUser<'_> {
        let &Self(ref bits) = self;

        XssCetUser::wrap(bits)
    }

    /// Mutably borrow user CET supervisor-managed state.
    #[inline]
    pub const fn cet_user_mut(&mut self) -> XssCetUserMut<'_> {
        let &mut Self(ref mut bits) = self;

        <XssCetUser<'_> as Counterpart>::Mut::wrap(bits)
    }

    /// Borrow supervisor CET state.
    #[inline]
    #[must_use]
    pub const fn cet_supervisor(&self) -> XssCetSupervisor<'_> {
        let &Self(ref bits) = self;

        XssCetSupervisor::wrap(bits)
    }

    /// Mutably borrow supervisor CET state.
    #[inline]
    pub const fn cet_supervisor_mut(&mut self) -> XssCetSupervisorMut<'_> {
        let &mut Self(ref mut bits) = self;

        <XssCetSupervisor<'_> as Counterpart>::Mut::wrap(bits)
    }

    /// Borrow hardware duty-cycle supervisor state.
    #[inline]
    #[must_use]
    pub const fn hdc(&self) -> XssHdc<'_> {
        let &Self(ref bits) = self;

        XssHdc::wrap(bits)
    }

    /// Mutably borrow hardware duty-cycle supervisor state.
    #[inline]
    pub const fn hdc_mut(&mut self) -> XssHdcMut<'_> {
        let &mut Self(ref mut bits) = self;

        <XssHdc<'_> as Counterpart>::Mut::wrap(bits)
    }

    /// Borrow user-interrupt supervisor state.
    #[inline]
    #[must_use]
    pub const fn uintr(&self) -> XssUintr<'_> {
        let &Self(ref bits) = self;

        XssUintr::wrap(bits)
    }

    /// Mutably borrow user-interrupt supervisor state.
    #[inline]
    pub const fn uintr_mut(&mut self) -> XssUintrMut<'_> {
        let &mut Self(ref mut bits) = self;

        <XssUintr<'_> as Counterpart>::Mut::wrap(bits)
    }

    /// Borrow architectural last-branch-record state.
    #[inline]
    #[must_use]
    pub const fn lbr(&self) -> XssLbr<'_> {
        let &Self(ref bits) = self;

        XssLbr::wrap(bits)
    }

    /// Mutably borrow architectural last-branch-record state.
    #[inline]
    pub const fn lbr_mut(&mut self) -> XssLbrMut<'_> {
        let &mut Self(ref mut bits) = self;

        <XssLbr<'_> as Counterpart>::Mut::wrap(bits)
    }

    /// Borrow hardware-managed performance supervisor state.
    #[inline]
    #[must_use]
    pub const fn hwp(&self) -> XssHwp<'_> {
        let &Self(ref bits) = self;

        XssHwp::wrap(bits)
    }

    /// Mutably borrow hardware-managed performance supervisor state.
    #[inline]
    pub const fn hwp_mut(&mut self) -> XssHwpMut<'_> {
        let &mut Self(ref mut bits) = self;

        <XssHwp<'_> as Counterpart>::Mut::wrap(bits)
    }
}

// SAFETY: `RawXss` is transparent over `u64` and every hardware bitmap remains representable.
unsafe impl Msr for RawXss {
    type Access = ReadWrite;

    const ADDRESS: u32 = 0x0000_0DA0;
}

impl super::private::Sealed for RawXss {}

#[cfg(test)]
mod tests {
    use nekor_bitwise::prelude::State;

    use super::{Msr, RawXfd, RawXfdErr, RawXss};

    #[test]
    fn xfd_bitmaps_preserve_component_identity() {
        let mut xfd = RawXfd::EMPTY;
        let mut error = RawXfdErr::EMPTY;

        xfd.component_mut::<18>().const_set(State::Set);
        error.component_mut::<18>().const_set(State::Set);

        assert_eq!(xfd.component::<18>().const_state(), State::Set);
        assert_eq!(error.component::<18>().const_state(), State::Set);
        assert!(!error.is_empty());
        assert!(RawXfdErr::EMPTY.is_empty());
    }

    #[test]
    fn xss_components_preserve_independent_state() {
        let mut xss = RawXss::EMPTY;

        xss.cet_supervisor_mut().const_set(State::Set);
        xss.lbr_mut().const_set(State::Set);

        assert_eq!(xss.cet_supervisor().const_state(), State::Set);
        assert_eq!(xss.lbr().const_state(), State::Set);
        assert_eq!(xss.hdc().const_state(), State::Cleared);
    }

    #[test]
    fn register_indices_match_architecture() {
        assert_eq!(RawXfd::REGISTER.address(), 0x1C4);
        assert_eq!(RawXfdErr::REGISTER.address(), 0x1C5);
        assert_eq!(RawXss::REGISTER.address(), 0xDA0);
    }
}

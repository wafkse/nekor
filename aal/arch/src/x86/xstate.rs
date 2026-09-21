//! x86 extended-control and XSTATE component masks.
//!
//! [`XstateComponents`] names architectural state components shared by XCR0,
//! IA32_XSS, IA32_XFD, and IA32_XFD_ERR. Register wrappers retain their own
//! identities while using the same component vocabulary.

use bitflags::bitflags;

/// Proof that the current processor exposes the XFD architectural MSRs and semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Xfd(());

impl Xfd {
    /// Create the XFD capability proof for the current processor.
    ///
    /// # Safety
    ///
    /// The current processor must report the XFD architectural feature.
    #[inline]
    #[must_use]
    pub const unsafe fn assume() -> Self {
        Self(())
    }
}

/// Proof that the current processor exposes IA32_XSS and supervisor XSAVE state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct XsaveSupervisor(());

impl XsaveSupervisor {
    /// Create the supervisor XSAVE capability proof for the current processor.
    ///
    /// # Safety
    ///
    /// The current processor must report support for XSAVE supervisor state and
    /// the IA32_XSS model-specific register.
    #[inline]
    #[must_use]
    pub const unsafe fn assume() -> Self {
        Self(())
    }
}

bitflags! {
    /// XSTATE component bitmap shared by architectural XSTATE controls.
    ///
    /// The unnamed mask makes every bit part of the represented domain. Bitwise
    /// operations therefore retain reserved and future architectural components.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[repr(transparent)]
    pub struct XstateComponents: u64 {
        /// Legacy x87 state.
        const X87 = 1 << 0;

        /// SSE state.
        const SSE = 1 << 1;

        /// AVX state.
        const AVX = 1 << 2;

        /// MPX bound-register state.
        const BNDREGS = 1 << 3;

        /// MPX bound-configuration state.
        const BNDCSR = 1 << 4;

        /// AVX-512 opmask state.
        const OPMASK = 1 << 5;

        /// AVX-512 upper ZMM state.
        const ZMM_HI256 = 1 << 6;

        /// AVX-512 high ZMM register state.
        const HI16_ZMM = 1 << 7;

        /// Processor Trace state.
        const PT = 1 << 8;

        /// Protection-key state.
        const PKRU = 1 << 9;

        /// Process-address-space identifier state.
        const PASID = 1 << 10;

        /// User CET state.
        const CET_USER = 1 << 11;

        /// Supervisor CET state.
        const CET_SUPERVISOR = 1 << 12;

        /// Hardware duty-cycle state.
        const HDC = 1 << 13;

        /// User-interrupt state.
        const UINTR = 1 << 14;

        /// Architectural last-branch-record state.
        const LBR = 1 << 15;

        /// Hardware-managed performance state.
        const HWP = 1 << 16;

        /// AMX tile-configuration state.
        const TILE_CONFIG = 1 << 17;

        /// AMX tile-data state.
        const TILE_DATA = 1 << 18;

        /// Intel APX extended general-purpose register state.
        const APX = 1 << 19;

        /// Retains reserved and future component identities.
        const _ = !0;
    }
}

/// XCR0 component bitmap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): `XstateComponents` retains every XCR0 bit, including values
// that require processor-specific validation before installation.
pub struct Xcr0(XstateComponents);

impl Xcr0 {
    /// Empty component bitmap.
    pub const EMPTY: Self = Self(XstateComponents::empty());

    /// Constructs an XCR0 image from raw architectural bits.
    #[inline]
    #[must_use]
    pub const fn new(target_value: u64) -> Self {
        Self(XstateComponents::from_bits_retain(target_value))
    }

    /// Returns the raw XCR0 image.
    #[inline]
    #[must_use]
    pub const fn bits(self) -> u64 {
        let Self(components) = self;

        components.bits()
    }

    /// Borrows the selected XSTATE components.
    #[inline]
    #[must_use]
    pub const fn components(&self) -> &XstateComponents {
        let &Self(ref components) = self;

        components
    }

    /// Mutably borrows the selected XSTATE components.
    #[inline]
    pub const fn components_mut(&mut self) -> &mut XstateComponents {
        let &mut Self(ref mut components) = self;

        components
    }

    /// Returns whether every required XCR0 component is selected.
    #[inline]
    #[must_use]
    pub const fn contains(self, required: Self) -> bool {
        let Self(components) = self;
        let Self(required) = required;

        components.contains(required)
    }

    /// Combines the selected components from both XCR0 images.
    #[inline]
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        let Self(components) = self;
        let Self(other) = other;

        Self(components.union(other))
    }

    /// Retains the components selected by both XCR0 images.
    #[inline]
    #[must_use]
    pub const fn intersection(self, other: Self) -> Self {
        let Self(components) = self;
        let Self(other) = other;

        Self(components.intersection(other))
    }

    /// Returns whether no XCR0 component is selected.
    #[inline]
    #[must_use]
    pub const fn is_empty(self) -> bool {
        let Self(components) = self;

        components.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use core::mem;

    use super::{Xcr0, XstateComponents};

    #[test]
    fn component_bitmap_matches_the_architectural_word() {
        assert_eq!(mem::size_of::<XstateComponents>(), mem::size_of::<u64>());
        assert_eq!(mem::align_of::<XstateComponents>(), mem::align_of::<u64>());
        assert_eq!(mem::size_of::<Xcr0>(), mem::size_of::<u64>());
        assert_eq!(mem::align_of::<Xcr0>(), mem::align_of::<u64>());
    }

    #[test]
    fn component_operations_preserve_reserved_bits() {
        let reserved = XstateComponents::from_bits_retain(1 << 63);
        let selected = reserved | XstateComponents::X87 | XstateComponents::SSE;

        assert!(selected.contains(XstateComponents::X87 | XstateComponents::SSE));
        assert_eq!(selected.bits(), (1 << 63) | 0b11);
    }

    #[test]
    fn xcr0_retains_register_identity_around_shared_components() {
        let mut provided = Xcr0::EMPTY;
        let required = Xcr0::new((XstateComponents::X87 | XstateComponents::SSE).bits());

        provided
            .components_mut()
            .insert(XstateComponents::X87 | XstateComponents::SSE | XstateComponents::AVX);

        assert!(provided.contains(required));
        assert!(!required.contains(provided));
    }
}

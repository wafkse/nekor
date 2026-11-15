//! Extended feature enable register representations and access.
//!
//! `MsrEfer` exposes the register image, including processor-maintained fields.
//! `LongModeEfer` prepares the active long-mode state used by saved control contexts.
//! Neither value proves that the corresponding state is installed on the processor.

use nekor_bitwise::prelude::{Bit, Counterpart, State};

use crate::x86::msr::detail::snapshotted;

/// The System Call Extensions enable flag in `MsrEfer`.
pub type EferSce<'a> = Bit<'a, u64, 0>;

/// The Long Mode Enable flag in `MsrEfer`.
pub type EferLme<'a> = Bit<'a, u64, 8>;

/// The processor-maintained Long Mode Active flag in `MsrEfer`.
pub type EferLma<'a> = Bit<'a, u64, 10>;

/// The No-Execute Enable flag in `MsrEfer`.
pub type EferNxe<'a> = Bit<'a, u64, 11>;

/// The mutable counterpart to `EferSce`.
pub type EferSceMut<'a> = <EferSce<'a> as Counterpart>::Mut;

/// The mutable counterpart to `EferLme`.
pub type EferLmeMut<'a> = <EferLme<'a> as Counterpart>::Mut;

/// The mutable counterpart to `EferLma`.
pub type EferLmaMut<'a> = <EferLma<'a> as Counterpart>::Mut;

/// The mutable counterpart to `EferNxe`.
pub type EferNxeMut<'a> = <EferNxe<'a> as Counterpart>::Mut;

snapshotted::snapshot!(
    /// The `IA32_EFER` extended feature enable model-specific register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MsrEfer for (0xC000_0080)
    for<'a> &'a crate::x86::privilege::Cpl<0, T>
);

impl MsrEfer {
    /// The architectural reset snapshot used by this model.
    pub const RESET: Self = Self(u64::MIN);

    /// Borrows the system-call enable flag.
    #[inline]
    #[must_use]
    pub const fn sce(&self) -> EferSce<'_> {
        let &Self(ref target_value) = self;

        EferSce::wrap(target_value)
    }

    /// Mutably borrows the system-call enable flag.
    #[inline]
    pub const fn sce_mut(&mut self) -> EferSceMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EferSceMut::wrap(target_value)
    }

    /// Borrows the long-mode enable flag.
    #[inline]
    #[must_use]
    pub const fn lme(&self) -> EferLme<'_> {
        let &Self(ref target_value) = self;

        EferLme::wrap(target_value)
    }

    /// Mutably borrows the long-mode enable flag.
    #[inline]
    pub const fn lme_mut(&mut self) -> EferLmeMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EferLmeMut::wrap(target_value)
    }

    /// Borrows the processor-maintained long-mode active flag.
    #[inline]
    #[must_use]
    pub const fn lma(&self) -> EferLma<'_> {
        let &Self(ref target_value) = self;

        EferLma::wrap(target_value)
    }

    /// Mutably borrows the processor-maintained long-mode active flag.
    ///
    /// Editing the image does not write the processor-maintained field. The write caller
    /// must still satisfy the instruction's requirements.
    #[inline]
    pub const fn lma_mut(&mut self) -> EferLmaMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EferLmaMut::wrap(target_value)
    }

    /// Borrows the no-execute enable flag.
    #[inline]
    #[must_use]
    pub const fn nxe(&self) -> EferNxe<'_> {
        let &Self(ref target_value) = self;

        EferNxe::wrap(target_value)
    }

    /// Mutably borrows the no-execute enable flag.
    #[inline]
    pub const fn nxe_mut(&mut self) -> EferNxeMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EferNxeMut::wrap(target_value)
    }
}

/// A saved `EFER` image with long-mode enable and active flags set.
///
/// A saved control context needs both flags when describing active long mode. Construction
/// requires LME and supplies LMA in that image. It does not activate long mode or authorize
/// writing the processor-maintained LMA field through `WRMSR`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The wrapped `EFER` snapshot always has both LME and LMA set. Construction checks
// LME and sets LMA in the image used for saved-state transport.
pub struct LongModeEfer(MsrEfer);

impl LongModeEfer {
    /// Construct the [`LongModeEfer`] proof-type when LME is enabled.
    #[inline]
    #[must_use]
    pub const fn new(efer: MsrEfer) -> Option<Self> {
        match efer.lme().const_state() {
            State::Set => {
                let mut target_value = efer;

                target_value.lma_mut().const_set(State::Set);

                Some(Self(target_value))
            },
            State::Cleared => None,
        }
    }

    /// Extract the saved `EFER` image from the [`LongModeEfer`] proof-type.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> MsrEfer {
        let Self(efer) = self;

        efer
    }
}

#[cfg(test)]
mod tests {
    use nekor_bitwise::prelude::State;

    use crate::x86::msr::{
        Msr,
        efer::{LongModeEfer, MsrEfer},
    };

    #[test]
    fn long_mode_requires_enable_and_sets_active_restore_state() {
        let inactive = LongModeEfer::new(MsrEfer::RESET);
        let mut enabled = MsrEfer::RESET;

        enabled.lme_mut().const_set(State::Set);

        let active = LongModeEfer::new(enabled);
        let raw = active.map(LongModeEfer::raw);

        assert_eq!(inactive, None);
        assert_eq!(raw.map(MsrEfer::raw), Some(0x500));
    }

    #[test]
    fn field_mutation_preserves_unrelated_state() {
        let mut efer = MsrEfer::RESET;

        efer.sce_mut().const_set(State::Set);
        efer.lme_mut().const_set(State::Set);
        efer.nxe_mut().const_set(State::Set);

        assert_eq!(efer.sce().state(), State::Set);
        assert_eq!(efer.lme().state(), State::Set);
        assert_eq!(efer.lma().state(), State::Cleared);
        assert_eq!(efer.nxe().state(), State::Set);
    }

    #[test]
    fn raw_efer_preserves_processor_owned_state() {
        let mut target_value = MsrEfer::new(u64::MAX);

        target_value.lma_mut().const_set(State::Cleared);

        assert_eq!(target_value.lma().const_state(), State::Cleared);
    }

    #[test]
    fn register_index_matches_architecture() {
        assert_eq!(MsrEfer::REGISTER.address(), 0xC000_0080);
    }
}

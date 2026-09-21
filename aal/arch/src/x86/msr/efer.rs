//! Extended feature enable register representations and access.
//!
//! RawEfer owns the architectural image. Efer exposes safe mutation
//! of software-controlled fields. LongModeEfer carries the stronger active
//! long-mode proof needed by saved control-state representations.

use nekor_bitwise::prelude::{Bit, Counterpart, State};

use crate::x86::msr::{Cpl0Access, Msr, ReadWrite};

/// System Call Extensions enable flag in RawEfer.
pub type EferSce<'value> = Bit<'value, u64, 0>;

/// Long Mode Enable flag in RawEfer.
pub type EferLme<'value> = Bit<'value, u64, 8>;

/// Processor-maintained Long Mode Active flag in RawEfer.
pub type EferLma<'value> = Bit<'value, u64, 10>;

/// No-Execute Enable flag in RawEfer.
pub type EferNxe<'value> = Bit<'value, u64, 11>;

/// Mutable counterpart to EferSce.
pub type EferSceMut<'value> = <EferSce<'value> as Counterpart>::Mut;

/// Mutable counterpart to EferLme.
pub type EferLmeMut<'value> = <EferLme<'value> as Counterpart>::Mut;

/// Mutable counterpart to EferLma.
pub type EferLmaMut<'value> = <EferLma<'value> as Counterpart>::Mut;

/// Mutable counterpart to EferNxe.
pub type EferNxeMut<'value> = <EferNxe<'value> as Counterpart>::Mut;

/// Exact IA32_EFER register image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): Every u64 value is a representable raw EFER image.
pub struct RawEfer(u64);

impl RawEfer {
    /// Architectural reset image used by this model.
    pub const RESET: Self = Self(u64::MIN);

    /// Constructs a raw EFER image.
    #[inline]
    #[must_use]
    pub const fn new(target_value: u64) -> Self {
        Self(target_value)
    }

    /// Returns the raw EFER image.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u64 {
        let Self(target_value) = self;

        target_value
    }

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
    /// Raw mutation does not claim that the resulting value is writable to
    /// live hardware.
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

// SAFETY: RawEfer is transparent over u64 and every bit pattern is valid.
unsafe impl Msr for RawEfer {
    type Access = ReadWrite;
    type Authority = Cpl0Access;

    const ADDRESS: u32 = 0xC000_0080;
}

impl super::private::Sealed for RawEfer {}

/// Software-controlled EFER value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The wrapped raw image can preserve every hardware-provided bit while the public
// mutation API excludes processor-maintained LMA.
pub struct Efer(RawEfer);

impl Efer {
    /// Architectural reset value used by this model.
    pub const RESET: Self = Self(RawEfer::RESET);

    /// Lifts a raw EFER image.
    #[inline]
    #[must_use]
    pub const fn lift(target_value: RawEfer) -> Self {
        Self(target_value)
    }

    /// Borrows the system-call enable flag.
    #[inline]
    #[must_use]
    pub const fn sce(&self) -> EferSce<'_> {
        let &Self(ref target_value) = self;

        target_value.sce()
    }

    /// Mutably borrows the system-call enable flag.
    #[inline]
    pub const fn sce_mut(&mut self) -> EferSceMut<'_> {
        let &mut Self(ref mut target_value) = self;

        target_value.sce_mut()
    }

    /// Borrows the long-mode enable flag.
    #[inline]
    #[must_use]
    pub const fn lme(&self) -> EferLme<'_> {
        let &Self(ref target_value) = self;

        target_value.lme()
    }

    /// Mutably borrows the long-mode enable flag.
    #[inline]
    pub const fn lme_mut(&mut self) -> EferLmeMut<'_> {
        let &mut Self(ref mut target_value) = self;

        target_value.lme_mut()
    }

    /// Borrows the processor-maintained long-mode active flag.
    #[inline]
    #[must_use]
    pub const fn lma(&self) -> EferLma<'_> {
        let &Self(ref target_value) = self;

        target_value.lma()
    }

    /// Borrows the no-execute enable flag.
    #[inline]
    #[must_use]
    pub const fn nxe(&self) -> EferNxe<'_> {
        let &Self(ref target_value) = self;

        target_value.nxe()
    }

    /// Mutably borrows the no-execute enable flag.
    #[inline]
    pub const fn nxe_mut(&mut self) -> EferNxeMut<'_> {
        let &mut Self(ref mut target_value) = self;

        target_value.nxe_mut()
    }

    /// Lowers this value into the exact EFER image.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> RawEfer {
        let Self(target_value) = self;

        target_value
    }
}

/// EFER value proven suitable for restoring an active long-mode context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The wrapped EFER image always has both LME and LMA set. Construction checks
// LME and writes LMA only through the raw representation used for saved-state transport.
pub struct LongModeEfer(Efer);

impl LongModeEfer {
    /// Creates an active long-mode EFER proof when LME is enabled.
    #[inline]
    #[must_use]
    pub const fn new(efer: Efer) -> Option<Self> {
        match efer.lme().const_state() {
            State::Set => {
                let mut target_value = efer.raw();

                target_value.lma_mut().const_set(State::Set);

                Some(Self(Efer::lift(target_value)))
            },
            State::Cleared => None,
        }
    }

    /// Lowers this proof into the exact saved EFER image.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> RawEfer {
        let Self(efer) = self;

        efer.raw()
    }
}

#[cfg(test)]
mod tests {
    use nekor_bitwise::prelude::State;

    use super::{Efer, LongModeEfer, Msr, RawEfer};

    #[test]
    fn long_mode_requires_enable_and_sets_active_restore_state() {
        let inactive = LongModeEfer::new(Efer::RESET);
        let mut enabled = Efer::RESET;

        enabled.lme_mut().const_set(State::Set);

        let active = LongModeEfer::new(enabled);
        let raw = active.map(LongModeEfer::raw);

        assert_eq!(inactive, None);
        assert_eq!(raw.map(RawEfer::raw), Some(0x500));
    }

    #[test]
    fn semantic_mutation_excludes_lma() {
        let mut efer = Efer::RESET;

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
        let mut target_value = RawEfer::new(u64::MAX);

        target_value.lma_mut().const_set(State::Cleared);

        assert_eq!(target_value.lma().const_state(), State::Cleared);
    }

    #[test]
    fn register_index_matches_architecture() {
        assert_eq!(RawEfer::REGISTER.address(), 0xC000_0080);
    }
}

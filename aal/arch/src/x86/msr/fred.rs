//! Flexible Return and Event Delivery model-specific register representations and access.
//!
//! Raw types preserve every architectural register snapshot. Checked values keep
//! address alignment and policy constraints out of the raw transport layer.

use nekor_bitwise::prelude::{Bit, Counterpart, Field};

use crate::x86::{
    fred::FredLevel,
    msr::{FredAccess, UnavailableAccess, detail::snapshotted},
};
#[cfg(target_arch = "x86_64")]
use crate::x86_64::paging::{La, LaMode};

/// Current stack-level field in RawFredConfig.
pub type FredConfigLevel<'value> = Field<'value, 0, 1, u64>;

/// Reserved bit two in RawFredConfig.
pub type FredConfigReserved2<'value> = Bit<'value, u64, 2>;

/// Same-stack shadow-stack decrement control in RawFredConfig.
pub type FredConfigShadow<'value> = Bit<'value, u64, 3>;

/// Reserved bits four and five in RawFredConfig.
pub type FredConfigReserved4_5<'value> = Field<'value, 4, 5, u64>;

/// Same-stack regular-stack decrement count in RawFredConfig.
pub type FredConfigRegular<'value> = Field<'value, 6, 8, u64>;

/// Ring-zero maskable-interrupt stack level in RawFredConfig.
pub type FredConfigInterrupt<'value> = Field<'value, 9, 10, u64>;

/// Reserved bit eleven in RawFredConfig.
pub type FredConfigReserved11<'value> = Bit<'value, u64, 11>;

/// Event-handler page field in RawFredConfig.
pub type FredConfigHandler<'value> = Field<'value, 12, 63, u64>;

/// Mutable counterpart to FredConfigLevel.
pub type FredConfigLevelMut<'value> = <FredConfigLevel<'value> as Counterpart>::Mut;

/// Mutable counterpart to FredConfigReserved2.
pub type FredConfigReserved2Mut<'value> = <FredConfigReserved2<'value> as Counterpart>::Mut;

/// Mutable counterpart to FredConfigShadow.
pub type FredConfigShadowMut<'value> = <FredConfigShadow<'value> as Counterpart>::Mut;

/// Mutable counterpart to FredConfigReserved4_5.
pub type FredConfigReserved4_5Mut<'value> = <FredConfigReserved4_5<'value> as Counterpart>::Mut;

/// Mutable counterpart to FredConfigRegular.
pub type FredConfigRegularMut<'value> = <FredConfigRegular<'value> as Counterpart>::Mut;

/// Mutable counterpart to FredConfigInterrupt.
pub type FredConfigInterruptMut<'value> = <FredConfigInterrupt<'value> as Counterpart>::Mut;

/// Mutable counterpart to FredConfigReserved11.
pub type FredConfigReserved11Mut<'value> = <FredConfigReserved11<'value> as Counterpart>::Mut;

/// Mutable counterpart to FredConfigHandler.
pub type FredConfigHandlerMut<'value> = <FredConfigHandler<'value> as Counterpart>::Mut;

/// Low page-offset classification used by checked `FRED` handler construction.
type FredHandlerOffset<'value> = Field<'value, 0, 11, u64>;

snapshotted::snapshot!(
    /// `IA32_FRED_CONFIG` register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct RawFredConfig for (0x0000_01D4) use FredAccess
);

impl RawFredConfig {
    /// Borrows the current stack-level field.
    #[inline]
    #[must_use]
    pub const fn level(&self) -> FredConfigLevel<'_> {
        let &Self(ref target_value) = self;

        FredConfigLevel::wrap(target_value)
    }

    /// Mutably borrows the current stack-level field.
    #[inline]
    pub const fn level_mut(&mut self) -> FredConfigLevelMut<'_> {
        let &mut Self(ref mut target_value) = self;

        FredConfigLevelMut::wrap(target_value)
    }

    /// Borrows reserved bit two.
    #[inline]
    #[must_use]
    pub const fn reserved2(&self) -> FredConfigReserved2<'_> {
        let &Self(ref target_value) = self;

        FredConfigReserved2::wrap(target_value)
    }

    /// Mutably borrows reserved bit two.
    #[inline]
    pub const fn reserved2_mut(&mut self) -> FredConfigReserved2Mut<'_> {
        let &mut Self(ref mut target_value) = self;

        FredConfigReserved2Mut::wrap(target_value)
    }

    /// Borrows the same-stack shadow-stack decrement flag.
    #[inline]
    #[must_use]
    pub const fn shadow(&self) -> FredConfigShadow<'_> {
        let &Self(ref target_value) = self;

        FredConfigShadow::wrap(target_value)
    }

    /// Mutably borrows the same-stack shadow-stack decrement flag.
    #[inline]
    pub const fn shadow_mut(&mut self) -> FredConfigShadowMut<'_> {
        let &mut Self(ref mut target_value) = self;

        FredConfigShadowMut::wrap(target_value)
    }

    /// Borrows reserved bits four and five.
    #[inline]
    #[must_use]
    pub const fn reserved45(&self) -> FredConfigReserved4_5<'_> {
        let &Self(ref target_value) = self;

        FredConfigReserved4_5::wrap(target_value)
    }

    /// Mutably borrows reserved bits four and five.
    #[inline]
    pub const fn reserved45_mut(&mut self) -> FredConfigReserved4_5Mut<'_> {
        let &mut Self(ref mut target_value) = self;

        FredConfigReserved4_5Mut::wrap(target_value)
    }

    /// Borrows the same-stack regular decrement field.
    #[inline]
    #[must_use]
    pub const fn regular(&self) -> FredConfigRegular<'_> {
        let &Self(ref target_value) = self;

        FredConfigRegular::wrap(target_value)
    }

    /// Mutably borrows the same-stack regular decrement field.
    #[inline]
    pub const fn regular_mut(&mut self) -> FredConfigRegularMut<'_> {
        let &mut Self(ref mut target_value) = self;

        FredConfigRegularMut::wrap(target_value)
    }

    /// Borrows the maskable-interrupt stack-level field.
    #[inline]
    #[must_use]
    pub const fn interrupt(&self) -> FredConfigInterrupt<'_> {
        let &Self(ref target_value) = self;

        FredConfigInterrupt::wrap(target_value)
    }

    /// Mutably borrows the maskable-interrupt stack-level field.
    #[inline]
    pub const fn interrupt_mut(&mut self) -> FredConfigInterruptMut<'_> {
        let &mut Self(ref mut target_value) = self;

        FredConfigInterruptMut::wrap(target_value)
    }

    /// Borrows reserved bit eleven.
    #[inline]
    #[must_use]
    pub const fn reserved11(&self) -> FredConfigReserved11<'_> {
        let &Self(ref target_value) = self;

        FredConfigReserved11::wrap(target_value)
    }

    /// Mutably borrows reserved bit eleven.
    #[inline]
    pub const fn reserved11_mut(&mut self) -> FredConfigReserved11Mut<'_> {
        let &mut Self(ref mut target_value) = self;

        FredConfigReserved11Mut::wrap(target_value)
    }

    /// Borrows the event-handler page field.
    #[inline]
    #[must_use]
    pub const fn handler(&self) -> FredConfigHandler<'_> {
        let &Self(ref target_value) = self;

        FredConfigHandler::wrap(target_value)
    }

    /// Mutably borrows the event-handler page field.
    #[inline]
    pub const fn handler_mut(&mut self) -> FredConfigHandlerMut<'_> {
        let &mut Self(ref mut target_value) = self;

        FredConfigHandlerMut::wrap(target_value)
    }
}

/// Checked baseline `FRED` configuration.
#[cfg(target_arch = "x86_64")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
// NOTE(invariant): The handler is page aligned and canonical. Lowering clears reserved fields,
// current stack level, and same-stack decrement controls while preserving the selected interrupt
// stack level.
pub struct FredConfig {
    /// Canonical page-aligned event-handler address.
    handler: La,

    /// Stack level used for ring-zero maskable interrupts.
    interrupt: FredLevel,
}

#[cfg(target_arch = "x86_64")]
impl FredConfig {
    /// Creates a baseline `FRED` configuration.
    #[inline]
    #[must_use]
    pub const fn new(handler: La, interrupt: FredLevel) -> Option<Self> {
        let address = handler.bits();

        match FredHandlerOffset::wrap(&address).const_value() {
            0 => Some(Self { handler, interrupt }),
            _ => None,
        }
    }

    /// Returns the configured event-handler page.
    #[inline]
    #[must_use]
    pub const fn handler(self) -> La {
        let Self { handler, .. } = self;

        handler
    }

    /// Returns the maskable-interrupt stack level.
    #[inline]
    #[must_use]
    pub const fn interrupt(self) -> FredLevel {
        let Self { interrupt, .. } = self;

        interrupt
    }

    /// Lowers this checked configuration into the register snapshot.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> RawFredConfig {
        let Self { handler, interrupt } = self;

        let address = handler.bits();
        let page = FredConfigHandler::wrap(&address).const_value();
        let mut target_value = RawFredConfig::new(u64::MIN);

        target_value.handler_mut().const_merge(page);
        target_value.interrupt_mut().const_merge(interrupt.raw());

        target_value
    }
}

/// NMI stack-level field in RawFredLevels.
pub type FredLevelsNmi<'value> = Field<'value, 4, 5, u64>;

/// Double-fault stack-level field in RawFredLevels.
pub type FredLevelsDf<'value> = Field<'value, 16, 17, u64>;

/// Machine-check stack-level field in RawFredLevels.
pub type FredLevelsMc<'value> = Field<'value, 36, 37, u64>;

/// Mutable counterpart to FredLevelsNmi.
pub type FredLevelsNmiMut<'value> = <FredLevelsNmi<'value> as Counterpart>::Mut;

/// Mutable counterpart to FredLevelsDf.
pub type FredLevelsDfMut<'value> = <FredLevelsDf<'value> as Counterpart>::Mut;

/// Mutable counterpart to FredLevelsMc.
pub type FredLevelsMcMut<'value> = <FredLevelsMc<'value> as Counterpart>::Mut;

snapshotted::snapshot!(
    /// `IA32_FRED_STKLVLS` register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct RawFredLevels for (0x0000_01D0) use FredAccess
);

impl RawFredLevels {
    /// Borrows the NMI stack-level field.
    #[inline]
    #[must_use]
    pub const fn nmi(&self) -> FredLevelsNmi<'_> {
        let &Self(ref target_value) = self;

        FredLevelsNmi::wrap(target_value)
    }

    /// Mutably borrows the NMI stack-level field.
    #[inline]
    pub const fn nmi_mut(&mut self) -> FredLevelsNmiMut<'_> {
        let &mut Self(ref mut target_value) = self;

        FredLevelsNmiMut::wrap(target_value)
    }

    /// Borrows the double-fault stack-level field.
    #[inline]
    #[must_use]
    pub const fn df(&self) -> FredLevelsDf<'_> {
        let &Self(ref target_value) = self;

        FredLevelsDf::wrap(target_value)
    }

    /// Mutably borrows the double-fault stack-level field.
    #[inline]
    pub const fn df_mut(&mut self) -> FredLevelsDfMut<'_> {
        let &mut Self(ref mut target_value) = self;

        FredLevelsDfMut::wrap(target_value)
    }

    /// Borrows the machine-check stack-level field.
    #[inline]
    #[must_use]
    pub const fn mc(&self) -> FredLevelsMc<'_> {
        let &Self(ref target_value) = self;

        FredLevelsMc::wrap(target_value)
    }

    /// Mutably borrows the machine-check stack-level field.
    #[inline]
    pub const fn mc_mut(&mut self) -> FredLevelsMcMut<'_> {
        let &mut Self(ref mut target_value) = self;

        FredLevelsMcMut::wrap(target_value)
    }
}

/// Checked machine-safety `FRED` stack-level policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
// NOTE(invariant): Construction fixes NMI to level one, double fault to level two, machine check
// to level three, and leaves every other raw field clear when lowered.
pub struct FredLevels {
    /// Stack level selected for NMI delivery.
    nmi: FredLevel,

    /// Stack level selected for double-fault delivery.
    df: FredLevel,

    /// Stack level selected for machine-check delivery.
    mc: FredLevel,
}

impl FredLevels {
    /// Constructs the revision 1 machine-safety assignment.
    #[inline]
    #[must_use]
    pub const fn safety() -> Self {
        Self {
            nmi: FredLevel::One,
            df: FredLevel::Two,
            mc: FredLevel::Three,
        }
    }

    /// Returns the NMI stack level.
    #[inline]
    #[must_use]
    pub const fn nmi(self) -> FredLevel {
        let Self { nmi, .. } = self;

        nmi
    }

    /// Returns the double-fault stack level.
    #[inline]
    #[must_use]
    pub const fn df(self) -> FredLevel {
        let Self { df, .. } = self;

        df
    }

    /// Returns the machine-check stack level.
    #[inline]
    #[must_use]
    pub const fn mc(self) -> FredLevel {
        let Self { mc, .. } = self;

        mc
    }

    /// Lowers this policy into the stack-level register snapshot.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> RawFredLevels {
        let Self { nmi, df, mc } = self;

        let mut target_value = RawFredLevels::new(u64::MIN);

        target_value.nmi_mut().const_merge(nmi.raw());
        target_value.df_mut().const_merge(df.raw());
        target_value.mc_mut().const_merge(mc.raw());

        target_value
    }
}

/// Low six-bit alignment field used to validate `FRED` regular-stack pointers.
pub type FredRspOffset<'value> = Field<'value, 0, 5, u64>;

/// Implementation details for `FRED` regular-stack register pairs.
mod detail {
    /// Defines a raw and checked regular-stack register pair for a `FRED` level.
    macro_rules! stack {
        () => {};
        (
            $(
                #[$target_meta:meta]
            )*

            $target_vis:vis struct $target_name:ident for ($target_address:expr)
        ) => {
            tokel::stream!(
                $crate::x86::msr::detail::snapshotted::snapshot!(
                    /// `FRED` regular-stack register snapshot.
                    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
                    $target_vis struct [< Raw $target_name >]:flatten:concatenate for ($target_address)
                    use FredAccess
                );

                impl [< Raw $target_name >]:flatten:concatenate {
                    /// Architectural zero snapshot.
                    $target_vis const ZERO: Self = Self(u64::MIN);
                }

                $(
                    #[$target_meta]
                )*
                #[cfg(target_arch = "x86_64")]
                #[repr(transparent)]
                $target_vis struct $target_name(La);

                #[cfg(target_arch = "x86_64")]
                impl $target_name {
                    /// Creates a checked regular-stack pointer.
                    #[inline]
                    #[must_use]
                    $target_vis const fn new(address: La) -> Option<Self> {
                        let target_value = address.bits();

                        match FredRspOffset::wrap(&target_value).const_value() {
                            0 => Some(Self(address)),
                            _ => None,
                        }
                    }

                    /// Lifts a raw snapshot when it is canonical under M and properly aligned.
                    #[inline]
                    #[must_use]
                    $target_vis const fn lift<M>(target_value: [< Raw $target_name >]:flatten:concatenate) -> Option<Self>
                    where
                        M: LaMode,
                    {
                        let target_value = target_value.raw();

                        match (
                            FredRspOffset::wrap(&target_value).const_value(),
                            La::new::<M>(target_value),
                        ) {
                            (0, Some(address)) => Some(Self(address)),
                            _ => None,
                        }
                    }

                    /// Returns the canonical regular-stack address.
                    #[inline]
                    #[must_use]
                    $target_vis const fn la(self) -> La {
                        let Self(address) = self;

                        address
                    }

                    /// Lowers this checked pointer into the register snapshot.
                    #[inline]
                    #[must_use]
                    $target_vis const fn raw(self) -> [< Raw $target_name >]:flatten:concatenate {
                        let Self(address) = self;

                        <[< Raw $target_name >]:flatten:concatenate>::new(address.bits())
                    }
                }
            );
        };
    }

    pub(in crate::x86::msr::fred) use stack;
}

detail::stack!(
    /// Checked `FRED` regular-stack pointer for level 0.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct FredRsp0 for (0x0000_01CC)
);

detail::stack!(
    /// Checked `FRED` regular-stack pointer for level 1.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct FredRsp1 for (0x0000_01CD)
);

detail::stack!(
    /// Checked `FRED` regular-stack pointer for level 2.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct FredRsp2 for (0x0000_01CE)
);

detail::stack!(
    /// Checked `FRED` regular-stack pointer for level 3.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct FredRsp3 for (0x0000_01CF)
);

// NOTE: `FRED` shadow-stack access requires a capability not modeled by the `FRED` proof alone.
snapshotted::snapshot!(
    /// `IA32_FRED_SSP1` register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct RawFredSsp1 for (0x0000_01D1) use UnavailableAccess
);

snapshotted::snapshot!(
    /// `IA32_FRED_SSP2` register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct RawFredSsp2 for (0x0000_01D2) use UnavailableAccess
);

snapshotted::snapshot!(
    /// `IA32_FRED_SSP3` register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct RawFredSsp3 for (0x0000_01D3) use UnavailableAccess
);

#[cfg(test)]
mod tests {
    use nekor_bitwise::prelude::State;

    use super::{
        FredConfig, FredLevels, FredRsp1, RawFredConfig, RawFredLevels, RawFredRsp0, RawFredRsp1, RawFredRsp2,
        RawFredRsp3, RawFredSsp1, RawFredSsp2, RawFredSsp3,
    };
    use crate::{
        x86::{fred::FredLevel, msr::Msr},
        x86_64::paging::{La, La48},
    };

    #[test]
    fn config_lowers_without_representation_leaks() {
        let handler = La::new::<La48>(0xffff_ffff_8000_1000).expect("fixture handler page must be canonical");
        let config = FredConfig::new(handler, FredLevel::Zero).expect("page-aligned handler must configure FRED");
        let raw = config.raw();

        assert_eq!(config.handler(), handler);
        assert_eq!(config.interrupt(), FredLevel::Zero);
        assert_eq!(raw.level().const_value(), 0);
        assert_eq!(raw.interrupt().const_value(), 0);
        assert_eq!(raw.regular().const_value(), 0);
        assert_eq!(raw.shadow().const_state(), State::Cleared);
        assert_eq!(raw.reserved2().const_state(), State::Cleared);
        assert_eq!(raw.reserved45().const_value(), 0);
        assert_eq!(raw.reserved11().const_state(), State::Cleared);

        let unaligned = La::new::<La48>(handler.bits() + 8).expect("fixture address remains canonical");

        assert!(FredConfig::new(unaligned, FredLevel::Zero).is_none());
    }

    #[test]
    fn safety_levels_lower_to_named_fields() {
        let levels = FredLevels::safety();
        let raw = levels.raw();

        assert_eq!(levels.nmi(), FredLevel::One);
        assert_eq!(levels.df(), FredLevel::Two);
        assert_eq!(levels.mc(), FredLevel::Three);
        assert_eq!(raw.nmi().const_value(), FredLevel::One.raw());
        assert_eq!(raw.df().const_value(), FredLevel::Two.raw());
        assert_eq!(raw.mc().const_value(), FredLevel::Three.raw());
    }

    #[test]
    fn regular_stacks_require_sixty_four_byte_alignment() {
        let aligned = La::new::<La48>(0xffff_ffff_8000_2040).expect("fixture stack top must be canonical");
        let unaligned = La::new::<La48>(aligned.bits() + 8).expect("fixture address remains canonical");
        let rsp = FredRsp1::new(aligned).expect("aligned `FRED` stack top must validate");

        assert_eq!(rsp.la(), aligned);
        assert_eq!(FredRsp1::lift::<La48>(rsp.raw()).map(FredRsp1::la), Some(aligned));
        assert!(FredRsp1::new(unaligned).is_none());
    }

    #[test]
    fn raw_config_exposes_all_named_fields() {
        let mut target_value = RawFredConfig::new(u64::MIN);

        target_value.level_mut().const_merge(FredLevel::Three.raw());
        target_value.reserved2_mut().const_set(State::Set);
        target_value.shadow_mut().const_set(State::Set);
        target_value.reserved45_mut().const_merge(3);
        target_value.regular_mut().const_merge(7);
        target_value.interrupt_mut().const_merge(FredLevel::Two.raw());
        target_value.reserved11_mut().const_set(State::Set);
        target_value.handler_mut().const_merge(1);

        assert_eq!(target_value.level().const_value(), FredLevel::Three.raw());
        assert_eq!(target_value.reserved2().const_state(), State::Set);
        assert_eq!(target_value.shadow().const_state(), State::Set);
        assert_eq!(target_value.reserved45().const_value(), 3);
        assert_eq!(target_value.regular().const_value(), 7);
        assert_eq!(target_value.interrupt().const_value(), FredLevel::Two.raw());
        assert_eq!(target_value.reserved11().const_state(), State::Set);
        assert_eq!(target_value.handler().const_value(), 1);
    }

    #[test]
    fn register_indices_match_architecture() {
        assert_eq!(RawFredRsp0::REGISTER.address(), 0x1CC);
        assert_eq!(RawFredRsp1::REGISTER.address(), 0x1CD);
        assert_eq!(RawFredRsp2::REGISTER.address(), 0x1CE);
        assert_eq!(RawFredRsp3::REGISTER.address(), 0x1CF);
        assert_eq!(RawFredLevels::REGISTER.address(), 0x1D0);
        assert_eq!(RawFredSsp1::REGISTER.address(), 0x1D1);
        assert_eq!(RawFredSsp2::REGISTER.address(), 0x1D2);
        assert_eq!(RawFredSsp3::REGISTER.address(), 0x1D3);
        assert_eq!(RawFredConfig::REGISTER.address(), 0x1D4);
    }
}

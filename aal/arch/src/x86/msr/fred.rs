//! Flexible Return and Event Delivery model-specific register representations and access.
//!
//! The images expose every register field. Configuration and stack-address helpers
//! check their inputs while preparing bits, without claiming that entry resources are installed.

use nekor_bitwise::prelude::{Bit, Counterpart, Field};

use crate::x86::{fred::FredLevel, msr::detail::snapshotted};
#[cfg(target_arch = "x86_64")]
use crate::x86_64::paging::{La, LaMode};

/// The current stack-level field in `MsrFredConfig`.
pub type FredConfigLevel<'a> = Field<'a, 0, 1, u64>;

/// The reserved bit at position two in `MsrFredConfig`.
pub type FredConfigReserved2<'a> = Bit<'a, u64, 2>;

/// The same-stack shadow-stack decrement control in `MsrFredConfig`.
pub type FredConfigShadow<'a> = Bit<'a, u64, 3>;

/// The reserved bits at positions four and five in `MsrFredConfig`.
pub type FredConfigReserved4_5<'a> = Field<'a, 4, 5, u64>;

/// The same-stack regular-stack decrement count in `MsrFredConfig`.
pub type FredConfigRegular<'a> = Field<'a, 6, 8, u64>;

/// The ring-zero maskable-interrupt stack level in `MsrFredConfig`.
pub type FredConfigInterrupt<'a> = Field<'a, 9, 10, u64>;

/// The reserved bit at position eleven in `MsrFredConfig`.
pub type FredConfigReserved11<'a> = Bit<'a, u64, 11>;

/// The event-handler page field in `MsrFredConfig`.
pub type FredConfigHandler<'a> = Field<'a, 12, 63, u64>;

/// The mutable counterpart to `FredConfigLevel`.
pub type FredConfigLevelMut<'a> = <FredConfigLevel<'a> as Counterpart>::Mut;

/// The mutable counterpart to `FredConfigReserved2`.
pub type FredConfigReserved2Mut<'a> = <FredConfigReserved2<'a> as Counterpart>::Mut;

/// The mutable counterpart to `FredConfigShadow`.
pub type FredConfigShadowMut<'a> = <FredConfigShadow<'a> as Counterpart>::Mut;

/// The mutable counterpart to `FredConfigReserved4_5`.
pub type FredConfigReserved4_5Mut<'a> = <FredConfigReserved4_5<'a> as Counterpart>::Mut;

/// The mutable counterpart to `FredConfigRegular`.
pub type FredConfigRegularMut<'a> = <FredConfigRegular<'a> as Counterpart>::Mut;

/// The mutable counterpart to `FredConfigInterrupt`.
pub type FredConfigInterruptMut<'a> = <FredConfigInterrupt<'a> as Counterpart>::Mut;

/// The mutable counterpart to `FredConfigReserved11`.
pub type FredConfigReserved11Mut<'a> = <FredConfigReserved11<'a> as Counterpart>::Mut;

/// The mutable counterpart to `FredConfigHandler`.
pub type FredConfigHandlerMut<'a> = <FredConfigHandler<'a> as Counterpart>::Mut;

/// The page-offset field checked when constructing a FRED handler configuration.
type FredHandlerOffset<'a> = Field<'a, 0, 11, u64>;

snapshotted::snapshot!(
    /// The `IA32_FRED_CONFIG` event-delivery configuration model-specific register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MsrFredConfig for (0x0000_01D4)
    for<'a> (&'a crate::x86::privilege::Cpl<0, T>, &'a crate::x86::fred::FredCapability)
);

impl MsrFredConfig {
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

#[cfg(target_arch = "x86_64")]
impl MsrFredConfig {
    /// Prepare a baseline handler page and ring-zero interrupt stack level.
    ///
    /// The handler must be page aligned. Other fields begin clear, including the current
    /// stack level and same-stack decrement controls. This prepares an image only;
    /// installing it also requires the entry code, mappings and stack resources.
    #[inline]
    #[must_use]
    pub const fn configure(handler: La, interrupt: FredLevel) -> Option<Self> {
        let address = handler.bits();

        match FredHandlerOffset::wrap(&address).const_value() {
            u16::MIN => {
                let mut target_value = Self::new(u64::MIN);
                let page = FredConfigHandler::wrap(&address).const_value();

                target_value.handler_mut().const_merge(page);
                target_value.interrupt_mut().const_merge(interrupt.raw());

                Some(target_value)
            },
            _ => None,
        }
    }
}

/// The NMI stack-level field in `MsrFredLevels`.
pub type FredLevelsNmi<'a> = Field<'a, 4, 5, u64>;

/// The double-fault stack-level field in `MsrFredLevels`.
pub type FredLevelsDf<'a> = Field<'a, 16, 17, u64>;

/// The machine-check stack-level field in `MsrFredLevels`.
pub type FredLevelsMc<'a> = Field<'a, 36, 37, u64>;

/// The mutable counterpart to `FredLevelsNmi`.
pub type FredLevelsNmiMut<'a> = <FredLevelsNmi<'a> as Counterpart>::Mut;

/// The mutable counterpart to `FredLevelsDf`.
pub type FredLevelsDfMut<'a> = <FredLevelsDf<'a> as Counterpart>::Mut;

/// The mutable counterpart to `FredLevelsMc`.
pub type FredLevelsMcMut<'a> = <FredLevelsMc<'a> as Counterpart>::Mut;

snapshotted::snapshot!(
    /// The `IA32_FRED_STKLVLS` event stack-level selection model-specific register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MsrFredLevels for (0x0000_01D0)
    for<'a> (&'a crate::x86::privilege::Cpl<0, T>, &'a crate::x86::fred::FredCapability)
);

impl MsrFredLevels {
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

impl MsrFredLevels {
    /// Prepare distinct stack levels for NMI, double fault and machine check.
    ///
    /// NMI uses level one, double fault level two and machine check level three.
    /// The owner installing this policy must supply the corresponding stack resources.
    #[inline]
    #[must_use]
    pub const fn safety() -> Self {
        let mut target_value = Self::new(u64::MIN);

        target_value.nmi_mut().const_merge(FredLevel::One.raw());
        target_value.df_mut().const_merge(FredLevel::Two.raw());
        target_value.mc_mut().const_merge(FredLevel::Three.raw());

        target_value
    }
}

/// The alignment field checked when constructing FRED regular-stack pointers.
pub type FredRspOffset<'a> = Field<'a, 0, 5, u64>;

/// The declarations shared by the regular-stack registers.
mod detail {
    /// Declare a stack-pointer image with checked address helpers.
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
                    $(
                        #[$target_meta]
                    )*
                    $target_vis struct $target_name for ($target_address)
                    for<'a> (&'a $crate::x86::privilege::Cpl<0, T>, &'a $crate::x86::fred::FredCapability)
                );

                impl $target_name {
                    /// The architectural zero-image for this snapshot.
                    $target_vis const ZERO: Self = Self::new(u64::MIN);

                    /// Instantiate a regular-stack pointer for the corresponding FRED stack level.
                    ///
                    /// This requires a [`linear address`] that is aligned to a 64-byte boundary.
                    ///
                    /// [`linear address`]: La
                    #[cfg(target_arch = "x86_64")]
                    #[inline]
                    #[must_use]
                    $target_vis const fn at(address: La) -> Option<Self> {
                        let target_value = address.bits();

                        match FredRspOffset::wrap(&target_value).const_value() {
                            u8::MIN => Some(Self::new(target_value)),
                            _ => None,
                        }
                    }

                    /// Interpret the image as a canonical, 64-byte-aligned stack address.
                    #[cfg(target_arch = "x86_64")]
                    #[inline]
                    #[must_use]
                    $target_vis const fn la<M>(&self) -> Option<La>
                    where
                        M: LaMode,
                    {
                        let target_value = self.raw();

                        match FredRspOffset::wrap(&target_value).const_value() {
                            u8::MIN => La::new::<M>(target_value),
                            _ => None,
                        }
                    }
                }
            );
        };
    }

    pub(in crate::x86::msr::fred) use stack;
}

detail::stack!(
    /// The `IA32_FRED_RSP0` regular-stack pointer model-specific register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MsrFredRsp0 for (0x000001CC)
);

detail::stack!(
    /// The `IA32_FRED_RSP1` regular-stack pointer model-specific register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MsrFredRsp1 for (0x000001CD)
);

detail::stack!(
    /// The `IA32_FRED_RSP2` regular-stack pointer model-specific register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MsrFredRsp2 for (0x000001CE)
);

detail::stack!(
    /// The `IA32_FRED_RSP3` regular-stack pointer model-specific register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MsrFredRsp3 for (0x000001CF)
);

// NOTE: FRED shadow-stack access needs additional authorization beyond the
// [`FredCapability`](crate::x86::fred::FredCapability) proof-type.
snapshotted::snapshot!(
    /// The `IA32_FRED_SSP1` shadow-stack pointer model-specific register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MsrFredSsp1 for (0x0000_01D1)
);

snapshotted::snapshot!(
    /// The `IA32_FRED_SSP2` shadow-stack pointer model-specific register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MsrFredSsp2 for (0x0000_01D2)
);

snapshotted::snapshot!(
    /// The `IA32_FRED_SSP3` shadow-stack pointer model-specific register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MsrFredSsp3 for (0x0000_01D3)
);

#[cfg(test)]
mod tests {
    use nekor_bitwise::prelude::State;

    use crate::{
        x86::{
            fred::FredLevel,
            msr::{
                Msr,
                fred::{
                    MsrFredConfig, MsrFredLevels, MsrFredRsp0, MsrFredRsp1, MsrFredRsp2, MsrFredRsp3, MsrFredSsp1,
                    MsrFredSsp2, MsrFredSsp3,
                },
            },
        },
        x86_64::paging::{La, La48},
    };

    #[test]
    fn config_lowers_without_representation_leaks() {
        let handler = La::new::<La48>(0xffff_ffff_8000_1000).expect("fixture handler page must be canonical");
        let config =
            MsrFredConfig::configure(handler, FredLevel::Zero).expect("page-aligned handler must configure FRED");
        let raw = config;

        assert_eq!(raw.handler().const_value(), handler.bits() / 4096);
        assert_eq!(raw.level().const_value(), 0);
        assert_eq!(raw.interrupt().const_value(), 0);
        assert_eq!(raw.regular().const_value(), 0);
        assert_eq!(raw.shadow().const_state(), State::Cleared);
        assert_eq!(raw.reserved2().const_state(), State::Cleared);
        assert_eq!(raw.reserved45().const_value(), 0);
        assert_eq!(raw.reserved11().const_state(), State::Cleared);

        let unaligned = La::new::<La48>(handler.bits() + 8).expect("fixture address remains canonical");

        assert!(MsrFredConfig::configure(unaligned, FredLevel::Zero).is_none());
    }

    #[test]
    fn safety_levels_lower_to_named_fields() {
        let raw = MsrFredLevels::safety();

        assert_eq!(raw.nmi().const_value(), FredLevel::One.raw());
        assert_eq!(raw.df().const_value(), FredLevel::Two.raw());
        assert_eq!(raw.mc().const_value(), FredLevel::Three.raw());
    }

    #[test]
    fn regular_stacks_require_sixty_four_byte_alignment() {
        let aligned = La::new::<La48>(0xffff_ffff_8000_2040).expect("fixture stack top must be canonical");
        let unaligned = La::new::<La48>(aligned.bits() + 8).expect("fixture address remains canonical");
        let rsp = MsrFredRsp1::at(aligned).expect("aligned `FRED` stack top must validate");

        assert_eq!(rsp.la::<La48>(), Some(aligned));
        assert_eq!(MsrFredRsp1::new(rsp.raw()).la::<La48>(), Some(aligned));
        assert!(MsrFredRsp1::at(unaligned).is_none());
    }

    #[test]
    fn raw_config_exposes_all_named_fields() {
        let mut target_value = MsrFredConfig::new(u64::MIN);

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
        assert_eq!(MsrFredRsp0::REGISTER.address(), 0x1CC);
        assert_eq!(MsrFredRsp1::REGISTER.address(), 0x1CD);
        assert_eq!(MsrFredRsp2::REGISTER.address(), 0x1CE);
        assert_eq!(MsrFredRsp3::REGISTER.address(), 0x1CF);
        assert_eq!(MsrFredLevels::REGISTER.address(), 0x1D0);
        assert_eq!(MsrFredSsp1::REGISTER.address(), 0x1D1);
        assert_eq!(MsrFredSsp2::REGISTER.address(), 0x1D2);
        assert_eq!(MsrFredSsp3::REGISTER.address(), 0x1D3);
        assert_eq!(MsrFredConfig::REGISTER.address(), 0x1D4);
    }
}

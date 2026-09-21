//! Module for structured representation of architectural registers
//! and ease of interface with saved contexts.

use core::mem;

use nekor_bitwise::prelude::{Bit, Counterpart, Field, State};

/// The lower 8 bits of a general-purpose architectural register.
pub type GprLow8<'value> = Field<'value, 0, 7, u64>;

/// The higher 8 bits of a general-purpose architectural register.
pub type GprHigh8<'value> = Field<'value, 8, 15, u64>;

/// The lower 16 bits of a general-purpose architectural register.
pub type GprVal16<'value> = Field<'value, 0, 15, u64>;

/// The lower 32 bits of a general-purpose architectural register.
pub type GprVal32<'value> = Field<'value, 0, 31, u64>;

/// The full 64 bits of a general-purpose architectural register.
pub type GprVal64<'value> = Field<'value, 0, 63, u64>;

/// A mutable counterpart to [`GprLow8`].
pub type GprLow8Mut<'value> = <GprLow8<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`GprHigh8`].
pub type GprHigh8Mut<'value> = <GprHigh8<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`GprVal16`].
pub type GprVal16Mut<'value> = <GprVal16<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`GprVal32`].
pub type GprVal32Mut<'value> = <GprVal32<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`GprVal64`].
pub type GprVal64Mut<'value> = <GprVal64<'value> as Counterpart>::Mut;

/// A general-purpose architectural register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar preserves general-purpose register identity while every `u64`
// register image remains representable.
pub struct Gpr(u64);

/// The 64-bit instruction pointer register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar preserves RIP identity while every `u64` register image
// remains representable.
pub struct Rip(u64);

impl Rip {
    /// Creates an instruction pointer from its register image.
    #[inline]
    #[must_use]
    pub const fn new(target_value: u64) -> Self {
        Self(target_value)
    }

    /// Returns the instruction pointer register image.
    #[inline]
    #[must_use]
    pub const fn unwrap(self) -> u64 {
        let Self(target_value) = self;

        target_value
    }
}

impl Gpr {
    /// Creates a general-purpose register from its architectural
    /// value.
    #[inline]
    #[must_use]
    pub const fn new(target_value: u64) -> Self {
        Self(target_value)
    }

    /// Returns the architectural register value.
    #[inline]
    #[must_use]
    pub const fn unwrap(self) -> u64 {
        let Self(target_value) = self;

        target_value
    }

    /// Determine the lower 8 bits of this general-purpose register.
    #[inline]
    #[must_use]
    pub const fn low8(&self) -> GprLow8<'_> {
        let &Self(ref target_value) = self;

        GprLow8::wrap(target_value)
    }

    /// Determine the higher 8 bits of this general-purpose register.
    #[inline]
    #[must_use]
    pub const fn high8(&self) -> GprHigh8<'_> {
        let &Self(ref target_value) = self;

        GprHigh8::wrap(target_value)
    }

    /// Determine the lower 16 bits of this general-purpose register.
    #[inline]
    #[must_use]
    pub const fn val16(&self) -> GprVal16<'_> {
        let &Self(ref target_value) = self;

        GprVal16::wrap(target_value)
    }

    /// Determine the lower 32 bits of this general-purpose register.
    #[inline]
    #[must_use]
    pub const fn val32(&self) -> GprVal32<'_> {
        let &Self(ref target_value) = self;

        GprVal32::wrap(target_value)
    }

    /// Determine the full 64 bits of this general-purpose register.
    #[inline]
    #[must_use]
    pub const fn val64(&self) -> GprVal64<'_> {
        let &Self(ref target_value) = self;

        GprVal64::wrap(target_value)
    }

    /// Resolve a mutable reference to the lower 8 bits of this
    /// general-purpose register.
    #[inline]
    pub const fn low8_mut(&mut self) -> GprLow8Mut<'_> {
        let &mut Self(ref mut target_value) = self;

        GprLow8Mut::wrap(target_value)
    }

    /// Resolve a mutable reference to the higher 8 bits of this
    /// general-purpose register.
    #[inline]
    pub const fn high8_mut(&mut self) -> GprHigh8Mut<'_> {
        let &mut Self(ref mut target_value) = self;

        GprHigh8Mut::wrap(target_value)
    }

    /// Resolve a mutable reference to the lower 16 bits of this
    /// general-purpose register.
    #[inline]
    pub const fn val16_mut(&mut self) -> GprVal16Mut<'_> {
        let &mut Self(ref mut target_value) = self;

        GprVal16Mut::wrap(target_value)
    }

    /// Resolve a mutable reference to the lower 32 bits of this
    /// general-purpose register.
    #[inline]
    pub const fn val32_mut(&mut self) -> GprVal32Mut<'_> {
        let &mut Self(ref mut target_value) = self;

        GprVal32Mut::wrap(target_value)
    }

    /// Resolve a mutable reference to the full 64 bits of this
    /// general-purpose register.
    #[inline]
    pub const fn val64_mut(&mut self) -> GprVal64Mut<'_> {
        let &mut Self(ref mut target_value) = self;

        GprVal64Mut::wrap(target_value)
    }
}

/// The lower 8 bits of an extended segment register base.
pub type BaseLow8<'value> = Field<'value, 0, 7, u64>;

/// The higher 8 bits of an extended segment register base.
pub type BaseHigh8<'value> = Field<'value, 8, 15, u64>;

/// The lower 16 bits of an extended segment register base.
pub type BaseVal16<'value> = Field<'value, 0, 15, u64>;

/// The lower 32 bits of an extended segment register base.
pub type BaseVal32<'value> = Field<'value, 0, 31, u64>;

/// The full 64 bits of an extended segment register base.
pub type BaseVal64<'value> = Field<'value, 0, 63, u64>;

/// A mutable counterpart to [`BaseLow8`].
pub type BaseLow8Mut<'value> = <BaseLow8<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`BaseHigh8`].
pub type BaseHigh8Mut<'value> = <BaseHigh8<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`BaseVal16`].
pub type BaseVal16Mut<'value> = <BaseVal16<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`BaseVal32`].
pub type BaseVal32Mut<'value> = <BaseVal32<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`BaseVal64`].
pub type BaseVal64Mut<'value> = <BaseVal64<'value> as Counterpart>::Mut;

/// An extended segment register base.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar preserves segment-base identity while every `u64` register
// image remains representable.
pub struct Base(u64);

impl Base {
    /// Creates an extended segment base from its architectural value.
    #[inline]
    #[must_use]
    pub const fn new(target_value: u64) -> Self {
        Self(target_value)
    }

    /// Returns the architectural base value.
    #[inline]
    #[must_use]
    pub const fn unwrap(self) -> u64 {
        let Self(target_value) = self;

        target_value
    }

    /// Determine the lower 8 bits of this extended segment register base.
    #[inline]
    #[must_use]
    pub const fn low8(&self) -> BaseLow8<'_> {
        let &Self(ref target_value) = self;

        BaseLow8::wrap(target_value)
    }

    /// Determine the higher 8 bits of this extended segment register base.
    #[inline]
    #[must_use]
    pub const fn high8(&self) -> BaseHigh8<'_> {
        let &Self(ref target_value) = self;

        BaseHigh8::wrap(target_value)
    }

    /// Determine the lower 16 bits of this extended segment register base.
    #[inline]
    #[must_use]
    pub const fn val16(&self) -> BaseVal16<'_> {
        let &Self(ref target_value) = self;

        BaseVal16::wrap(target_value)
    }

    /// Determine the lower 32 bits of this extended segment register base.
    #[inline]
    #[must_use]
    pub const fn val32(&self) -> BaseVal32<'_> {
        let &Self(ref target_value) = self;

        BaseVal32::wrap(target_value)
    }

    /// Determine the full 64 bits of this extended segment register base.
    #[inline]
    #[must_use]
    pub const fn val64(&self) -> BaseVal64<'_> {
        let &Self(ref target_value) = self;

        BaseVal64::wrap(target_value)
    }

    /// Resolve a mutable reference to the lower 8 bits of this extended
    /// segment register base.
    #[inline]
    pub const fn low8_mut(&mut self) -> BaseLow8Mut<'_> {
        let &mut Self(ref mut target_value) = self;

        BaseLow8Mut::wrap(target_value)
    }

    /// Resolve a mutable reference to the higher 8 bits of this extended
    /// segment register base.
    #[inline]
    pub const fn high8_mut(&mut self) -> BaseHigh8Mut<'_> {
        let &mut Self(ref mut target_value) = self;

        BaseHigh8Mut::wrap(target_value)
    }

    /// Resolve a mutable reference to the lower 16 bits of this extended
    /// segment register base.
    #[inline]
    pub const fn val16_mut(&mut self) -> BaseVal16Mut<'_> {
        let &mut Self(ref mut target_value) = self;

        BaseVal16Mut::wrap(target_value)
    }

    /// Resolve a mutable reference to the lower 32 bits of this extended
    /// segment register base.
    #[inline]
    pub const fn val32_mut(&mut self) -> BaseVal32Mut<'_> {
        let &mut Self(ref mut target_value) = self;

        BaseVal32Mut::wrap(target_value)
    }

    /// Resolve a mutable reference to the full 64 bits of this extended
    /// segment register base.
    #[inline]
    pub const fn val64_mut(&mut self) -> BaseVal64Mut<'_> {
        let &mut Self(ref mut target_value) = self;

        BaseVal64Mut::wrap(target_value)
    }
}

/// Breakpoint condition zero sampled in [`RawDr6`].
pub type Dr6Bp0<'value> = Bit<'value, u64, 0>;

/// Breakpoint condition 1 sampled in [`RawDr6`].
pub type Dr6Bp1<'value> = Bit<'value, u64, 1>;

/// Breakpoint condition two sampled in [`RawDr6`].
pub type Dr6Bp2<'value> = Bit<'value, u64, 2>;

/// Breakpoint condition three sampled in [`RawDr6`].
pub type Dr6Bp3<'value> = Bit<'value, u64, 3>;

/// Reserved DR6 bits four through ten.
pub type Dr6Reserved4_10<'value> = Field<'value, 4, 10, u64>;

/// Bus-lock debug state in [`RawDr6`].
pub type Dr6Bld<'value> = Bit<'value, u64, 11>;

/// Reserved DR6 bit twelve.
pub type Dr6Reserved12<'value> = Bit<'value, u64, 12>;

/// Debug-register access-detected state in [`RawDr6`].
pub type Dr6Bd<'value> = Bit<'value, u64, 13>;

/// Single-step state in [`RawDr6`].
pub type Dr6Bs<'value> = Bit<'value, u64, 14>;

/// Task-switch debug state in [`RawDr6`].
pub type Dr6Bt<'value> = Bit<'value, u64, 15>;

/// Restricted-transactional-memory state in [`RawDr6`].
pub type Dr6Rtm<'value> = Bit<'value, u64, 16>;

/// Reserved DR6 bits seventeen through 63.
pub type Dr6Reserved17_63<'value> = Field<'value, 17, 63, u64>;

/// The 64-bit debug-status register image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): Every u64 hardware image remains representable, including reserved and
// feature-dependent bits.
pub struct RawDr6(u64);

impl RawDr6 {
    /// Constructs a raw DR6 image.
    #[inline]
    #[must_use]
    pub const fn new(target_value: u64) -> Self {
        Self(target_value)
    }

    /// Returns the raw DR6 image.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u64 {
        let Self(target_value) = self;

        target_value
    }

    /// Return breakpoint-condition zero state.
    #[inline]
    #[must_use]
    pub const fn b0(&self) -> Dr6Bp0<'_> {
        let &Self(ref target_value) = self;

        Dr6Bp0::wrap(target_value)
    }

    /// Return breakpoint-condition 1 state.
    #[inline]
    #[must_use]
    pub const fn b1(&self) -> Dr6Bp1<'_> {
        let &Self(ref target_value) = self;

        Dr6Bp1::wrap(target_value)
    }

    /// Return breakpoint-condition two state.
    #[inline]
    #[must_use]
    pub const fn b2(&self) -> Dr6Bp2<'_> {
        let &Self(ref target_value) = self;

        Dr6Bp2::wrap(target_value)
    }

    /// Return breakpoint-condition three state.
    #[inline]
    #[must_use]
    pub const fn b3(&self) -> Dr6Bp3<'_> {
        let &Self(ref target_value) = self;

        Dr6Bp3::wrap(target_value)
    }

    /// Return reserved bits four through ten.
    #[inline]
    #[must_use]
    pub const fn reserved_4_10(&self) -> Dr6Reserved4_10<'_> {
        let &Self(ref target_value) = self;

        Dr6Reserved4_10::wrap(target_value)
    }

    /// Return bus-lock debug state.
    #[inline]
    #[must_use]
    pub const fn bld(&self) -> Dr6Bld<'_> {
        let &Self(ref target_value) = self;

        Dr6Bld::wrap(target_value)
    }

    /// Return reserved bit twelve.
    #[inline]
    #[must_use]
    pub const fn reserved_12(&self) -> Dr6Reserved12<'_> {
        let &Self(ref target_value) = self;

        Dr6Reserved12::wrap(target_value)
    }

    /// Return debug-register access-detected state.
    #[inline]
    #[must_use]
    pub const fn bd(&self) -> Dr6Bd<'_> {
        let &Self(ref target_value) = self;

        Dr6Bd::wrap(target_value)
    }

    /// Return single-step state.
    #[inline]
    #[must_use]
    pub const fn bs(&self) -> Dr6Bs<'_> {
        let &Self(ref target_value) = self;

        Dr6Bs::wrap(target_value)
    }

    /// Return task-switch debug state.
    #[inline]
    #[must_use]
    pub const fn bt(&self) -> Dr6Bt<'_> {
        let &Self(ref target_value) = self;

        Dr6Bt::wrap(target_value)
    }

    /// Return restricted-transactional-memory state.
    #[inline]
    #[must_use]
    pub const fn rtm(&self) -> Dr6Rtm<'_> {
        let &Self(ref target_value) = self;

        Dr6Rtm::wrap(target_value)
    }

    /// Return reserved bits seventeen through 63.
    #[inline]
    #[must_use]
    pub const fn reserved_17_63(&self) -> Dr6Reserved17_63<'_> {
        let &Self(ref target_value) = self;

        Dr6Reserved17_63::wrap(target_value)
    }
}

/// The "Carry Flag" architectural flag.
pub type FlagCf<'value> = Bit<'value, u64, 0>;

/// The architecturally reserved bit 1 of [`Rflags`].
pub type FlagReserved<'value> = Bit<'value, u64, 1>;

/// A mutable counterpart to [`FlagReserved`].
pub type FlagReservedMut<'value> = <FlagReserved<'value> as Counterpart>::Mut;

/// The "Parity Flag" architectural flag.
pub type FlagPf<'value> = Bit<'value, u64, 2>;

/// The "Auxiliary Carry Flag" architectural flag.
pub type FlagAf<'value> = Bit<'value, u64, 4>;

/// The "Zero Flag" architectural flag.
pub type FlagZf<'value> = Bit<'value, u64, 6>;

/// The "Sign Flag" architectural flag.
pub type FlagSf<'value> = Bit<'value, u64, 7>;

/// The "Trap Flag" architectural flag.
pub type FlagTf<'value> = Bit<'value, u64, 8>;

/// The "Interrupt Enable Flag" architectural flag.
pub type FlagIf<'value> = Bit<'value, u64, 9>;

/// The "Direction Flag" architectural flag.
pub type FlagDf<'value> = Bit<'value, u64, 10>;

/// The "Overflow Flag" architectural flag.
pub type FlagOf<'value> = Bit<'value, u64, 11>;

/// The "I/O Privilege Level" architectural field.
pub type FieldIopl<'value> = Field<'value, 12, 13, u64>;

/// The "Nested Task Flag" architectural flag.
pub type FlagNt<'value> = Bit<'value, u64, 14>;

/// The "Resume Flag" architectural flag.
pub type FlagRf<'value> = Bit<'value, u64, 16>;

/// The "Virtual-8086 Mode Flag" architectural flag.
pub type FlagVm<'value> = Bit<'value, u64, 17>;

/// The "Alignment Check Flag" architectural flag.
pub type FlagAc<'value> = Bit<'value, u64, 18>;

/// The "Virtual Interrupt Flag" architectural flag.
pub type FlagVif<'value> = Bit<'value, u64, 19>;

/// The "Virtual Interrupt Pending Flag" architectural flag.
pub type FlagVip<'value> = Bit<'value, u64, 20>;

/// The "ID Flag" architectural flag.
pub type FlagId<'value> = Bit<'value, u64, 21>;

/// A mutable counterpart to [`FlagCf`].
pub type FlagCfMut<'value> = <FlagCf<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`FlagPf`].
pub type FlagPfMut<'value> = <FlagPf<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`FlagAf`].
pub type FlagAfMut<'value> = <FlagAf<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`FlagZf`].
pub type FlagZfMut<'value> = <FlagZf<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`FlagSf`].
pub type FlagSfMut<'value> = <FlagSf<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`FlagTf`].
pub type FlagTfMut<'value> = <FlagTf<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`FlagIf`].
pub type FlagIfMut<'value> = <FlagIf<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`FlagDf`].
pub type FlagDfMut<'value> = <FlagDf<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`FlagOf`].
pub type FlagOfMut<'value> = <FlagOf<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`FieldIopl`].
pub type FieldIoplMut<'value> = <FieldIopl<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`FlagNt`].
pub type FlagNtMut<'value> = <FlagNt<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`FlagRf`].
pub type FlagRfMut<'value> = <FlagRf<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`FlagVm`].
pub type FlagVmMut<'value> = <FlagVm<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`FlagAc`].
pub type FlagAcMut<'value> = <FlagAc<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`FlagVif`].
pub type FlagVifMut<'value> = <FlagVif<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`FlagVip`].
pub type FlagVipMut<'value> = <FlagVip<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`FlagId`].
pub type FlagIdMut<'value> = <FlagId<'value> as Counterpart>::Mut;

/// The 64-bit extended "FLAGS" register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): Safe construction preserves architectural reserved-bit requirements, while
// trusted raw reconstruction carries an explicit unsafe proof.
pub struct Rflags(u64);

/// Raw transport representation of [`Rflags`].
///
/// This type exists for hardware and ABI boundaries that must preserve the
/// register image without exposing a general raw constructor on
/// [`Rflags`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct RawRflags(pub u64);

impl RawRflags {
    /// Erases typed flags into their transport representation.
    #[inline]
    #[must_use]
    pub const fn take(target_value: Rflags) -> Self {
        let Rflags(target_value) = target_value;

        Self(target_value)
    }

    /// Reconstructs typed flags from a trusted architectural transport image.
    ///
    /// # Safety
    ///
    /// The stored bits must be an architectural RFLAGS image supplied by a
    /// trusted processor or virtualization boundary, or otherwise satisfy the
    /// architectural requirements for the state that will consume it.
    #[inline]
    #[must_use]
    pub const unsafe fn restore(self) -> Rflags {
        let Self(target_value) = self;

        // SAFETY: The caller supplies the architectural validity proof and the
        // transparent representation guarantees identical size and alignment.
        unsafe { mem::transmute_copy(&target_value) }
    }
}

impl Rflags {
    /// Architectural reset value.
    ///
    /// The processor defines bit 1 as reserved and fixed to 1 in the
    /// reset state. The value is constructed through [`FlagReserved`] rather
    /// than duplicating that architectural bit position as an integer shift.
    pub const RESET: Self = {
        let mut target_value = u64::MIN;
        let mut reserved = FlagReservedMut::wrap(&mut target_value);

        reserved.const_set(State::Set);

        Self(target_value)
    };

    /// Determine the "Carry Flag" (CF) state.
    #[inline]
    #[must_use]
    pub const fn flag_cf(&self) -> FlagCf<'_> {
        let &Self(ref target_value) = self;

        FlagCf::wrap(target_value)
    }

    /// Determine the architecturally reserved bit 1 state.
    #[inline]
    #[must_use]
    pub const fn flag_reserved(&self) -> FlagReserved<'_> {
        let &Self(ref target_value) = self;

        FlagReserved::wrap(target_value)
    }

    /// Determine the "Parity Flag" (PF) state.
    #[inline]
    #[must_use]
    pub const fn flag_pf(&self) -> FlagPf<'_> {
        let &Self(ref target_value) = self;

        FlagPf::wrap(target_value)
    }

    /// Determine the "Auxiliary Carry Flag" (AF) state.
    #[inline]
    #[must_use]
    pub const fn flag_af(&self) -> FlagAf<'_> {
        let &Self(ref target_value) = self;

        FlagAf::wrap(target_value)
    }

    /// Determine the "Zero Flag" (ZF) state.
    #[inline]
    #[must_use]
    pub const fn flag_zf(&self) -> FlagZf<'_> {
        let &Self(ref target_value) = self;

        FlagZf::wrap(target_value)
    }

    /// Determine the "Sign Flag" (SF) state.
    #[inline]
    #[must_use]
    pub const fn flag_sf(&self) -> FlagSf<'_> {
        let &Self(ref target_value) = self;

        FlagSf::wrap(target_value)
    }

    /// Determine the "Trap Flag" (TF) state.
    #[inline]
    #[must_use]
    pub const fn flag_tf(&self) -> FlagTf<'_> {
        let &Self(ref target_value) = self;

        FlagTf::wrap(target_value)
    }

    /// Determine the "Interrupt Enable Flag" (IF) state.
    #[inline]
    #[must_use]
    pub const fn flag_if(&self) -> FlagIf<'_> {
        let &Self(ref target_value) = self;

        FlagIf::wrap(target_value)
    }

    /// Determine the "Direction Flag" (DF) state.
    #[inline]
    #[must_use]
    pub const fn flag_df(&self) -> FlagDf<'_> {
        let &Self(ref target_value) = self;

        FlagDf::wrap(target_value)
    }

    /// Determine the "Overflow Flag" (OF) state.
    #[inline]
    #[must_use]
    pub const fn flag_of(&self) -> FlagOf<'_> {
        let &Self(ref target_value) = self;

        FlagOf::wrap(target_value)
    }

    /// Determine the "I/O Privilege Level" (IOPL) field.
    #[inline]
    #[must_use]
    pub const fn field_iopl(&self) -> FieldIopl<'_> {
        let &Self(ref target_value) = self;

        FieldIopl::wrap(target_value)
    }

    /// Determine the "Nested Task Flag" (NT) state.
    #[inline]
    #[must_use]
    pub const fn flag_nt(&self) -> FlagNt<'_> {
        let &Self(ref target_value) = self;

        FlagNt::wrap(target_value)
    }

    /// Determine the "Resume Flag" (RF) state.
    #[inline]
    #[must_use]
    pub const fn flag_rf(&self) -> FlagRf<'_> {
        let &Self(ref target_value) = self;

        FlagRf::wrap(target_value)
    }

    /// Determine the "Virtual-8086 Mode Flag" (VM) state.
    #[inline]
    #[must_use]
    pub const fn flag_vm(&self) -> FlagVm<'_> {
        let &Self(ref target_value) = self;

        FlagVm::wrap(target_value)
    }

    /// Determine the "Alignment Check Flag" (AC) state.
    #[inline]
    #[must_use]
    pub const fn flag_ac(&self) -> FlagAc<'_> {
        let &Self(ref target_value) = self;

        FlagAc::wrap(target_value)
    }

    /// Determine the "Virtual Interrupt Flag" (VIF) state.
    #[inline]
    #[must_use]
    pub const fn flag_vif(&self) -> FlagVif<'_> {
        let &Self(ref target_value) = self;

        FlagVif::wrap(target_value)
    }

    /// Determine the "Virtual Interrupt Pending Flag" (VIP) state.
    #[inline]
    #[must_use]
    pub const fn flag_vip(&self) -> FlagVip<'_> {
        let &Self(ref target_value) = self;

        FlagVip::wrap(target_value)
    }

    /// Determine the "ID Flag" (ID) state.
    #[inline]
    #[must_use]
    pub const fn flag_id(&self) -> FlagId<'_> {
        let &Self(ref target_value) = self;

        FlagId::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Carry Flag" (CF) state.
    #[inline]
    pub const fn flag_cf_mut(&mut self) -> FlagCfMut<'_> {
        let &mut Self(ref mut target_value) = self;

        FlagCfMut::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Parity Flag" (PF) state.
    #[inline]
    pub const fn flag_pf_mut(&mut self) -> FlagPfMut<'_> {
        let &mut Self(ref mut target_value) = self;

        FlagPfMut::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Auxiliary Carry Flag" (AF)
    /// state.
    #[inline]
    pub const fn flag_af_mut(&mut self) -> FlagAfMut<'_> {
        let &mut Self(ref mut target_value) = self;

        FlagAfMut::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Zero Flag" (ZF) state.
    #[inline]
    pub const fn flag_zf_mut(&mut self) -> FlagZfMut<'_> {
        let &mut Self(ref mut target_value) = self;

        FlagZfMut::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Sign Flag" (SF) state.
    #[inline]
    pub const fn flag_sf_mut(&mut self) -> FlagSfMut<'_> {
        let &mut Self(ref mut target_value) = self;

        FlagSfMut::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Trap Flag" (TF) state.
    #[inline]
    pub const fn flag_tf_mut(&mut self) -> FlagTfMut<'_> {
        let &mut Self(ref mut target_value) = self;

        FlagTfMut::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Interrupt Enable Flag" (IF)
    /// state.
    #[inline]
    pub const fn flag_if_mut(&mut self) -> FlagIfMut<'_> {
        let &mut Self(ref mut target_value) = self;

        FlagIfMut::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Direction Flag" (DF) state.
    #[inline]
    pub const fn flag_df_mut(&mut self) -> FlagDfMut<'_> {
        let &mut Self(ref mut target_value) = self;

        FlagDfMut::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Overflow Flag" (OF) state.
    #[inline]
    pub const fn flag_of_mut(&mut self) -> FlagOfMut<'_> {
        let &mut Self(ref mut target_value) = self;

        FlagOfMut::wrap(target_value)
    }

    /// Resolve a mutable reference to the "I/O Privilege Level" (IOPL)
    /// field.
    #[inline]
    pub const fn field_iopl_mut(&mut self) -> FieldIoplMut<'_> {
        let &mut Self(ref mut target_value) = self;

        FieldIoplMut::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Nested Task Flag" (NT) state.
    #[inline]
    pub const fn flag_nt_mut(&mut self) -> FlagNtMut<'_> {
        let &mut Self(ref mut target_value) = self;

        FlagNtMut::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Resume Flag" (RF) state.
    #[inline]
    pub const fn flag_rf_mut(&mut self) -> FlagRfMut<'_> {
        let &mut Self(ref mut target_value) = self;

        FlagRfMut::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Virtual-8086 Mode Flag" (VM)
    /// state.
    #[inline]
    pub const fn flag_vm_mut(&mut self) -> FlagVmMut<'_> {
        let &mut Self(ref mut target_value) = self;

        FlagVmMut::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Alignment Check Flag" (AC)
    /// state.
    #[inline]
    pub const fn flag_ac_mut(&mut self) -> FlagAcMut<'_> {
        let &mut Self(ref mut target_value) = self;

        FlagAcMut::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Virtual Interrupt Flag" (VIF)
    /// state.
    #[inline]
    pub const fn flag_vif_mut(&mut self) -> FlagVifMut<'_> {
        let &mut Self(ref mut target_value) = self;

        FlagVifMut::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Virtual Interrupt Pending Flag"
    /// (VIP) state.
    #[inline]
    pub const fn flag_vip_mut(&mut self) -> FlagVipMut<'_> {
        let &mut Self(ref mut target_value) = self;

        FlagVipMut::wrap(target_value)
    }

    /// Resolve a mutable reference to the "ID Flag" (ID) state.
    #[inline]
    pub const fn flag_id_mut(&mut self) -> FlagIdMut<'_> {
        let &mut Self(ref mut target_value) = self;

        FlagIdMut::wrap(target_value)
    }
}
#[cfg(test)]
mod tests {
    use nekor_bitwise::prelude::{Counterpart, State};

    use super::{Dr6Bd, Dr6Bld, Dr6Bp0, Dr6Bs, Dr6Bt, Dr6Rtm, RawDr6, RawRflags, Rflags, Rip};

    #[test]
    fn dr6_views_preserve_named_debug_state() {
        let mut bits = u64::MIN;
        let mut b0 = <Dr6Bp0<'_> as Counterpart>::Mut::wrap(&mut bits);

        b0.const_set(State::Set);

        let mut bld = <Dr6Bld<'_> as Counterpart>::Mut::wrap(&mut bits);

        bld.const_set(State::Set);

        let mut bd = <Dr6Bd<'_> as Counterpart>::Mut::wrap(&mut bits);

        bd.const_set(State::Set);

        let mut bs = <Dr6Bs<'_> as Counterpart>::Mut::wrap(&mut bits);

        bs.const_set(State::Set);

        let mut bt = <Dr6Bt<'_> as Counterpart>::Mut::wrap(&mut bits);

        bt.const_set(State::Set);

        let mut rtm = <Dr6Rtm<'_> as Counterpart>::Mut::wrap(&mut bits);

        rtm.const_set(State::Set);

        let dr6 = RawDr6::new(bits);

        assert_eq!(dr6.b0().const_state(), State::Set);
        assert_eq!(dr6.bld().const_state(), State::Set);
        assert_eq!(dr6.bd().const_state(), State::Set);
        assert_eq!(dr6.bs().const_state(), State::Set);
        assert_eq!(dr6.bt().const_state(), State::Set);
        assert_eq!(dr6.rtm().const_state(), State::Set);

        assert_eq!(dr6.raw(), bits);
    }

    #[test]
    fn instruction_pointer_preserves_complete_register_image() {
        let rip = Rip::new(0xffff_ffff_ffff_fff0);

        assert_eq!(rip.unwrap(), 0xffff_ffff_ffff_fff0);
    }

    #[test]
    fn rflags_transport_round_trips_trusted_images() {
        let raw = RawRflags::take(Rflags::RESET);

        let RawRflags(bits) = raw;

        assert_eq!(bits, 2);

        // SAFETY: The bits were produced from the typed reset value immediately above.
        let restored = unsafe { RawRflags(bits).restore() };

        assert_eq!(restored, Rflags::RESET);
    }

    #[test]
    fn rflags_reset_models_reserved_bit_one_explicitly() {
        let flags = Rflags::RESET;

        assert_eq!(flags.flag_reserved().state(), State::Set);
        assert_eq!(flags.flag_cf().state(), State::Cleared);
        assert_eq!(flags.flag_pf().state(), State::Cleared);
    }
}

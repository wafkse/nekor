//! Module for structured representation of architectural registers
//! and ease of interface with saved contexts.

use nekor_bitwise::prelude::{Bit, BitMut, Field, FieldMut};

/// The lower 8 bits of a general-purpose architectural register.
pub type GprLow8<'a> = Field<'a, 0, 7, u64>;

/// The higher 8 bits of a general-purpose architectural register.
pub type GprHigh8<'a> = Field<'a, 8, 15, u64>;

/// The lower 16 bits of a general-purpose architectural register.
pub type GprVal16<'a> = Field<'a, 0, 15, u64>;

/// The lower 32 bits of a general-purpose architectural register.
pub type GprVal32<'a> = Field<'a, 0, 31, u64>;

/// The full 64 bits of a general-purpose architectural register.
pub type GprVal64<'a> = Field<'a, 0, 63, u64>;

/// A mutable counterpart to [`GprLow8`].
pub type GprLow8Mut<'a> = FieldMut<'a, 0, 7, u64>;

/// A mutable counterpart to [`GprHigh8`].
pub type GprHigh8Mut<'a> = FieldMut<'a, 8, 15, u64>;

/// A mutable counterpart to [`GprVal16`].
pub type GprVal16Mut<'a> = FieldMut<'a, 0, 15, u64>;

/// A mutable counterpart to [`GprVal32`].
pub type GprVal32Mut<'a> = FieldMut<'a, 0, 31, u64>;

/// A mutable counterpart to [`GprVal64`].
pub type GprVal64Mut<'a> = FieldMut<'a, 0, 63, u64>;

/// A general-purpose architectural register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Gpr(u64);

impl Gpr {
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
pub type BaseLow8<'a> = Field<'a, 0, 7, u64>;

/// The higher 8 bits of an extended segment register base.
pub type BaseHigh8<'a> = Field<'a, 8, 15, u64>;

/// The lower 16 bits of an extended segment register base.
pub type BaseVal16<'a> = Field<'a, 0, 15, u64>;

/// The lower 32 bits of an extended segment register base.
pub type BaseVal32<'a> = Field<'a, 0, 31, u64>;

/// The full 64 bits of an extended segment register base.
pub type BaseVal64<'a> = Field<'a, 0, 63, u64>;

/// A mutable counterpart to [`BaseLow8`].
pub type BaseLow8Mut<'a> = FieldMut<'a, 0, 7, u64>;

/// A mutable counterpart to [`BaseHigh8`].
pub type BaseHigh8Mut<'a> = FieldMut<'a, 8, 15, u64>;

/// A mutable counterpart to [`BaseVal16`].
pub type BaseVal16Mut<'a> = FieldMut<'a, 0, 15, u64>;

/// A mutable counterpart to [`BaseVal32`].
pub type BaseVal32Mut<'a> = FieldMut<'a, 0, 31, u64>;

/// A mutable counterpart to [`BaseVal64`].
pub type BaseVal64Mut<'a> = FieldMut<'a, 0, 63, u64>;

/// An extended segment register base.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Base(u64);

impl Base {
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

/// The "Carry Flag" architectural flag.
pub type FlagCf<'a> = Bit<'a, u64, 0>;

/// The "Parity Flag" architectural flag.
pub type FlagPf<'a> = Bit<'a, u64, 2>;

/// The "Auxiliary Carry Flag" architectural flag.
pub type FlagAf<'a> = Bit<'a, u64, 4>;

/// The "Zero Flag" architectural flag.
pub type FlagZf<'a> = Bit<'a, u64, 6>;

/// The "Sign Flag" architectural flag.
pub type FlagSf<'a> = Bit<'a, u64, 7>;

/// The "Trap Flag" architectural flag.
pub type FlagTf<'a> = Bit<'a, u64, 8>;

/// The "Interrupt Enable Flag" architectural flag.
pub type FlagIf<'a> = Bit<'a, u64, 9>;

/// The "Direction Flag" architectural flag.
pub type FlagDf<'a> = Bit<'a, u64, 10>;

/// The "Overflow Flag" architectural flag.
pub type FlagOf<'a> = Bit<'a, u64, 11>;

/// The "I/O Privilege Level" architectural field.
pub type FieldIopl<'a> = Field<'a, 12, 13, u64>;

/// The "Nested Task Flag" architectural flag.
pub type FlagNt<'a> = Bit<'a, u64, 14>;

/// The "Resume Flag" architectural flag.
pub type FlagRf<'a> = Bit<'a, u64, 16>;

/// The "Virtual-8086 Mode Flag" architectural flag.
pub type FlagVm<'a> = Bit<'a, u64, 17>;

/// The "Alignment Check Flag" architectural flag.
pub type FlagAc<'a> = Bit<'a, u64, 18>;

/// The "Virtual Interrupt Flag" architectural flag.
pub type FlagVif<'a> = Bit<'a, u64, 19>;

/// The "Virtual Interrupt Pending Flag" architectural flag.
pub type FlagVip<'a> = Bit<'a, u64, 20>;

/// The "ID Flag" architectural flag.
pub type FlagId<'a> = Bit<'a, u64, 21>;

/// A mutable counterpart to [`FlagCf`].
pub type FlagCfMut<'a> = BitMut<'a, u64, 0>;

/// A mutable counterpart to [`FlagPf`].
pub type FlagPfMut<'a> = BitMut<'a, u64, 2>;

/// A mutable counterpart to [`FlagAf`].
pub type FlagAfMut<'a> = BitMut<'a, u64, 4>;

/// A mutable counterpart to [`FlagZf`].
pub type FlagZfMut<'a> = BitMut<'a, u64, 6>;

/// A mutable counterpart to [`FlagSf`].
pub type FlagSfMut<'a> = BitMut<'a, u64, 7>;

/// A mutable counterpart to [`FlagTf`].
pub type FlagTfMut<'a> = BitMut<'a, u64, 8>;

/// A mutable counterpart to [`FlagIf`].
pub type FlagIfMut<'a> = BitMut<'a, u64, 9>;

/// A mutable counterpart to [`FlagDf`].
pub type FlagDfMut<'a> = BitMut<'a, u64, 10>;

/// A mutable counterpart to [`FlagOf`].
pub type FlagOfMut<'a> = BitMut<'a, u64, 11>;

/// A mutable counterpart to [`FieldIopl`].
pub type FieldIoplMut<'a> = FieldMut<'a, 12, 13, u64>;

/// A mutable counterpart to [`FlagNt`].
pub type FlagNtMut<'a> = BitMut<'a, u64, 14>;

/// A mutable counterpart to [`FlagRf`].
pub type FlagRfMut<'a> = BitMut<'a, u64, 16>;

/// A mutable counterpart to [`FlagVm`].
pub type FlagVmMut<'a> = BitMut<'a, u64, 17>;

/// A mutable counterpart to [`FlagAc`].
pub type FlagAcMut<'a> = BitMut<'a, u64, 18>;

/// A mutable counterpart to [`FlagVif`].
pub type FlagVifMut<'a> = BitMut<'a, u64, 19>;

/// A mutable counterpart to [`FlagVip`].
pub type FlagVipMut<'a> = BitMut<'a, u64, 20>;

/// A mutable counterpart to [`FlagId`].
pub type FlagIdMut<'a> = BitMut<'a, u64, 21>;

/// The 64-bit extended "FLAGS" register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Rflags(u64);

impl Rflags {
    /// Determine the "Carry Flag" (CF) state.
    #[inline]
    #[must_use]
    pub const fn flag_cf(&self) -> FlagCf<'_> {
        let &Self(ref target_value) = self;

        FlagCf::wrap(target_value)
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

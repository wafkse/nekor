//! Structured x86-64 control-register representations.
//!
//! This module represents architectural control registers as their exact
//! integer storage together with typed [`nekor_bitwise`] views over individual
//! architectural fields.
//!
//! The register types intentionally provide no whole-register raw constructor
//! or getter. Callers compose architectural state through named fields instead
//! of bypassing the representation with integer masks.

use core::mem;

use nekor_bitwise::prelude::{Bit, Counterpart, Field, State};

use crate::{x86::msr::efer::LongModeEfer, x86_64::paging::Pa};

/// The "Protection Enable" (PE) flag in [`Cr0`].
pub type Cr0Pe<'value> = Bit<'value, u64, 0>;

/// The "Monitor Coprocessor" (MP) flag in [`Cr0`].
pub type Cr0Mp<'value> = Bit<'value, u64, 1>;

/// The "Emulation" (EM) flag in [`Cr0`].
pub type Cr0Em<'value> = Bit<'value, u64, 2>;

/// The "Task Switched" (TS) flag in [`Cr0`].
pub type Cr0Ts<'value> = Bit<'value, u64, 3>;

/// The historical "Extension Type" (ET) bit in [`Cr0`].
///
/// On modern x86 processors this architectural bit is reserved and fixed to
/// one. It is therefore exposed for inspection without a mutable counterpart.
pub type Cr0Et<'value> = Bit<'value, u64, 4>;

/// Internal mutable counterpart used only to construct architectural values.
type Cr0EtMut<'value> = <Cr0Et<'value> as Counterpart>::Mut;

/// The "Numeric Error" (NE) flag in [`Cr0`].
pub type Cr0Ne<'value> = Bit<'value, u64, 5>;

/// Reserved CR0 field spanning bits 6 through 15.
pub type Cr0Reserved6_15<'value> = Field<'value, 6, 15, u64>;

/// The "Write Protect" (WP) flag in [`Cr0`].
pub type Cr0Wp<'value> = Bit<'value, u64, 16>;

/// Architecturally reserved CR0 bit 17.
pub type Cr0Reserved17<'value> = Bit<'value, u64, 17>;

/// The "Alignment Mask" (AM) flag in [`Cr0`].
pub type Cr0Am<'value> = Bit<'value, u64, 18>;

/// Reserved CR0 field spanning bits 19 through 28.
pub type Cr0Reserved19_28<'value> = Field<'value, 19, 28, u64>;

/// The "Not Write-through" (NW) flag in [`Cr0`].
pub type Cr0Nw<'value> = Bit<'value, u64, 29>;

/// The "Cache Disable" (CD) flag in [`Cr0`].
pub type Cr0Cd<'value> = Bit<'value, u64, 30>;

/// The "Paging" (PG) flag in [`Cr0`].
pub type Cr0Pg<'value> = Bit<'value, u64, 31>;

/// Reserved upper half of [`Cr0`].
pub type Cr0Reserved32_63<'value> = Field<'value, 32, 63, u64>;
/// A mutable counterpart to [`Cr0Pe`].
pub type Cr0PeMut<'value> = <Cr0Pe<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`Cr0Mp`].
pub type Cr0MpMut<'value> = <Cr0Mp<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`Cr0Em`].
pub type Cr0EmMut<'value> = <Cr0Em<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`Cr0Ts`].
pub type Cr0TsMut<'value> = <Cr0Ts<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`Cr0Ne`].
pub type Cr0NeMut<'value> = <Cr0Ne<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`Cr0Wp`].
pub type Cr0WpMut<'value> = <Cr0Wp<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`Cr0Am`].
pub type Cr0AmMut<'value> = <Cr0Am<'value> as Counterpart>::Mut;
/// A mutable counterpart to [`Cr0Nw`].
pub type Cr0NwMut<'value> = <Cr0Nw<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`Cr0Cd`].
pub type Cr0CdMut<'value> = <Cr0Cd<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`Cr0Pg`].
pub type Cr0PgMut<'value> = <Cr0Pg<'value> as Counterpart>::Mut;

/// The 64-bit CR0 control register.
///
/// # Layout
///
/// CR0 controls the processor's basic execution mode, cache policy, alignment
/// checking, write protection, and paging enable state. Reserved fields remain
/// inspectable but have no mutable accessors through this representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): Safe construction preserves CR0 fixed and reserved fields, while trusted raw
// reconstruction carries an explicit unsafe proof.
pub struct Cr0(u64);

/// Raw transport representation of [`Cr0`].
///
/// This type preserves the complete control-register image for hardware and
/// virtualization boundaries without adding a general raw constructor to
/// [`Cr0`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct RawCr0(pub u64);

impl RawCr0 {
    /// Erases typed CR0 state into its complete transport representation.
    #[inline]
    #[must_use]
    pub const fn take(value: Cr0) -> Self {
        let Cr0(value) = value;
        Self(value)
    }

    /// Reconstructs typed CR0 state from a trusted architectural image.
    ///
    /// # Safety
    ///
    /// The stored bits must come from a trusted processor or virtualization
    /// boundary, or otherwise satisfy the architectural CR0 requirements for
    /// the state that will consume them.
    #[inline]
    #[must_use]
    pub const unsafe fn restore(self) -> Cr0 {
        let Self(value) = self;

        // SAFETY: The caller supplies the architectural validity proof and the
        // transparent representation guarantees identical size and alignment.
        unsafe { mem::transmute_copy(&value) }
    }
}

impl Cr0 {
    /// Architectural reset value.
    ///
    /// ET is fixed to one. The architectural reset state also starts with NW
    /// and CD set. The value is assembled through the named bit aliases rather
    /// than duplicated as an integer literal.
    pub const RESET: Self = {
        let mut value = u64::MIN;
        let mut et = Cr0EtMut::wrap(&mut value);

        et.const_set(State::Set);

        let mut nw = Cr0NwMut::wrap(&mut value);

        nw.const_set(State::Set);

        let mut cd = Cr0CdMut::wrap(&mut value);

        cd.const_set(State::Set);

        Self(value)
    };

    /// Determine the "Protection Enable" (PE) flag.
    #[inline]
    #[must_use]
    pub const fn flag_pe(&self) -> Cr0Pe<'_> {
        let &Self(ref target_value) = self;

        Cr0Pe::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Protection Enable" (PE) flag.
    #[inline]
    pub const fn flag_pe_mut(&mut self) -> Cr0PeMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr0PeMut::wrap(target_value)
    }

    /// Determine the "Monitor Coprocessor" (MP) flag.
    #[inline]
    #[must_use]
    pub const fn flag_mp(&self) -> Cr0Mp<'_> {
        let &Self(ref target_value) = self;

        Cr0Mp::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Monitor Coprocessor" (MP) flag.
    #[inline]
    pub const fn flag_mp_mut(&mut self) -> Cr0MpMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr0MpMut::wrap(target_value)
    }

    /// Determine the "Emulation" (EM) flag.
    #[inline]
    #[must_use]
    pub const fn flag_em(&self) -> Cr0Em<'_> {
        let &Self(ref target_value) = self;

        Cr0Em::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Emulation" (EM) flag.
    #[inline]
    pub const fn flag_em_mut(&mut self) -> Cr0EmMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr0EmMut::wrap(target_value)
    }

    /// Determine the "Task Switched" (TS) flag.
    #[inline]
    #[must_use]
    pub const fn flag_ts(&self) -> Cr0Ts<'_> {
        let &Self(ref target_value) = self;

        Cr0Ts::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Task Switched" (TS) flag.
    #[inline]
    pub const fn flag_ts_mut(&mut self) -> Cr0TsMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr0TsMut::wrap(target_value)
    }

    /// Determine the "Extension Type" (ET) flag.
    #[inline]
    #[must_use]
    pub const fn flag_et(&self) -> Cr0Et<'_> {
        let &Self(ref target_value) = self;

        Cr0Et::wrap(target_value)
    }

    /// Determine the "Numeric Error" (NE) flag.
    #[inline]
    #[must_use]
    pub const fn flag_ne(&self) -> Cr0Ne<'_> {
        let &Self(ref target_value) = self;

        Cr0Ne::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Numeric Error" (NE) flag.
    #[inline]
    pub const fn flag_ne_mut(&mut self) -> Cr0NeMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr0NeMut::wrap(target_value)
    }

    /// Determine reserved CR0 bits 6 through 15.
    #[inline]
    #[must_use]
    pub const fn reserved_6_15(&self) -> Cr0Reserved6_15<'_> {
        let &Self(ref target_value) = self;

        Cr0Reserved6_15::wrap(target_value)
    }

    /// Determine the "Write Protect" (WP) flag.
    #[inline]
    #[must_use]
    pub const fn flag_wp(&self) -> Cr0Wp<'_> {
        let &Self(ref target_value) = self;

        Cr0Wp::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Write Protect" (WP) flag.
    #[inline]
    pub const fn flag_wp_mut(&mut self) -> Cr0WpMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr0WpMut::wrap(target_value)
    }

    /// Determine architecturally reserved CR0 bit 17.
    #[inline]
    #[must_use]
    pub const fn flag_reserved17(&self) -> Cr0Reserved17<'_> {
        let &Self(ref target_value) = self;

        Cr0Reserved17::wrap(target_value)
    }

    /// Determine the "Alignment Mask" (AM) flag.
    #[inline]
    #[must_use]
    pub const fn flag_am(&self) -> Cr0Am<'_> {
        let &Self(ref target_value) = self;

        Cr0Am::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Alignment Mask" (AM) flag.
    #[inline]
    pub const fn flag_am_mut(&mut self) -> Cr0AmMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr0AmMut::wrap(target_value)
    }

    /// Determine reserved CR0 bits 19 through 28.
    #[inline]
    #[must_use]
    pub const fn reserved_19_28(&self) -> Cr0Reserved19_28<'_> {
        let &Self(ref target_value) = self;

        Cr0Reserved19_28::wrap(target_value)
    }

    /// Determine the "Not Write-through" (NW) flag.
    #[inline]
    #[must_use]
    pub const fn flag_nw(&self) -> Cr0Nw<'_> {
        let &Self(ref target_value) = self;

        Cr0Nw::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Not Write-through" (NW) flag.
    #[inline]
    pub const fn flag_nw_mut(&mut self) -> Cr0NwMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr0NwMut::wrap(target_value)
    }

    /// Determine the "Cache Disable" (CD) flag.
    #[inline]
    #[must_use]
    pub const fn flag_cd(&self) -> Cr0Cd<'_> {
        let &Self(ref target_value) = self;

        Cr0Cd::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Cache Disable" (CD) flag.
    #[inline]
    pub const fn flag_cd_mut(&mut self) -> Cr0CdMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr0CdMut::wrap(target_value)
    }

    /// Determine the "Paging" (PG) flag.
    #[inline]
    #[must_use]
    pub const fn flag_pg(&self) -> Cr0Pg<'_> {
        let &Self(ref target_value) = self;

        Cr0Pg::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Paging" (PG) flag.
    #[inline]
    pub const fn flag_pg_mut(&mut self) -> Cr0PgMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr0PgMut::wrap(target_value)
    }

    /// Determine reserved CR0 bits 32 through 63.
    #[inline]
    #[must_use]
    pub const fn reserved_32_63(&self) -> Cr0Reserved32_63<'_> {
        let &Self(ref target_value) = self;

        Cr0Reserved32_63::wrap(target_value)
    }
}

/// The process-context identifier field in [`Cr3`] when PCIDE is enabled.
pub type Cr3Pcid<'value> = Field<'value, 0, 11, u64>;

/// Reserved CR3 bits zero through two when PCIDE is disabled.
pub type Cr3Reserved0_2<'value> = Field<'value, 0, 2, u64>;

/// The page-level write-through flag in [`Cr3`] when PCIDE is disabled.
pub type Cr3Pwt<'value> = Bit<'value, u64, 3>;

/// The page-level cache-disable flag in [`Cr3`] when PCIDE is disabled.
pub type Cr3Pcd<'value> = Bit<'value, u64, 4>;

/// Reserved CR3 bits five through eleven when PCIDE is disabled.
pub type Cr3Reserved5_11<'value> = Field<'value, 5, 11, u64>;

/// The physical page-number field selecting the active paging root.
pub type Cr3Root<'value> = Field<'value, 12, 51, u64>;

/// Reserved CR3 bits 52 through 60.
pub type Cr3Reserved52_60<'value> = Field<'value, 52, 60, u64>;

/// The user LAM57 enable flag.
pub type Cr3LamU57<'value> = Bit<'value, u64, 61>;

/// The user LAM48 enable flag.
pub type Cr3LamU48<'value> = Bit<'value, u64, 62>;

/// The PCID no-flush hint in bit 63.
pub type Cr3NoFlush<'value> = Bit<'value, u64, 63>;

/// Mutable counterpart to [`Cr3Pcid`].
pub type Cr3PcidMut<'value> = <Cr3Pcid<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`Cr3Pwt`].
pub type Cr3PwtMut<'value> = <Cr3Pwt<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`Cr3Pcd`].
pub type Cr3PcdMut<'value> = <Cr3Pcd<'value> as Counterpart>::Mut;

/// Internal mutable counterpart used to construct the CR3 root field.
type Cr3RootMut<'value> = <Cr3Root<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`Cr3LamU57`].
pub type Cr3LamU57Mut<'value> = <Cr3LamU57<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`Cr3LamU48`].
pub type Cr3LamU48Mut<'value> = <Cr3LamU48<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`Cr3NoFlush`].
pub type Cr3NoFlushMut<'value> = <Cr3NoFlush<'value> as Counterpart>::Mut;

/// The 64-bit CR3 paging-root register.
///
/// [`Cr3::root_table`] constructs the common bootstrap form with low
/// mode-dependent fields cleared. When CR4.PCIDE is clear, bits three and four
/// are exposed as PWT and PCD. A future PCID API can model the overlapping low
/// field without changing this representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): Safe root construction accepts only 4 KiB aligned architectural physical
// addresses, while trusted raw reconstruction carries an explicit unsafe proof.
pub struct Cr3(u64);

/// Raw transport representation of [`Cr3`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct RawCr3(pub u64);

impl RawCr3 {
    /// Erases typed CR3 state into its complete transport representation.
    #[inline]
    #[must_use]
    pub const fn take(value: Cr3) -> Self {
        let Cr3(value) = value;
        Self(value)
    }

    /// Reconstructs typed CR3 state from a trusted architectural image.
    ///
    /// # Safety
    ///
    /// The stored bits must come from a trusted processor or virtualization
    /// boundary, or otherwise satisfy the architectural CR3 requirements for
    /// the state that will consume them.
    #[inline]
    #[must_use]
    pub const unsafe fn restore(self) -> Cr3 {
        let Self(value) = self;

        // SAFETY: The caller supplies the architectural validity proof and the
        // transparent representation guarantees identical size and alignment.
        unsafe { mem::transmute_copy(&value) }
    }
}

impl Cr3 {
    /// Create a CR3 root-table image from a 4 KiB-aligned physical address.
    ///
    /// Returns `None` when `address` has a nonzero 4 KiB page offset.
    #[inline]
    #[must_use]
    pub const fn root_table(address: Pa) -> Option<Self> {
        match address.offset_4kib().const_value() {
            0 => {
                let mut value = u64::MIN;
                let mut root = Cr3RootMut::wrap(&mut value);

                root.const_merge(address.frame_4kib().const_value());

                Some(Self(value))
            },
            _ => None,
        }
    }

    /// Determine the PCID field.
    #[inline]
    #[must_use]
    pub const fn pcid(&self) -> Cr3Pcid<'_> {
        let &Self(ref value) = self;

        Cr3Pcid::wrap(value)
    }

    /// Mutably access the PCID field.
    #[inline]
    pub const fn pcid_mut(&mut self) -> Cr3PcidMut<'_> {
        let &mut Self(ref mut value) = self;

        Cr3PcidMut::wrap(value)
    }

    /// Determine reserved bits zero through two for the non-PCID interpretation.
    #[inline]
    #[must_use]
    pub const fn reserved_0_2(&self) -> Cr3Reserved0_2<'_> {
        let &Self(ref value) = self;

        Cr3Reserved0_2::wrap(value)
    }

    /// Determine page-level write-through when PCID is disabled.
    #[inline]
    #[must_use]
    pub const fn flag_write_through(&self) -> Cr3Pwt<'_> {
        let &Self(ref value) = self;

        Cr3Pwt::wrap(value)
    }

    /// Mutably access page-level write-through when PCID is disabled.
    #[inline]
    pub const fn flag_write_through_mut(&mut self) -> Cr3PwtMut<'_> {
        let &mut Self(ref mut value) = self;

        Cr3PwtMut::wrap(value)
    }

    /// Determine page-level cache-disable when PCID is disabled.
    #[inline]
    #[must_use]
    pub const fn flag_cache_disable(&self) -> Cr3Pcd<'_> {
        let &Self(ref value) = self;

        Cr3Pcd::wrap(value)
    }

    /// Mutably access page-level cache-disable when PCID is disabled.
    #[inline]
    pub const fn flag_cache_disable_mut(&mut self) -> Cr3PcdMut<'_> {
        let &mut Self(ref mut value) = self;

        Cr3PcdMut::wrap(value)
    }

    /// Determine reserved bits five through eleven for the non-PCID interpretation.
    #[inline]
    #[must_use]
    pub const fn reserved_5_11(&self) -> Cr3Reserved5_11<'_> {
        let &Self(ref value) = self;

        Cr3Reserved5_11::wrap(value)
    }

    /// Determine the paging-root physical page-number field.
    #[inline]
    #[must_use]
    pub const fn root(&self) -> Cr3Root<'_> {
        let &Self(ref value) = self;

        Cr3Root::wrap(value)
    }

    /// Determine reserved bits 52 through 60.
    #[inline]
    #[must_use]
    pub const fn reserved_52_60(&self) -> Cr3Reserved52_60<'_> {
        let &Self(ref value) = self;

        Cr3Reserved52_60::wrap(value)
    }

    /// Determine the user LAM57 enable flag.
    #[inline]
    #[must_use]
    pub const fn flag_lam_u57(&self) -> Cr3LamU57<'_> {
        let &Self(ref value) = self;

        Cr3LamU57::wrap(value)
    }

    /// Mutably access the user LAM57 enable flag.
    #[inline]
    pub const fn flag_lam_u57_mut(&mut self) -> Cr3LamU57Mut<'_> {
        let &mut Self(ref mut value) = self;

        Cr3LamU57Mut::wrap(value)
    }

    /// Determine the user LAM48 enable flag.
    #[inline]
    #[must_use]
    pub const fn flag_lam_u48(&self) -> Cr3LamU48<'_> {
        let &Self(ref value) = self;

        Cr3LamU48::wrap(value)
    }

    /// Mutably access the user LAM48 enable flag.
    #[inline]
    pub const fn flag_lam_u48_mut(&mut self) -> Cr3LamU48Mut<'_> {
        let &mut Self(ref mut value) = self;

        Cr3LamU48Mut::wrap(value)
    }

    /// Determine the PCID no-flush hint.
    #[inline]
    #[must_use]
    pub const fn flag_no_flush(&self) -> Cr3NoFlush<'_> {
        let &Self(ref value) = self;

        Cr3NoFlush::wrap(value)
    }

    /// Mutably access the PCID no-flush hint.
    #[inline]
    pub const fn flag_no_flush_mut(&mut self) -> Cr3NoFlushMut<'_> {
        let &mut Self(ref mut value) = self;

        Cr3NoFlushMut::wrap(value)
    }
}

/// The page-fault linear address held in [`Cr2`].
pub type Cr2LinearAddress<'value> = Field<'value, 0, 63, u64>;

/// The 64-bit CR2 page-fault linear-address register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar preserves the complete architectural CR2 image.
pub struct Cr2(u64);

/// Raw transport representation of [`Cr2`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct RawCr2(pub u64);

impl RawCr2 {
    /// Erase typed CR2 state into its complete transport representation.
    #[inline]
    #[must_use]
    pub const fn take(value: Cr2) -> Self {
        let Cr2(value) = value;

        Self(value)
    }

    /// Restore typed CR2 state from a complete architectural image.
    #[inline]
    #[must_use]
    pub const fn restore(self) -> Cr2 {
        let Self(value) = self;

        Cr2(value)
    }
}

impl Cr2 {
    /// Determine the complete page-fault linear address.
    #[inline]
    #[must_use]
    pub const fn linear_address(&self) -> Cr2LinearAddress<'_> {
        let &Self(ref value) = self;

        Cr2LinearAddress::wrap(value)
    }
}

/// The task-priority class held in [`Cr8`].
pub type Cr8TaskPriority<'value> = Field<'value, 0, 3, u64>;

/// Mutable counterpart to [`Cr8TaskPriority`].
pub type Cr8TaskPriorityMut<'value> = <Cr8TaskPriority<'value> as Counterpart>::Mut;

/// Reserved CR8 bits four through 63.
pub type Cr8Reserved4_63<'value> = Field<'value, 4, 63, u64>;

/// The 64-bit CR8 task-priority register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): Safe mutation is limited to the four-bit task-priority class while reserved
// bits remain read-only through the typed API.
pub struct Cr8(u64);

/// Raw transport representation of [`Cr8`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct RawCr8(pub u64);

impl RawCr8 {
    /// Erase typed CR8 state into its complete transport representation.
    #[inline]
    #[must_use]
    pub const fn take(value: Cr8) -> Self {
        let Cr8(value) = value;

        Self(value)
    }

    /// Reconstruct typed CR8 state from a trusted architectural image.
    ///
    /// # Safety
    ///
    /// Reserved bits must satisfy the architectural CR8 requirements.
    #[inline]
    #[must_use]
    pub const unsafe fn restore(self) -> Cr8 {
        let Self(value) = self;

        Cr8(value)
    }
}

impl Cr8 {
    /// Architectural reset value.
    pub const RESET: Self = Self(0);

    /// Determine the task-priority class.
    #[inline]
    #[must_use]
    pub const fn task_priority(&self) -> Cr8TaskPriority<'_> {
        let &Self(ref value) = self;

        Cr8TaskPriority::wrap(value)
    }

    /// Mutably access the task-priority class.
    #[inline]
    pub const fn task_priority_mut(&mut self) -> Cr8TaskPriorityMut<'_> {
        let &mut Self(ref mut value) = self;

        Cr8TaskPriorityMut::wrap(value)
    }

    /// Determine reserved bits four through 63.
    #[inline]
    #[must_use]
    pub const fn reserved_4_63(&self) -> Cr8Reserved4_63<'_> {
        let &Self(ref value) = self;

        Cr8Reserved4_63::wrap(value)
    }
}

/// Coupled x86-64 control state proven sufficient to activate long mode.
///
/// The proof covers the architectural control relationship that must hold as a
/// unit during saved-state or virtualization restore. It requires CR0.PE and
/// CR0.PG, CR4.PAE, a page-aligned typed CR3 root, and an active long-mode EFER
/// restore proof. Other control bits remain caller-selected and are preserved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
// NOTE(invariant): Construction succeeds only when CR0 has PE and PG set, CR4 has PAE set, CR3
// already carries its typed root invariant, and EFER is proven active through `LongModeEfer`.
pub struct LongModeControl {
    /// CR0 image with protection and paging enabled.
    cr0: Cr0,

    /// Typed page-table root.
    cr3: Cr3,

    /// CR4 image with physical-address extension enabled.
    cr4: Cr4,

    /// EFER image proven to contain LME and processor-maintained LMA.
    efer: LongModeEfer,
}

impl LongModeControl {
    /// Create a long-mode control proof from a coupled CR0, CR3, and CR4 image.
    ///
    /// Returns `None` unless CR0 enables protection and paging and CR4 enables
    /// physical-address extension. The supplied EFER must already carry the
    /// active long-mode proof.
    #[inline]
    #[must_use]
    pub const fn new(cr0: Cr0, cr3: Cr3, cr4: Cr4, efer: LongModeEfer) -> Option<Self> {
        let protection_enabled = matches!(cr0.flag_pe().const_state(), State::Set);
        let paging_enabled = matches!(cr0.flag_pg().const_state(), State::Set);
        let pae_enabled = matches!(cr4.flag_pae().const_state(), State::Set);

        match (protection_enabled, paging_enabled, pae_enabled) {
            (true, true, true) => Some(Self { cr0, cr3, cr4, efer }),
            _ => None,
        }
    }

    /// Returns the proven CR0 image.
    #[inline]
    #[must_use]
    pub const fn cr0(self) -> Cr0 {
        let Self { cr0, .. } = self;

        cr0
    }

    /// Returns the typed page-table root.
    #[inline]
    #[must_use]
    pub const fn cr3(self) -> Cr3 {
        let Self { cr3, .. } = self;

        cr3
    }

    /// Returns the proven CR4 image.
    #[inline]
    #[must_use]
    pub const fn cr4(self) -> Cr4 {
        let Self { cr4, .. } = self;

        cr4
    }

    /// Returns the active long-mode EFER proof.
    #[inline]
    #[must_use]
    pub const fn efer(self) -> LongModeEfer {
        let Self { efer, .. } = self;

        efer
    }
}

/// The Virtual-8086 mode extensions flag in [`Cr4`].
pub type Cr4Vme<'value> = Bit<'value, u64, 0>;

/// The Protected-mode virtual interrupts flag in [`Cr4`].
pub type Cr4Pvi<'value> = Bit<'value, u64, 1>;

/// The Time stamp disable flag in [`Cr4`].
pub type Cr4Tsd<'value> = Bit<'value, u64, 2>;

/// The Debugging extensions flag in [`Cr4`].
pub type Cr4De<'value> = Bit<'value, u64, 3>;

/// The Page size extensions flag in [`Cr4`].
pub type Cr4Pse<'value> = Bit<'value, u64, 4>;

/// The Physical address extension flag in [`Cr4`].
pub type Cr4Pae<'value> = Bit<'value, u64, 5>;

/// The Machine-check enable flag in [`Cr4`].
pub type Cr4Mce<'value> = Bit<'value, u64, 6>;

/// The Page global enable flag in [`Cr4`].
pub type Cr4Pge<'value> = Bit<'value, u64, 7>;

/// The Performance-monitoring counter enable flag in [`Cr4`].
pub type Cr4Pce<'value> = Bit<'value, u64, 8>;

/// The OS support for FXSAVE and FXRSTOR flag in [`Cr4`].
pub type Cr4Osfxsr<'value> = Bit<'value, u64, 9>;

/// The OS support for unmasked SIMD floating-point exceptions flag in [`Cr4`].
pub type Cr4Osxmmexcpt<'value> = Bit<'value, u64, 10>;

/// The User-mode instruction prevention flag in [`Cr4`].
pub type Cr4Umip<'value> = Bit<'value, u64, 11>;

/// The 57-bit linear addresses flag in [`Cr4`].
pub type Cr4La57<'value> = Bit<'value, u64, 12>;

/// The VMX enable flag in [`Cr4`].
pub type Cr4Vmxe<'value> = Bit<'value, u64, 13>;

/// The SMX enable flag in [`Cr4`].
pub type Cr4Smxe<'value> = Bit<'value, u64, 14>;

/// The FSGSBASE enable flag in [`Cr4`].
pub type Cr4Fsgsbase<'value> = Bit<'value, u64, 16>;

/// The PCID enable flag in [`Cr4`].
pub type Cr4Pcide<'value> = Bit<'value, u64, 17>;

/// The OS XSAVE enable flag in [`Cr4`].
pub type Cr4Osxsave<'value> = Bit<'value, u64, 18>;

/// The Key Locker enable flag in [`Cr4`].
pub type Cr4Kl<'value> = Bit<'value, u64, 19>;

/// The Supervisor-mode execution prevention flag in [`Cr4`].
pub type Cr4Smep<'value> = Bit<'value, u64, 20>;

/// The Supervisor-mode access prevention flag in [`Cr4`].
pub type Cr4Smap<'value> = Bit<'value, u64, 21>;

/// The Protection keys for user pages flag in [`Cr4`].
pub type Cr4Pke<'value> = Bit<'value, u64, 22>;

/// The Control-flow enforcement technology flag in [`Cr4`].
pub type Cr4Cet<'value> = Bit<'value, u64, 23>;

/// The Protection keys for supervisor pages flag in [`Cr4`].
pub type Cr4Pks<'value> = Bit<'value, u64, 24>;

/// The User interrupts enable flag in [`Cr4`].
pub type Cr4Uintr<'value> = Bit<'value, u64, 25>;

/// The Linear-address-space separation flag in [`Cr4`].
pub type Cr4Lass<'value> = Bit<'value, u64, 27>;

/// The Supervisor linear-address masking flag in [`Cr4`].
pub type Cr4LamSup<'value> = Bit<'value, u64, 28>;

/// The Flexible return and event delivery flag in [`Cr4`].
pub type Cr4Fred<'value> = Bit<'value, u64, 32>;

/// Reserved CR4 bit 15.
pub type Cr4Reserved15<'value> = Bit<'value, u64, 15>;

/// Reserved CR4 bit 26.
pub type Cr4Reserved26<'value> = Bit<'value, u64, 26>;

/// Reserved CR4 bits 29 through 31.
pub type Cr4Reserved29_31<'value> = Field<'value, 29, 31, u64>;

/// Reserved CR4 bits 33 through 63.
pub type Cr4Reserved33_63<'value> = Field<'value, 33, 63, u64>;

/// Mutable counterpart to [`Cr4Vme`].
pub type Cr4VmeMut<'value> = <Cr4Vme<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`Cr4Pvi`].
pub type Cr4PviMut<'value> = <Cr4Pvi<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`Cr4Tsd`].
pub type Cr4TsdMut<'value> = <Cr4Tsd<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`Cr4De`].
pub type Cr4DeMut<'value> = <Cr4De<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`Cr4Pse`].
pub type Cr4PseMut<'value> = <Cr4Pse<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`Cr4Pae`].
pub type Cr4PaeMut<'value> = <Cr4Pae<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`Cr4Mce`].
pub type Cr4MceMut<'value> = <Cr4Mce<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`Cr4Pge`].
pub type Cr4PgeMut<'value> = <Cr4Pge<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`Cr4Pce`].
pub type Cr4PceMut<'value> = <Cr4Pce<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`Cr4Osfxsr`].
pub type Cr4OsfxsrMut<'value> = <Cr4Osfxsr<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`Cr4Osxmmexcpt`].
pub type Cr4OsxmmexcptMut<'value> = <Cr4Osxmmexcpt<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`Cr4Umip`].
pub type Cr4UmipMut<'value> = <Cr4Umip<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`Cr4La57`].
pub type Cr4La57Mut<'value> = <Cr4La57<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`Cr4Vmxe`].
pub type Cr4VmxeMut<'value> = <Cr4Vmxe<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`Cr4Smxe`].
pub type Cr4SmxeMut<'value> = <Cr4Smxe<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`Cr4Fsgsbase`].
pub type Cr4FsgsbaseMut<'value> = <Cr4Fsgsbase<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`Cr4Pcide`].
pub type Cr4PcideMut<'value> = <Cr4Pcide<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`Cr4Osxsave`].
pub type Cr4OsxsaveMut<'value> = <Cr4Osxsave<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`Cr4Kl`].
pub type Cr4KlMut<'value> = <Cr4Kl<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`Cr4Smep`].
pub type Cr4SmepMut<'value> = <Cr4Smep<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`Cr4Smap`].
pub type Cr4SmapMut<'value> = <Cr4Smap<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`Cr4Pke`].
pub type Cr4PkeMut<'value> = <Cr4Pke<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`Cr4Cet`].
pub type Cr4CetMut<'value> = <Cr4Cet<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`Cr4Pks`].
pub type Cr4PksMut<'value> = <Cr4Pks<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`Cr4Uintr`].
pub type Cr4UintrMut<'value> = <Cr4Uintr<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`Cr4Lass`].
pub type Cr4LassMut<'value> = <Cr4Lass<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`Cr4LamSup`].
pub type Cr4LamSupMut<'value> = <Cr4LamSup<'value> as Counterpart>::Mut;

/// Mutable counterpart to [`Cr4Fred`].
pub type Cr4FredMut<'value> = <Cr4Fred<'value> as Counterpart>::Mut;

/// The 64-bit CR4 control register.
///
/// Only fields currently modeled by Nekor are exposed. The storage remains an
/// exact `u64`, allowing additional architectural fields to be added without
/// changing the register representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): Safe construction mutates only modeled CR4 fields, while trusted raw
// reconstruction carries an explicit unsafe proof for the complete image.
pub struct Cr4(u64);
/// Raw transport representation of [`Cr4`].
///
/// This type preserves the complete control-register image for hardware and
/// virtualization boundaries without adding a general raw constructor to
/// [`Cr4`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct RawCr4(pub u64);

impl RawCr4 {
    /// Erases typed CR4 state into its complete transport representation.
    #[inline]
    #[must_use]
    pub const fn take(value: Cr4) -> Self {
        let Cr4(value) = value;
        Self(value)
    }

    /// Reconstructs typed CR4 state from a trusted architectural image.
    ///
    /// # Safety
    ///
    /// The stored bits must come from a trusted processor or virtualization
    /// boundary, or otherwise satisfy the architectural CR4 requirements for
    /// the state that will consume them.
    #[inline]
    #[must_use]
    pub const unsafe fn restore(self) -> Cr4 {
        let Self(value) = self;

        // SAFETY: The caller supplies the architectural validity proof and the
        // transparent representation guarantees identical size and alignment.
        unsafe { mem::transmute_copy(&value) }
    }
}

impl Cr4 {
    /// Architectural reset value.
    pub const RESET: Self = Self(u64::MIN);

    /// Determine the vme flag.
    #[inline]
    #[must_use]
    pub const fn flag_vme(&self) -> Cr4Vme<'_> {
        let &Self(ref target_value) = self;

        Cr4Vme::wrap(target_value)
    }

    /// Mutably access the vme flag.
    #[inline]
    pub const fn flag_vme_mut(&mut self) -> Cr4VmeMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr4VmeMut::wrap(target_value)
    }

    /// Determine the pvi flag.
    #[inline]
    #[must_use]
    pub const fn flag_pvi(&self) -> Cr4Pvi<'_> {
        let &Self(ref target_value) = self;

        Cr4Pvi::wrap(target_value)
    }

    /// Mutably access the pvi flag.
    #[inline]
    pub const fn flag_pvi_mut(&mut self) -> Cr4PviMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr4PviMut::wrap(target_value)
    }

    /// Determine the tsd flag.
    #[inline]
    #[must_use]
    pub const fn flag_tsd(&self) -> Cr4Tsd<'_> {
        let &Self(ref target_value) = self;

        Cr4Tsd::wrap(target_value)
    }

    /// Mutably access the tsd flag.
    #[inline]
    pub const fn flag_tsd_mut(&mut self) -> Cr4TsdMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr4TsdMut::wrap(target_value)
    }

    /// Determine the de flag.
    #[inline]
    #[must_use]
    pub const fn flag_de(&self) -> Cr4De<'_> {
        let &Self(ref target_value) = self;

        Cr4De::wrap(target_value)
    }

    /// Mutably access the de flag.
    #[inline]
    pub const fn flag_de_mut(&mut self) -> Cr4DeMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr4DeMut::wrap(target_value)
    }

    /// Determine the pse flag.
    #[inline]
    #[must_use]
    pub const fn flag_pse(&self) -> Cr4Pse<'_> {
        let &Self(ref target_value) = self;

        Cr4Pse::wrap(target_value)
    }

    /// Mutably access the pse flag.
    #[inline]
    pub const fn flag_pse_mut(&mut self) -> Cr4PseMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr4PseMut::wrap(target_value)
    }

    /// Determine the pae flag.
    #[inline]
    #[must_use]
    pub const fn flag_pae(&self) -> Cr4Pae<'_> {
        let &Self(ref target_value) = self;

        Cr4Pae::wrap(target_value)
    }

    /// Mutably access the pae flag.
    #[inline]
    pub const fn flag_pae_mut(&mut self) -> Cr4PaeMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr4PaeMut::wrap(target_value)
    }

    /// Determine the mce flag.
    #[inline]
    #[must_use]
    pub const fn flag_mce(&self) -> Cr4Mce<'_> {
        let &Self(ref target_value) = self;

        Cr4Mce::wrap(target_value)
    }

    /// Mutably access the mce flag.
    #[inline]
    pub const fn flag_mce_mut(&mut self) -> Cr4MceMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr4MceMut::wrap(target_value)
    }

    /// Determine the pge flag.
    #[inline]
    #[must_use]
    pub const fn flag_pge(&self) -> Cr4Pge<'_> {
        let &Self(ref target_value) = self;

        Cr4Pge::wrap(target_value)
    }

    /// Mutably access the pge flag.
    #[inline]
    pub const fn flag_pge_mut(&mut self) -> Cr4PgeMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr4PgeMut::wrap(target_value)
    }

    /// Determine the pce flag.
    #[inline]
    #[must_use]
    pub const fn flag_pce(&self) -> Cr4Pce<'_> {
        let &Self(ref target_value) = self;

        Cr4Pce::wrap(target_value)
    }

    /// Mutably access the pce flag.
    #[inline]
    pub const fn flag_pce_mut(&mut self) -> Cr4PceMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr4PceMut::wrap(target_value)
    }

    /// Determine the osfxsr flag.
    #[inline]
    #[must_use]
    pub const fn flag_osfxsr(&self) -> Cr4Osfxsr<'_> {
        let &Self(ref target_value) = self;

        Cr4Osfxsr::wrap(target_value)
    }

    /// Mutably access the osfxsr flag.
    #[inline]
    pub const fn flag_osfxsr_mut(&mut self) -> Cr4OsfxsrMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr4OsfxsrMut::wrap(target_value)
    }

    /// Determine the osxmmexcpt flag.
    #[inline]
    #[must_use]
    pub const fn flag_osxmmexcpt(&self) -> Cr4Osxmmexcpt<'_> {
        let &Self(ref target_value) = self;

        Cr4Osxmmexcpt::wrap(target_value)
    }

    /// Mutably access the osxmmexcpt flag.
    #[inline]
    pub const fn flag_osxmmexcpt_mut(&mut self) -> Cr4OsxmmexcptMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr4OsxmmexcptMut::wrap(target_value)
    }

    /// Determine the umip flag.
    #[inline]
    #[must_use]
    pub const fn flag_umip(&self) -> Cr4Umip<'_> {
        let &Self(ref target_value) = self;

        Cr4Umip::wrap(target_value)
    }

    /// Mutably access the umip flag.
    #[inline]
    pub const fn flag_umip_mut(&mut self) -> Cr4UmipMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr4UmipMut::wrap(target_value)
    }

    /// Determine the la57 flag.
    #[inline]
    #[must_use]
    pub const fn flag_la57(&self) -> Cr4La57<'_> {
        let &Self(ref target_value) = self;

        Cr4La57::wrap(target_value)
    }

    /// Mutably access the la57 flag.
    #[inline]
    pub const fn flag_la57_mut(&mut self) -> Cr4La57Mut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr4La57Mut::wrap(target_value)
    }

    /// Determine the vmxe flag.
    #[inline]
    #[must_use]
    pub const fn flag_vmxe(&self) -> Cr4Vmxe<'_> {
        let &Self(ref target_value) = self;

        Cr4Vmxe::wrap(target_value)
    }

    /// Mutably access the vmxe flag.
    #[inline]
    pub const fn flag_vmxe_mut(&mut self) -> Cr4VmxeMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr4VmxeMut::wrap(target_value)
    }

    /// Determine the smxe flag.
    #[inline]
    #[must_use]
    pub const fn flag_smxe(&self) -> Cr4Smxe<'_> {
        let &Self(ref target_value) = self;

        Cr4Smxe::wrap(target_value)
    }

    /// Mutably access the smxe flag.
    #[inline]
    pub const fn flag_smxe_mut(&mut self) -> Cr4SmxeMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr4SmxeMut::wrap(target_value)
    }

    /// Determine the fsgsbase flag.
    #[inline]
    #[must_use]
    pub const fn flag_fsgsbase(&self) -> Cr4Fsgsbase<'_> {
        let &Self(ref target_value) = self;

        Cr4Fsgsbase::wrap(target_value)
    }

    /// Mutably access the fsgsbase flag.
    #[inline]
    pub const fn flag_fsgsbase_mut(&mut self) -> Cr4FsgsbaseMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr4FsgsbaseMut::wrap(target_value)
    }

    /// Determine the pcide flag.
    #[inline]
    #[must_use]
    pub const fn flag_pcide(&self) -> Cr4Pcide<'_> {
        let &Self(ref target_value) = self;

        Cr4Pcide::wrap(target_value)
    }

    /// Mutably access the pcide flag.
    #[inline]
    pub const fn flag_pcide_mut(&mut self) -> Cr4PcideMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr4PcideMut::wrap(target_value)
    }

    /// Determine the osxsave flag.
    #[inline]
    #[must_use]
    pub const fn flag_osxsave(&self) -> Cr4Osxsave<'_> {
        let &Self(ref target_value) = self;

        Cr4Osxsave::wrap(target_value)
    }

    /// Mutably access the osxsave flag.
    #[inline]
    pub const fn flag_osxsave_mut(&mut self) -> Cr4OsxsaveMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr4OsxsaveMut::wrap(target_value)
    }

    /// Determine the kl flag.
    #[inline]
    #[must_use]
    pub const fn flag_kl(&self) -> Cr4Kl<'_> {
        let &Self(ref target_value) = self;

        Cr4Kl::wrap(target_value)
    }

    /// Mutably access the kl flag.
    #[inline]
    pub const fn flag_kl_mut(&mut self) -> Cr4KlMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr4KlMut::wrap(target_value)
    }

    /// Determine the smep flag.
    #[inline]
    #[must_use]
    pub const fn flag_smep(&self) -> Cr4Smep<'_> {
        let &Self(ref target_value) = self;

        Cr4Smep::wrap(target_value)
    }

    /// Mutably access the smep flag.
    #[inline]
    pub const fn flag_smep_mut(&mut self) -> Cr4SmepMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr4SmepMut::wrap(target_value)
    }

    /// Determine the smap flag.
    #[inline]
    #[must_use]
    pub const fn flag_smap(&self) -> Cr4Smap<'_> {
        let &Self(ref target_value) = self;

        Cr4Smap::wrap(target_value)
    }

    /// Mutably access the smap flag.
    #[inline]
    pub const fn flag_smap_mut(&mut self) -> Cr4SmapMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr4SmapMut::wrap(target_value)
    }

    /// Determine the pke flag.
    #[inline]
    #[must_use]
    pub const fn flag_pke(&self) -> Cr4Pke<'_> {
        let &Self(ref target_value) = self;

        Cr4Pke::wrap(target_value)
    }

    /// Mutably access the pke flag.
    #[inline]
    pub const fn flag_pke_mut(&mut self) -> Cr4PkeMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr4PkeMut::wrap(target_value)
    }

    /// Determine the cet flag.
    #[inline]
    #[must_use]
    pub const fn flag_cet(&self) -> Cr4Cet<'_> {
        let &Self(ref target_value) = self;

        Cr4Cet::wrap(target_value)
    }

    /// Mutably access the cet flag.
    #[inline]
    pub const fn flag_cet_mut(&mut self) -> Cr4CetMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr4CetMut::wrap(target_value)
    }

    /// Determine the pks flag.
    #[inline]
    #[must_use]
    pub const fn flag_pks(&self) -> Cr4Pks<'_> {
        let &Self(ref target_value) = self;

        Cr4Pks::wrap(target_value)
    }

    /// Mutably access the pks flag.
    #[inline]
    pub const fn flag_pks_mut(&mut self) -> Cr4PksMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr4PksMut::wrap(target_value)
    }

    /// Determine the uintr flag.
    #[inline]
    #[must_use]
    pub const fn flag_uintr(&self) -> Cr4Uintr<'_> {
        let &Self(ref target_value) = self;

        Cr4Uintr::wrap(target_value)
    }

    /// Mutably access the uintr flag.
    #[inline]
    pub const fn flag_uintr_mut(&mut self) -> Cr4UintrMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr4UintrMut::wrap(target_value)
    }

    /// Determine the lass flag.
    #[inline]
    #[must_use]
    pub const fn flag_lass(&self) -> Cr4Lass<'_> {
        let &Self(ref target_value) = self;

        Cr4Lass::wrap(target_value)
    }

    /// Mutably access the lass flag.
    #[inline]
    pub const fn flag_lass_mut(&mut self) -> Cr4LassMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr4LassMut::wrap(target_value)
    }

    /// Determine the lam sup flag.
    #[inline]
    #[must_use]
    pub const fn flag_lam_sup(&self) -> Cr4LamSup<'_> {
        let &Self(ref target_value) = self;

        Cr4LamSup::wrap(target_value)
    }

    /// Mutably access the lam sup flag.
    #[inline]
    pub const fn flag_lam_sup_mut(&mut self) -> Cr4LamSupMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr4LamSupMut::wrap(target_value)
    }

    /// Determine the fred flag.
    #[inline]
    #[must_use]
    pub const fn flag_fred(&self) -> Cr4Fred<'_> {
        let &Self(ref target_value) = self;

        Cr4Fred::wrap(target_value)
    }

    /// Mutably access the fred flag.
    #[inline]
    pub const fn flag_fred_mut(&mut self) -> Cr4FredMut<'_> {
        let &mut Self(ref mut target_value) = self;

        Cr4FredMut::wrap(target_value)
    }

    /// Determine reserved bit 15.
    #[inline]
    #[must_use]
    pub const fn reserved_15(&self) -> Cr4Reserved15<'_> {
        let &Self(ref target_value) = self;

        Cr4Reserved15::wrap(target_value)
    }

    /// Determine reserved bit 26.
    #[inline]
    #[must_use]
    pub const fn reserved_26(&self) -> Cr4Reserved26<'_> {
        let &Self(ref target_value) = self;

        Cr4Reserved26::wrap(target_value)
    }

    /// Determine reserved bits 29 through 31.
    #[inline]
    #[must_use]
    pub const fn reserved_29_31(&self) -> Cr4Reserved29_31<'_> {
        let &Self(ref target_value) = self;

        Cr4Reserved29_31::wrap(target_value)
    }

    /// Determine reserved bits 33 through 63.
    #[inline]
    #[must_use]
    pub const fn reserved_33_63(&self) -> Cr4Reserved33_63<'_> {
        let &Self(ref target_value) = self;

        Cr4Reserved33_63::wrap(target_value)
    }
}

#[cfg(test)]
mod tests {
    use nekor_bitwise::prelude::State;

    use super::{Cr0, Cr3, Cr4, LongModeControl, RawCr0, RawCr3, RawCr4};
    use crate::{
        x86::msr::efer::{Efer, LongModeEfer},
        x86_64::paging::Pa,
    };

    #[test]
    fn control_register_transport_round_trips_trusted_images() {
        let raw_cr0 = RawCr0::take(Cr0::RESET);

        let RawCr0(cr0_bits) = raw_cr0;

        let raw_cr4 = RawCr4::take(Cr4::RESET);

        let RawCr4(cr4_bits) = raw_cr4;

        // SAFETY: Both images were erased from typed reset values immediately above.
        let cr0 = unsafe { RawCr0(cr0_bits).restore() };
        // SAFETY: Both images were erased from typed reset values immediately above.
        let cr4 = unsafe { RawCr4(cr4_bits).restore() };

        assert_eq!(cr0, Cr0::RESET);
        assert_eq!(cr4, Cr4::RESET);
    }

    #[test]
    fn cr3_root_requires_page_alignment_and_round_trips() {
        let aligned = Pa::new(0x4000).and_then(Cr3::root_table);
        let unaligned = Pa::new(0x4123).and_then(Cr3::root_table);
        let restored = aligned.map(|cr3| {
            let raw = RawCr3::take(cr3);

            let RawCr3(bits) = raw;

            // SAFETY: This image was erased from a typed CR3 immediately above.
            unsafe { RawCr3(bits).restore() }
        });

        assert_eq!(aligned.map(|cr3| cr3.root().const_value()), Some(4));
        assert_eq!(unaligned, None);
        assert_eq!(restored, aligned);
    }

    #[test]
    fn long_mode_control_rejects_incomplete_control_state() {
        let cr3 = Pa::new(0x1000).and_then(Cr3::root_table);
        let mut efer = Efer::RESET;

        efer.lme_mut().const_set(State::Set);

        let efer = LongModeEfer::new(efer);
        let inactive = match (cr3, efer) {
            (Some(cr3), Some(efer)) => LongModeControl::new(Cr0::RESET, cr3, Cr4::RESET, efer),
            _ => None,
        };
        let active = match (cr3, efer) {
            (Some(cr3), Some(efer)) => {
                let mut cr0 = Cr0::RESET;
                let mut cr4 = Cr4::RESET;

                cr0.flag_pe_mut().const_set(State::Set);
                cr0.flag_pg_mut().const_set(State::Set);
                cr4.flag_pae_mut().const_set(State::Set);

                LongModeControl::new(cr0, cr3, cr4, efer)
            },
            _ => None,
        };

        assert_eq!(inactive, None);
        assert!(active.is_some(), "complete long-mode control state must validate");
    }

    #[test]
    fn control_register_fields_are_independent_named_views() {
        let mut cr0 = Cr0::RESET;
        let mut cr4 = Cr4::RESET;

        cr0.flag_pe_mut().const_set(State::Set);
        cr0.flag_pg_mut().const_set(State::Set);
        cr4.flag_pae_mut().const_set(State::Set);
        cr4.flag_osfxsr_mut().const_set(State::Set);
        cr4.flag_osxmmexcpt_mut().const_set(State::Set);
        cr4.flag_fsgsbase_mut().const_set(State::Set);
        cr4.flag_la57_mut().const_set(State::Set);

        assert_eq!(cr0.flag_pe().state(), State::Set);
        assert_eq!(cr0.flag_pg().state(), State::Set);
        assert_eq!(cr0.flag_wp().state(), State::Cleared);
        assert_eq!(cr4.flag_pae().state(), State::Set);
        assert_eq!(cr4.flag_pge().state(), State::Cleared);
        assert_eq!(cr4.flag_osfxsr().state(), State::Set);
        assert_eq!(cr4.flag_osxmmexcpt().state(), State::Set);
        assert_eq!(cr4.flag_fsgsbase().state(), State::Set);
        assert_eq!(cr4.flag_la57().state(), State::Set);
    }

    #[test]
    fn reserved_cr0_fields_have_no_mutable_architectural_api() {
        let cr0 = Cr0::RESET;

        assert_eq!(cr0.flag_et().state(), State::Set);
        assert_eq!(cr0.reserved_6_15().const_value(), 0);
        assert_eq!(cr0.flag_reserved17().state(), State::Cleared);
        assert_eq!(cr0.flag_nw().state(), State::Set);
        assert_eq!(cr0.flag_cd().state(), State::Set);
        assert_eq!(cr0.reserved_19_28().const_value(), 0);
        assert_eq!(cr0.reserved_32_63().const_value(), 0);
    }
}

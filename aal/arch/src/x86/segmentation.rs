//! Segmentation in `x86`.

use core::ops::{Deref, DerefMut};

use nekor_bitwise::prelude::{Bit, Counterpart, Field, State};

use crate::x86::{descriptor::DescriptorIndex, privilege::PrivilegeLevel};

/// A single-bit segment source in a [`SegmentSelector`].
///
/// # Layout
///
/// | Value |   Name   | Description                               |
/// |-------|----------|-------------------------------------------|
/// | 0     | [`Gdt`]  | Use the Global Descriptor Table (GDT)     |
/// | 1     | [`Ldt`]  | Use the Local Descriptor Table (LDT)      |
///
/// [`Gdt`]: TableIndicator::Gdt
/// [`Ldt`]: TableIndicator::Ldt
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
#[repr(u8)]
pub enum TableIndicator {
    /// Use the *Global Descriptor Table* as the *Segment Selector* source.
    Gdt = 0b0000_0000,

    /// Use the *Local Descriptor Table* as the *Segment Selector* source.
    Ldt = 0b0000_0001,
}

/// The bitwise field of the *Index* field of this [`SegmentSelector`].
///
/// This is the index used in the selected [`SegmentSource`].
pub type SegmentSelectorIndex<'a> = Field<'a, 3, 15, u16>;

/// The bit of the *TI* (*Table Indicator*) field of this
/// [`RawSegmentSelector`].
///
/// This determines whether the target *Descriptor Table* is the *GDT* or
/// the *LDT*.
pub type SegmentSelectorTableIndicator<'a> = Bit<'a, u16, 2>;

/// The bitwise field of the (*Requested Privilege Level*) field of this
/// [`RawSegmentSelector`].
///
/// This determines the requested [`PrivilegeLevel`] to use this selector.
pub type SegmentSelectorRpl<'a> = Field<'a, 0, 1, u16>;

/// The mutable bitwise field of the *Index* field of this [`SegmentSelector`].
///
/// This is the index used in the selected [`SegmentSource`].
pub type SegmentSelectorIndexMut<'a> = <SegmentSelectorIndex<'a> as Counterpart>::Mut;

/// The mutable bit of the *TI* (*Table Indicator*) field of this
/// [`RawSegmentSelector`].
///
/// This determines whether the target *Descriptor Table* is the *GDT* or
/// the *LDT*.
pub type SegmentSelectorTableIndicatorMut<'a> = <SegmentSelectorTableIndicator<'a> as Counterpart>::Mut;

/// The mutable bitwise field of the (*Requested Privilege Level*) field of this
/// [`RawSegmentSelector`].
///
/// This determines the requested [`PrivilegeLevel`] to use this selector.
pub type SegmentSelectorRplMut<'a> = <SegmentSelectorRpl<'a> as Counterpart>::Mut;

/// A **raw** *Segment Selector* structure.
///
/// # Layout
///
/// | Bits  | Field | Size (bits) | Description                |
/// |-------|-------|-------------|----------------------------|
/// | 15..3  | Index | 13          | Index into descriptor table|
/// | 2     | TI    | 1           | Table Indicator (GDT/LDT)  |
/// | 1-0   | RPL   | 2           | Requested Privilege Level  |
///
/// The bit layout of this type is exact to the one expected by the CPU.
///
/// This is compiled from the [`SegmentSelector`] structure.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
#[repr(transparent)]
pub struct RawSegmentSelector(u16);

impl RawSegmentSelector {
    /// Construct a zeroed [`RawSegmentSelector`].
    #[inline]
    #[must_use]
    pub const fn zeroed() -> Self {
        Self(u16::MIN)
    }

    /// Access the `Index` field (bits 3–15) of this `RawSegmentSelector`.
    #[inline]
    #[must_use]
    pub const fn index(&self) -> SegmentSelectorIndex<'_> {
        let &Self(ref target_value) = self;

        SegmentSelectorIndex::wrap(target_value)
    }

    /// Mutably access the `Index` field (bits 3–15) of this
    /// `RawSegmentSelector`.
    #[inline]
    pub const fn index_mut(&mut self) -> SegmentSelectorIndexMut<'_> {
        let &mut Self(ref mut target_value) = self;

        SegmentSelectorIndexMut::wrap(target_value)
    }

    /// Access the `TI` (Table Indicator) field (bit 2) of this
    /// `RawSegmentSelector`.
    #[inline]
    #[must_use]
    pub const fn table_indicator(&self) -> SegmentSelectorTableIndicator<'_> {
        let &Self(ref target_value) = self;

        SegmentSelectorTableIndicator::wrap(target_value)
    }

    /// Mutably access the `TI` (Table Indicator) field (bit 2) of this
    /// `RawSegmentSelector`.
    #[inline]
    pub const fn table_indicator_mut(&mut self) -> SegmentSelectorTableIndicatorMut<'_> {
        let &mut Self(ref mut target_value) = self;

        SegmentSelectorTableIndicatorMut::wrap(target_value)
    }

    /// Access the `RPL` (Requested Privilege Level) field (bits 0–1) of this
    /// `RawSegmentSelector`.
    #[inline]
    #[must_use]
    pub const fn requested_privilege_level(&self) -> SegmentSelectorRpl<'_> {
        let &Self(ref target_value) = self;

        SegmentSelectorRpl::wrap(target_value)
    }

    /// Mutably access the `RPL` (Requested Privilege Level) field (bits 0–1) of
    /// this `RawSegmentSelector`.
    #[inline]
    pub const fn requested_privilege_level_mut(&mut self) -> SegmentSelectorRplMut<'_> {
        let &mut Self(ref mut target_value) = self;

        SegmentSelectorRplMut::wrap(target_value)
    }
}

/// A high-level representation of a segment selector into a *Descriptor Table*.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct SegmentSelector {
    /// The thirteen-bit segment index into the respective *Descriptor Table*.
    descriptor_index: DescriptorIndex,

    /// Whether to use the *Global Descriptor Table* or the *Local Descriptor
    /// Table* as source of this segment.
    descriptor_table: TableIndicator,

    /// The *Requested Privilege Level* for this *Segment Selector*.
    ///
    /// This is used by the processor to evaluate an *Effective Privilege
    /// Level* to determine whether a segment descriptor is accessible, which
    /// corresponds to `max(CPL, RPL)` (i.e., the maximum number between
    /// the current *Current Privilege Level* and the *Requested Privilege
    /// Level*).
    ///
    /// When unsure, use [`PrivilegeLevel::Ring0`] to avoid any possible
    /// problems.
    requested_privilege_level: PrivilegeLevel,
}

impl SegmentSelector {
    /// Determine the raw bit-for-bit representation for the target
    /// [`SegmentSelector`].
    #[inline]
    #[must_use]
    pub const fn raw(&self) -> RawSegmentSelector {
        let &Self {
            descriptor_index,
            descriptor_table,
            requested_privilege_level,
        } = self;

        let mut target_selector = RawSegmentSelector::zeroed();

        target_selector.index_mut().const_merge(descriptor_index.raw());

        let target_state = if matches!(descriptor_table, TableIndicator::Gdt) {
            State::Cleared
        } else {
            State::Set
        };

        target_selector.table_indicator_mut().const_set(target_state);

        target_selector
            .requested_privilege_level_mut()
            .const_merge(requested_privilege_level as u8);

        target_selector
    }
}

impl SegmentSelector {
    /// The [`DescriptorIndex`] associated with this [`SegmentSelector`].
    #[inline]
    #[must_use]
    pub const fn index(&self) -> &DescriptorIndex {
        let &Self {
            ref descriptor_index, ..
        } = self;

        descriptor_index
    }

    /// The [`DescriptorIndex`] associated to this [`SegmentSelector`], but in a
    /// mutable manner.
    pub const fn index_mut(&mut self) -> &mut DescriptorIndex {
        let &mut Self {
            ref mut descriptor_index,
            ..
        } = self;

        descriptor_index
    }

    /// The [`TableIndicator`] associated to this [`SegmentSelector`].
    #[inline]
    #[must_use]
    pub const fn table(&self) -> &TableIndicator {
        let &Self {
            ref descriptor_table, ..
        } = self;

        descriptor_table
    }

    /// The [`TableIndicator`] associated to this [`SegmentSelector`], but in a
    /// mutable manner.
    #[inline]
    pub const fn table_mut(&mut self) -> &mut TableIndicator {
        let &mut Self {
            ref mut descriptor_table,
            ..
        } = self;

        descriptor_table
    }

    /// The *Requested* [`PrivilegeLevel`] of this [`SegmentSelector`].
    #[inline]
    #[must_use]
    pub const fn privilege(&self) -> &PrivilegeLevel {
        let &Self {
            ref requested_privilege_level,
            ..
        } = self;

        requested_privilege_level
    }

    /// The *Requested* [`PrivilegeLevel`] of this [`SegmentSelector`], but in a
    /// mutable manner.
    #[inline]
    pub const fn privilege_mut(&mut self) -> &mut PrivilegeLevel {
        let &mut Self {
            ref mut requested_privilege_level,
            ..
        } = self;

        requested_privilege_level
    }
}

/// The bitwise field of the *Index* field of this [`CodeSegment`].
///
/// This is the index used in the selected [`SegmentSource`].
pub type CodeSegmentSelectorIndex<'a> = Field<'a, 3, 15, u16>;

/// The bit of the *TI* (*Table Indicator*) field of this
/// [`SegmentSelector`].
///
/// This determines whether the target *Descriptor Table* is the *GDT* or
/// the *LDT*.
pub type CodeSegmentSelectorTableIndicator<'a> = Bit<'a, u16, 2>;

/// The bitwise field of the (*Current Privilege Level*) field of this
/// [`SegmentSelector`].
///
/// This determines the current [`PrivilegeLevel`] to use this selector.
pub type CodeSegmentSelectorCpl<'a> = Field<'a, 0, 1, u16>;

/// The mutable bitwise field of the *Index* field of this [`SegmentSelector`].
///
/// This is the index used in the selected [`SegmentSource`].
pub type CodeSegmentSelectorIndexMut<'a> = <CodeSegmentSelectorIndex<'a> as Counterpart>::Mut;

/// The mutable bit of the *TI* (*Table Indicator*) field of this
/// [`SegmentSelector`].
///
/// This determines whether the target *Descriptor Table* is the *GDT* or
/// the *LDT*.
pub type CodeSegmentSelectorTableIndicatorMut<'a> = <CodeSegmentSelectorTableIndicator<'a> as Counterpart>::Mut;

/// The mutable bitwise field of the (*Current Privilege Level*) field of this
/// [`RawCodeSegment`].
///
/// This determines the current [`PrivilegeLevel`] to use this selector.
pub type CodeSegmentSelectorCplMut<'a> = <CodeSegmentSelectorCpl<'a> as Counterpart>::Mut;

/// A new-type with the sole invariant that the underlying
/// [`RawSegmentSelector`] is for a valid *Code Segment Descriptor*.
///
/// For the higher-level [`SegmentSelector`] interface, defer to the
/// [`CodeSegment`] new-type instead.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct RawCodeSegment(RawSegmentSelector);

impl RawCodeSegment {
    /// Access the `Index` field (bits 3–15) of this `RawCodeSegment`.
    #[inline]
    #[must_use]
    pub const fn index(&self) -> CodeSegmentSelectorIndex<'_> {
        let &Self(RawSegmentSelector(ref target_value)) = self;

        CodeSegmentSelectorIndex::wrap(target_value)
    }

    /// Mutably access the `Index` field (bits 3–15) of this
    /// `RawCodeSegment`.
    #[inline]
    pub const fn index_mut(&mut self) -> CodeSegmentSelectorIndexMut<'_> {
        let &mut Self(RawSegmentSelector(ref mut target_value)) = self;

        CodeSegmentSelectorIndexMut::wrap(target_value)
    }

    /// Access the `TI` (Table Indicator) field (bit 2) of this
    /// `RawCodeSegment`.
    #[inline]
    #[must_use]
    pub const fn table_indicator(&self) -> CodeSegmentSelectorTableIndicator<'_> {
        let &Self(RawSegmentSelector(ref target_value)) = self;

        CodeSegmentSelectorTableIndicator::wrap(target_value)
    }

    /// Mutably access the `TI` (Table Indicator) field (bit 2) of this
    /// `RawCodeSegment`.
    #[inline]
    pub const fn table_indicator_mut(&mut self) -> CodeSegmentSelectorTableIndicatorMut<'_> {
        let &mut Self(RawSegmentSelector(ref mut target_value)) = self;

        CodeSegmentSelectorTableIndicatorMut::wrap(target_value)
    }

    /// Access the `CPL` (Current Privilege Level) field (bits 0–1) of this
    /// `RawCodeSegment`.
    #[inline]
    #[must_use]
    pub const fn current_privilege_level(&self) -> CodeSegmentSelectorCpl<'_> {
        let &Self(RawSegmentSelector(ref target_value)) = self;

        SegmentSelectorRpl::wrap(target_value)
    }

    /// Mutably access the `CPL` (Current Privilege Level) field (bits 0–1) of
    /// this `RawCodeSegment`.
    #[inline]
    pub const fn current_privilege_level_mut(&mut self) -> CodeSegmentSelectorCplMut<'_> {
        let &mut Self(RawSegmentSelector(ref mut target_value)) = self;

        CodeSegmentSelectorCplMut::wrap(target_value)
    }
}

/// A new-type with the sole invariant that the underlying
/// [`RawSegmentSelector`] is for a valid *Data Segment Descriptor*.
///
/// For the higher-level [`SegmentSelector`] interface, defer to the
/// [`DataSegment`] new-type instead.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct RawDataSegment(RawSegmentSelector);

impl Deref for RawDataSegment {
    type Target = RawSegmentSelector;

    #[inline]
    fn deref(&self) -> &Self::Target {
        let &Self(ref target_value) = self;

        target_value
    }
}

/// A new-type with the sole invariant that the underlying
/// [`RawSegmentSelector`] is for a valid *Code Segment Descriptor*.
///
/// For the lower-level [`RawSegmentSelector`]-based interface, defer to the
/// [`RawCodeSegment`] new-type instead.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct CodeSegment(SegmentSelector);

impl CodeSegment {
    /// The [`DescriptorIndex`] associated with this [`CodeSegment`].
    #[inline]
    #[must_use]
    pub const fn index(&self) -> &DescriptorIndex {
        let &Self(SegmentSelector {
            ref descriptor_index, ..
        }) = self;

        descriptor_index
    }

    /// The [`DescriptorIndex`] associated to this [`CodeSegment`], but in a
    /// mutable manner.
    pub const fn index_mut(&mut self) -> &mut DescriptorIndex {
        let &mut Self(SegmentSelector {
            ref mut descriptor_index,
            ..
        }) = self;

        descriptor_index
    }

    /// The [`TableIndicator`] associated to this [`CodeSegment`].
    #[inline]
    #[must_use]
    pub const fn table(&self) -> &TableIndicator {
        let &Self(SegmentSelector {
            ref descriptor_table, ..
        }) = self;

        descriptor_table
    }

    /// The [`TableIndicator`] associated to this [`CodeSegment`], but in a
    /// mutable manner.
    #[inline]
    pub const fn table_mut(&mut self) -> &mut TableIndicator {
        let &mut Self(SegmentSelector {
            ref mut descriptor_table,
            ..
        }) = self;

        descriptor_table
    }

    /// The *Current* [`PrivilegeLevel`] of this [`CodeSegment`].
    #[inline]
    #[must_use]
    pub const fn privilege(&self) -> &PrivilegeLevel {
        let &Self(SegmentSelector {
            ref requested_privilege_level,
            ..
        }) = self;

        requested_privilege_level
    }

    /// The *Current* [`PrivilegeLevel`] of this [`CodeSegment`], but in a
    /// mutable manner.
    #[inline]
    pub const fn privilege_mut(&mut self) -> &mut PrivilegeLevel {
        let &mut Self(SegmentSelector {
            ref mut requested_privilege_level,
            ..
        }) = self;

        requested_privilege_level
    }
}

/// A new-type with the sole invariant that the underlying
/// [`RawSegmentSelector`] is for a valid *Data Segment Descriptor*.
///
/// For the lower-level [`RawSegmentSelector`]-based interface, defer to the
/// [`RawDataSegment`] new-type instead.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct DataSegment(SegmentSelector);

impl Deref for DataSegment {
    type Target = SegmentSelector;

    #[inline]
    fn deref(&self) -> &Self::Target {
        let &Self(ref target_value) = self;

        target_value
    }
}

impl DerefMut for DataSegment {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        let &mut Self(ref mut target_value) = self;

        target_value
    }
}

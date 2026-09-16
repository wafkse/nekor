//! Exact x86-64 paging-entry layouts.
//!
//! Every public entry type in this module is a transparent `u64` matching one
//! hardware slot at its hierarchy level. Presence is bit zero of that storage.
//! A non-present entry therefore remains a normal entry value and may retain
//! software-owned bits according to the processor paging rules.
//!
//! # Shared fields
//!
//! Bits 0 through 5 have common meanings across the paging-structure levels.
//! Bits 7, 8, and 12 become level-dependent when an entry terminates the walk
//! as a large page. Address fields likewise begin at bit 12 for table pointers
//! and 4 KiB leaves, bit 21 for 2 MiB leaves, and bit 30 for 1 GiB leaves.
//!
//! Constructors in this module are conveniences for architecturally aligned
//! table pointers and leaves. They do not replace the field-level interface.
//! The complete integer representation is intentionally not exposed. A one-way
//! little-endian byte image is available for installing typed entries into
//! hardware paging-structure memory.
//!
//! # Intel SDM references
//!
//! The field ranges follow Intel SDM Vol. 3A, Section 5.5.4. PML5E and PML4E
//! correspond to Tables 5-14 and 5-15. PDPTE forms correspond to Tables 5-16
//! and 5-17. PDE forms correspond to Tables 5-18 and 5-19. PTE corresponds to
//! Table 5-20. Reserved physical-address bits above MAXPHYADDR follow Section
//! 5.1.4.

use core::mem;

use nekor_bitwise::prelude::{Bit, Counterpart, Field, State};

use super::address::Pa;

/// The "Present" (P) flag. Bit zero determines whether the processor treats the
/// entry as present.
pub type EntryPresent<'value> = Bit<'value, u64, 0>;

/// The "Read/Write" (R/W) flag. Set permits writes subject to higher-level
/// protection.
pub type EntryWritable<'value> = Bit<'value, u64, 1>;

/// The "User/Supervisor" (U/S) flag. Set permits user-mode access subject to
/// the complete walk.
pub type EntryUser<'value> = Bit<'value, u64, 2>;

/// The page-level write-through (PWT) flag.
pub type EntryWriteThrough<'value> = Bit<'value, u64, 3>;

/// The page-level cache-disable (PCD) flag.
pub type EntryCacheDisable<'value> = Bit<'value, u64, 4>;

/// The processor-maintained "Accessed" (A) flag.
pub type EntryAccessed<'value> = Bit<'value, u64, 5>;

/// The processor-maintained "Dirty" (D) flag used by leaf mappings.
pub type EntryDirty<'value> = Bit<'value, u64, 6>;

/// The "Page Size" (PS) flag used by PDPT and page-directory entries.
pub type EntryPageSize<'value> = Bit<'value, u64, 7>;

/// The PAT selector bit used by a 4 KiB page-table leaf.
pub type EntryPat4K<'value> = Bit<'value, u64, 7>;

/// The "Global" (G) flag used by leaf mappings.
pub type EntryGlobal<'value> = Bit<'value, u64, 8>;

/// The low software-available field spanning bits 9 through 11.
pub type EntryAvailableLow<'value> = Field<'value, 9, 11, u64>;

/// The PAT selector bit used by 2 MiB and 1 GiB leaf mappings.
pub type EntryPatLarge<'value> = Bit<'value, u64, 12>;

/// The address field used by table pointers and 4 KiB leaves.
pub type EntryAddress4KiB<'value> = Field<'value, 12, 51, u64>;

/// Reserved address-alignment field in a 2 MiB leaf.
pub type EntryReserved2MiB<'value> = Field<'value, 13, 20, u64>;

/// The physical page-number field used by a 2 MiB leaf.
pub type EntryAddress2MiB<'value> = Field<'value, 21, 51, u64>;

/// Reserved address-alignment field in a 1 GiB leaf.
pub type EntryReserved1GiB<'value> = Field<'value, 13, 29, u64>;

/// The physical page-number field used by a 1 GiB leaf.
pub type EntryAddress1GiB<'value> = Field<'value, 30, 51, u64>;

/// The high software-available field spanning bits 52 through 58.
pub type EntryAvailableHigh<'value> = Field<'value, 52, 58, u64>;

/// The protection-key field used by leaf mappings when protection keys are
/// enabled.
pub type EntryProtectionKey<'value> = Field<'value, 59, 62, u64>;

/// The "Execute Disable" (XD/NX) flag when execute-disable support is enabled.
pub type EntryNoExecute<'value> = Bit<'value, u64, 63>;

/// A mutable counterpart to [`EntryPresent`].
pub type EntryPresentMut<'value> = <EntryPresent<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`EntryWritable`].
pub type EntryWritableMut<'value> = <EntryWritable<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`EntryUser`].
pub type EntryUserMut<'value> = <EntryUser<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`EntryWriteThrough`].
pub type EntryWriteThroughMut<'value> = <EntryWriteThrough<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`EntryCacheDisable`].
pub type EntryCacheDisableMut<'value> = <EntryCacheDisable<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`EntryAccessed`].
pub type EntryAccessedMut<'value> = <EntryAccessed<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`EntryDirty`].
pub type EntryDirtyMut<'value> = <EntryDirty<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`EntryPageSize`].
pub type EntryPageSizeMut<'value> = <EntryPageSize<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`EntryPat4K`].
pub type EntryPat4KMut<'value> = <EntryPat4K<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`EntryGlobal`].
pub type EntryGlobalMut<'value> = <EntryGlobal<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`EntryAvailableLow`].
pub type EntryAvailableLowMut<'value> = <EntryAvailableLow<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`EntryPatLarge`].
pub type EntryPatLargeMut<'value> = <EntryPatLarge<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`EntryAddress4KiB`].
pub type EntryAddress4KiBMut<'value> = <EntryAddress4KiB<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`EntryAddress2MiB`].
pub type EntryAddress2MiBMut<'value> = <EntryAddress2MiB<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`EntryAddress1GiB`].
pub type EntryAddress1GiBMut<'value> = <EntryAddress1GiB<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`EntryAvailableHigh`].
pub type EntryAvailableHighMut<'value> = <EntryAvailableHigh<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`EntryProtectionKey`].
pub type EntryProtectionKeyMut<'value> = <EntryProtectionKey<'value> as Counterpart>::Mut;

/// A mutable counterpart to [`EntryNoExecute`].
pub type EntryNoExecuteMut<'value> = <EntryNoExecute<'value> as Counterpart>::Mut;

/// One exact PML5 entry.
///
/// A present PML5 entry always selects a PML4 table. It exists only in an LA57
/// five-level walk and cannot terminate the walk as a leaf.
///
/// # Relevant layout
///
/// | Bits | Meaning |
/// |---|---|
/// | 0 | Present |
/// | 1 | Read/write |
/// | 2 | User/supervisor |
/// | 3 | Page-level write-through |
/// | 4 | Page-level cache-disable |
/// | 5 | Accessed |
/// | 12..51 | PML4 physical page number |
/// | 52..58 | Software available |
/// | 63 | Execute disable |
///
/// Bit 7 is not a page-size selector at this level.
///
/// See Intel SDM Vol. 3A, Table 5-14.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar is always one complete architectural PML5 entry image.
pub struct Pml5e(u64);

/// One exact PML4 entry.
///
/// A present PML4 entry always selects a page-directory-pointer table. It is
/// the root entry under LA48 and the second entry under LA57. It cannot
/// terminate the walk as a leaf.
///
/// # Relevant layout
///
/// | Bits | Meaning |
/// |---|---|
/// | 0 | Present |
/// | 1 | Read/write |
/// | 2 | User/supervisor |
/// | 3 | Page-level write-through |
/// | 4 | Page-level cache-disable |
/// | 5 | Accessed |
/// | 12..51 | PDPT physical page number |
/// | 52..58 | Software available |
/// | 63 | Execute disable |
///
/// Bit 7 is not a page-size selector at this level.
///
/// See Intel SDM Vol. 3A, Table 5-15.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar is always one complete architectural PML4 entry image.
pub struct Pml4e(u64);

/// One exact page-directory-pointer-table entry.
///
/// A present PDPT entry has two architectural interpretations selected by PS.
/// With PS clear, bits 12 through 51 name the next page-directory page. With
/// PS set, the entry is a 1 GiB leaf and bits 30 through 51 name the mapped
/// physical page while bits 13 through 29 are reserved.
///
/// # Level-dependent fields
///
/// | Field | PS clear | PS set |
/// |---|---|---|
/// | Bit 6 | non-leaf semantics | Dirty |
/// | Bit 7 | Page Size selector | Page Size selector |
/// | Bit 8 | non-leaf semantics | Global |
/// | Bit 12 | address field | PAT |
/// | Bits 13..29 | address field | Reserved |
/// | Bits 30..51 | address field | 1 GiB physical page number |
/// | Bits 59..62 | non-leaf semantics | Protection key |
///
/// See Intel SDM Vol. 3A, Tables 5-16 and 5-17.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar is always one complete architectural PDPT entry image.
pub struct Pdpte(u64);

/// One exact page-directory entry.
///
/// A present page-directory entry has two architectural interpretations
/// selected by PS. With PS clear, bits 12 through 51 name the next page-table
/// page. With PS set, the entry is a 2 MiB leaf and bits 21 through 51 name the
/// mapped physical page while bits 13 through 20 are reserved.
///
/// # Level-dependent fields
///
/// | Field | PS clear | PS set |
/// |---|---|---|
/// | Bit 6 | non-leaf semantics | Dirty |
/// | Bit 7 | Page Size selector | Page Size selector |
/// | Bit 8 | non-leaf semantics | Global |
/// | Bit 12 | address field | PAT |
/// | Bits 13..20 | address field | Reserved |
/// | Bits 21..51 | address field | 2 MiB physical page number |
/// | Bits 59..62 | non-leaf semantics | Protection key |
///
/// See Intel SDM Vol. 3A, Tables 5-18 and 5-19.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar is always one complete architectural page-directory entry
// image.
pub struct Pde(u64);

/// One exact page-table entry.
///
/// A present page-table entry is always a 4 KiB leaf in the shared lower walk.
/// Unlike PDPT and page-directory leaves, its PAT selector occupies bit 7 and
/// its physical page number begins at bit 12.
///
/// # Relevant layout
///
/// | Bits | Meaning |
/// |---|---|
/// | 0 | Present |
/// | 1 | Read/write |
/// | 2 | User/supervisor |
/// | 3 | Page-level write-through |
/// | 4 | Page-level cache-disable |
/// | 5 | Accessed |
/// | 6 | Dirty |
/// | 7 | PAT |
/// | 8 | Global |
/// | 12..51 | 4 KiB physical page number |
/// | 52..58 | Software available |
/// | 59..62 | Protection key |
/// | 63 | Execute disable |
///
/// See Intel SDM Vol. 3A, Table 5-20.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private scalar is always one complete architectural page-table entry image.
pub struct Pte(u64);

impl Pml5e {
    /// Construct an all-zero non-present entry.
    #[inline]
    #[must_use]
    pub const fn empty() -> Self {
        Self(u64::MIN)
    }

    /// Returns the little-endian hardware memory image of this entry.
    #[inline]
    #[must_use]
    pub const fn to_le_bytes(self) -> [u8; 8] {
        let Self(value) = self;

        value.to_le_bytes()
    }

    /// Determine the "Present" (P) flag.
    #[inline]
    #[must_use]
    pub const fn present(&self) -> EntryPresent<'_> {
        let &Self(ref target_value) = self;

        EntryPresent::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Present" (P) flag.
    #[inline]
    pub const fn present_mut(&mut self) -> EntryPresentMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryPresentMut::wrap(target_value)
    }

    /// Determine the "Read/Write" (R/W) flag.
    #[inline]
    #[must_use]
    pub const fn writable(&self) -> EntryWritable<'_> {
        let &Self(ref target_value) = self;

        EntryWritable::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Read/Write" (R/W) flag.
    #[inline]
    pub const fn writable_mut(&mut self) -> EntryWritableMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryWritableMut::wrap(target_value)
    }

    /// Determine the "User/Supervisor" (U/S) flag.
    #[inline]
    #[must_use]
    pub const fn user(&self) -> EntryUser<'_> {
        let &Self(ref target_value) = self;

        EntryUser::wrap(target_value)
    }

    /// Resolve a mutable reference to the "User/Supervisor" (U/S) flag.
    #[inline]
    pub const fn user_mut(&mut self) -> EntryUserMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryUserMut::wrap(target_value)
    }

    /// Determine the page-level write-through (PWT) flag.
    #[inline]
    #[must_use]
    pub const fn write_through(&self) -> EntryWriteThrough<'_> {
        let &Self(ref target_value) = self;

        EntryWriteThrough::wrap(target_value)
    }

    /// Resolve a mutable reference to the page-level write-through (PWT) flag.
    #[inline]
    pub const fn write_through_mut(&mut self) -> EntryWriteThroughMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryWriteThroughMut::wrap(target_value)
    }

    /// Determine the page-level cache-disable (PCD) flag.
    #[inline]
    #[must_use]
    pub const fn cache_disable(&self) -> EntryCacheDisable<'_> {
        let &Self(ref target_value) = self;

        EntryCacheDisable::wrap(target_value)
    }

    /// Resolve a mutable reference to the page-level cache-disable (PCD) flag.
    #[inline]
    pub const fn cache_disable_mut(&mut self) -> EntryCacheDisableMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryCacheDisableMut::wrap(target_value)
    }

    /// Determine the processor-maintained "Accessed" (A) flag.
    #[inline]
    #[must_use]
    pub const fn accessed(&self) -> EntryAccessed<'_> {
        let &Self(ref target_value) = self;

        EntryAccessed::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Accessed" (A) flag.
    #[inline]
    pub const fn accessed_mut(&mut self) -> EntryAccessedMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryAccessedMut::wrap(target_value)
    }

    /// Determine the low software-available field.
    #[inline]
    #[must_use]
    pub const fn available_low(&self) -> EntryAvailableLow<'_> {
        let &Self(ref target_value) = self;

        EntryAvailableLow::wrap(target_value)
    }

    /// Resolve a mutable reference to the low software-available field.
    #[inline]
    pub const fn available_low_mut(&mut self) -> EntryAvailableLowMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryAvailableLowMut::wrap(target_value)
    }

    /// Determine the high software-available field.
    #[inline]
    #[must_use]
    pub const fn available_high(&self) -> EntryAvailableHigh<'_> {
        let &Self(ref target_value) = self;

        EntryAvailableHigh::wrap(target_value)
    }

    /// Resolve a mutable reference to the high software-available field.
    #[inline]
    pub const fn available_high_mut(&mut self) -> EntryAvailableHighMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryAvailableHighMut::wrap(target_value)
    }

    /// Determine the execute-disable (XD/NX) flag.
    #[inline]
    #[must_use]
    pub const fn no_execute(&self) -> EntryNoExecute<'_> {
        let &Self(ref target_value) = self;

        EntryNoExecute::wrap(target_value)
    }

    /// Resolve a mutable reference to the execute-disable (XD/NX) flag.
    #[inline]
    pub const fn no_execute_mut(&mut self) -> EntryNoExecuteMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryNoExecuteMut::wrap(target_value)
    }

    /// Create a present PML4 table-pointer entry.
    ///
    /// Returns `None` when `address` is not 4 KiB-aligned.
    #[inline]
    #[must_use]
    pub const fn table(address: Pa) -> Option<Self> {
        match address.offset_4kib().const_value() {
            0 => {
                let mut entry = Self::empty();

                entry.present_mut().const_set(State::Set);
                entry.address_mut().const_merge(address.frame_4kib().const_value());

                Some(entry)
            },
            _ => None,
        }
    }

    /// Determine the PML4 physical page-number field.
    #[inline]
    #[must_use]
    pub const fn address(&self) -> EntryAddress4KiB<'_> {
        let &Self(ref target_value) = self;

        EntryAddress4KiB::wrap(target_value)
    }

    /// Resolve a mutable reference to the PML4 physical page-number field.
    #[inline]
    pub const fn address_mut(&mut self) -> EntryAddress4KiBMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryAddress4KiBMut::wrap(target_value)
    }
}

impl Pml4e {
    /// Construct an all-zero non-present entry.
    #[inline]
    #[must_use]
    pub const fn empty() -> Self {
        Self(u64::MIN)
    }

    /// Returns the little-endian hardware memory image of this entry.
    #[inline]
    #[must_use]
    pub const fn to_le_bytes(self) -> [u8; 8] {
        let Self(value) = self;

        value.to_le_bytes()
    }

    /// Determine the "Present" (P) flag.
    #[inline]
    #[must_use]
    pub const fn present(&self) -> EntryPresent<'_> {
        let &Self(ref target_value) = self;

        EntryPresent::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Present" (P) flag.
    #[inline]
    pub const fn present_mut(&mut self) -> EntryPresentMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryPresentMut::wrap(target_value)
    }

    /// Determine the "Read/Write" (R/W) flag.
    #[inline]
    #[must_use]
    pub const fn writable(&self) -> EntryWritable<'_> {
        let &Self(ref target_value) = self;

        EntryWritable::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Read/Write" (R/W) flag.
    #[inline]
    pub const fn writable_mut(&mut self) -> EntryWritableMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryWritableMut::wrap(target_value)
    }

    /// Determine the "User/Supervisor" (U/S) flag.
    #[inline]
    #[must_use]
    pub const fn user(&self) -> EntryUser<'_> {
        let &Self(ref target_value) = self;

        EntryUser::wrap(target_value)
    }

    /// Resolve a mutable reference to the "User/Supervisor" (U/S) flag.
    #[inline]
    pub const fn user_mut(&mut self) -> EntryUserMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryUserMut::wrap(target_value)
    }

    /// Determine the page-level write-through (PWT) flag.
    #[inline]
    #[must_use]
    pub const fn write_through(&self) -> EntryWriteThrough<'_> {
        let &Self(ref target_value) = self;

        EntryWriteThrough::wrap(target_value)
    }

    /// Resolve a mutable reference to the page-level write-through (PWT) flag.
    #[inline]
    pub const fn write_through_mut(&mut self) -> EntryWriteThroughMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryWriteThroughMut::wrap(target_value)
    }

    /// Determine the page-level cache-disable (PCD) flag.
    #[inline]
    #[must_use]
    pub const fn cache_disable(&self) -> EntryCacheDisable<'_> {
        let &Self(ref target_value) = self;

        EntryCacheDisable::wrap(target_value)
    }

    /// Resolve a mutable reference to the page-level cache-disable (PCD) flag.
    #[inline]
    pub const fn cache_disable_mut(&mut self) -> EntryCacheDisableMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryCacheDisableMut::wrap(target_value)
    }

    /// Determine the processor-maintained "Accessed" (A) flag.
    #[inline]
    #[must_use]
    pub const fn accessed(&self) -> EntryAccessed<'_> {
        let &Self(ref target_value) = self;

        EntryAccessed::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Accessed" (A) flag.
    #[inline]
    pub const fn accessed_mut(&mut self) -> EntryAccessedMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryAccessedMut::wrap(target_value)
    }

    /// Determine the low software-available field.
    #[inline]
    #[must_use]
    pub const fn available_low(&self) -> EntryAvailableLow<'_> {
        let &Self(ref target_value) = self;

        EntryAvailableLow::wrap(target_value)
    }

    /// Resolve a mutable reference to the low software-available field.
    #[inline]
    pub const fn available_low_mut(&mut self) -> EntryAvailableLowMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryAvailableLowMut::wrap(target_value)
    }

    /// Determine the high software-available field.
    #[inline]
    #[must_use]
    pub const fn available_high(&self) -> EntryAvailableHigh<'_> {
        let &Self(ref target_value) = self;

        EntryAvailableHigh::wrap(target_value)
    }

    /// Resolve a mutable reference to the high software-available field.
    #[inline]
    pub const fn available_high_mut(&mut self) -> EntryAvailableHighMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryAvailableHighMut::wrap(target_value)
    }

    /// Determine the execute-disable (XD/NX) flag.
    #[inline]
    #[must_use]
    pub const fn no_execute(&self) -> EntryNoExecute<'_> {
        let &Self(ref target_value) = self;

        EntryNoExecute::wrap(target_value)
    }

    /// Resolve a mutable reference to the execute-disable (XD/NX) flag.
    #[inline]
    pub const fn no_execute_mut(&mut self) -> EntryNoExecuteMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryNoExecuteMut::wrap(target_value)
    }

    /// Create a present PDPT table-pointer entry.
    ///
    /// Returns `None` when `address` is not 4 KiB-aligned.
    #[inline]
    #[must_use]
    pub const fn table(address: Pa) -> Option<Self> {
        match address.offset_4kib().const_value() {
            0 => {
                let mut entry = Self::empty();

                entry.present_mut().const_set(State::Set);
                entry.address_mut().const_merge(address.frame_4kib().const_value());

                Some(entry)
            },
            _ => None,
        }
    }

    /// Determine the PDPT physical page-number field.
    #[inline]
    #[must_use]
    pub const fn address(&self) -> EntryAddress4KiB<'_> {
        let &Self(ref target_value) = self;

        EntryAddress4KiB::wrap(target_value)
    }

    /// Resolve a mutable reference to the PDPT physical page-number field.
    #[inline]
    pub const fn address_mut(&mut self) -> EntryAddress4KiBMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryAddress4KiBMut::wrap(target_value)
    }
}

impl Pdpte {
    /// Construct an all-zero non-present entry.
    #[inline]
    #[must_use]
    pub const fn empty() -> Self {
        Self(u64::MIN)
    }

    /// Returns the little-endian hardware memory image of this entry.
    #[inline]
    #[must_use]
    pub const fn to_le_bytes(self) -> [u8; 8] {
        let Self(value) = self;

        value.to_le_bytes()
    }

    /// Determine the "Present" (P) flag.
    #[inline]
    #[must_use]
    pub const fn present(&self) -> EntryPresent<'_> {
        let &Self(ref target_value) = self;

        EntryPresent::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Present" (P) flag.
    #[inline]
    pub const fn present_mut(&mut self) -> EntryPresentMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryPresentMut::wrap(target_value)
    }

    /// Determine the "Read/Write" (R/W) flag.
    #[inline]
    #[must_use]
    pub const fn writable(&self) -> EntryWritable<'_> {
        let &Self(ref target_value) = self;

        EntryWritable::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Read/Write" (R/W) flag.
    #[inline]
    pub const fn writable_mut(&mut self) -> EntryWritableMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryWritableMut::wrap(target_value)
    }

    /// Determine the "User/Supervisor" (U/S) flag.
    #[inline]
    #[must_use]
    pub const fn user(&self) -> EntryUser<'_> {
        let &Self(ref target_value) = self;

        EntryUser::wrap(target_value)
    }

    /// Resolve a mutable reference to the "User/Supervisor" (U/S) flag.
    #[inline]
    pub const fn user_mut(&mut self) -> EntryUserMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryUserMut::wrap(target_value)
    }

    /// Determine the page-level write-through (PWT) flag.
    #[inline]
    #[must_use]
    pub const fn write_through(&self) -> EntryWriteThrough<'_> {
        let &Self(ref target_value) = self;

        EntryWriteThrough::wrap(target_value)
    }

    /// Resolve a mutable reference to the page-level write-through (PWT) flag.
    #[inline]
    pub const fn write_through_mut(&mut self) -> EntryWriteThroughMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryWriteThroughMut::wrap(target_value)
    }

    /// Determine the page-level cache-disable (PCD) flag.
    #[inline]
    #[must_use]
    pub const fn cache_disable(&self) -> EntryCacheDisable<'_> {
        let &Self(ref target_value) = self;

        EntryCacheDisable::wrap(target_value)
    }

    /// Resolve a mutable reference to the page-level cache-disable (PCD) flag.
    #[inline]
    pub const fn cache_disable_mut(&mut self) -> EntryCacheDisableMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryCacheDisableMut::wrap(target_value)
    }

    /// Determine the processor-maintained "Accessed" (A) flag.
    #[inline]
    #[must_use]
    pub const fn accessed(&self) -> EntryAccessed<'_> {
        let &Self(ref target_value) = self;

        EntryAccessed::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Accessed" (A) flag.
    #[inline]
    pub const fn accessed_mut(&mut self) -> EntryAccessedMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryAccessedMut::wrap(target_value)
    }

    /// Determine the low software-available field.
    #[inline]
    #[must_use]
    pub const fn available_low(&self) -> EntryAvailableLow<'_> {
        let &Self(ref target_value) = self;

        EntryAvailableLow::wrap(target_value)
    }

    /// Resolve a mutable reference to the low software-available field.
    #[inline]
    pub const fn available_low_mut(&mut self) -> EntryAvailableLowMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryAvailableLowMut::wrap(target_value)
    }

    /// Determine the high software-available field.
    #[inline]
    #[must_use]
    pub const fn available_high(&self) -> EntryAvailableHigh<'_> {
        let &Self(ref target_value) = self;

        EntryAvailableHigh::wrap(target_value)
    }

    /// Resolve a mutable reference to the high software-available field.
    #[inline]
    pub const fn available_high_mut(&mut self) -> EntryAvailableHighMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryAvailableHighMut::wrap(target_value)
    }

    /// Determine the execute-disable (XD/NX) flag.
    #[inline]
    #[must_use]
    pub const fn no_execute(&self) -> EntryNoExecute<'_> {
        let &Self(ref target_value) = self;

        EntryNoExecute::wrap(target_value)
    }

    /// Resolve a mutable reference to the execute-disable (XD/NX) flag.
    #[inline]
    pub const fn no_execute_mut(&mut self) -> EntryNoExecuteMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryNoExecuteMut::wrap(target_value)
    }

    /// Create a present page-directory table-pointer entry.
    ///
    /// Returns `None` when `address` is not 4 KiB-aligned.
    #[inline]
    #[must_use]
    pub const fn table(address: Pa) -> Option<Self> {
        match address.offset_4kib().const_value() {
            0 => {
                let mut entry = Self::empty();

                entry.present_mut().const_set(State::Set);
                entry
                    .table_address_mut()
                    .const_merge(address.frame_4kib().const_value());

                Some(entry)
            },
            _ => None,
        }
    }

    /// Create a present 1 GiB leaf entry.
    ///
    /// Returns `None` when `address` is not 1 GiB-aligned.
    #[inline]
    #[must_use]
    pub const fn page_1gib(address: Pa) -> Option<Self> {
        match address.offset_1gib().const_value() {
            0 => {
                let mut entry = Self::empty();

                entry.present_mut().const_set(State::Set);
                entry.page_size_mut().const_set(State::Set);
                entry.page_address_mut().const_merge(address.frame_1gib().const_value());

                Some(entry)
            },
            _ => None,
        }
    }

    /// Determine the "Page Size" (PS) flag.
    #[inline]
    #[must_use]
    pub const fn page_size(&self) -> EntryPageSize<'_> {
        let &Self(ref target_value) = self;

        EntryPageSize::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Page Size" (PS) flag.
    #[inline]
    pub const fn page_size_mut(&mut self) -> EntryPageSizeMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryPageSizeMut::wrap(target_value)
    }

    /// Determine the page-directory physical page-number field when PS is
    /// clear.
    #[inline]
    #[must_use]
    pub const fn table_address(&self) -> EntryAddress4KiB<'_> {
        let &Self(ref target_value) = self;

        EntryAddress4KiB::wrap(target_value)
    }

    /// Resolve a mutable reference to the page-directory physical page-number
    /// field when PS is clear.
    #[inline]
    pub const fn table_address_mut(&mut self) -> EntryAddress4KiBMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryAddress4KiBMut::wrap(target_value)
    }

    /// Determine the 1 GiB physical page-number field when PS is set.
    #[inline]
    #[must_use]
    pub const fn page_address(&self) -> EntryAddress1GiB<'_> {
        let &Self(ref target_value) = self;

        EntryAddress1GiB::wrap(target_value)
    }

    /// Resolve a mutable reference to the 1 GiB physical page-number field when
    /// PS is set.
    #[inline]
    pub const fn page_address_mut(&mut self) -> EntryAddress1GiBMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryAddress1GiBMut::wrap(target_value)
    }

    /// Determine the leaf "Dirty" (D) flag when PS is set.
    #[inline]
    #[must_use]
    pub const fn dirty(&self) -> EntryDirty<'_> {
        let &Self(ref target_value) = self;

        EntryDirty::wrap(target_value)
    }

    /// Resolve a mutable reference to the leaf "Dirty" (D) flag when PS is set.
    #[inline]
    pub const fn dirty_mut(&mut self) -> EntryDirtyMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryDirtyMut::wrap(target_value)
    }

    /// Determine the leaf "Global" (G) flag when PS is set.
    #[inline]
    #[must_use]
    pub const fn global(&self) -> EntryGlobal<'_> {
        let &Self(ref target_value) = self;

        EntryGlobal::wrap(target_value)
    }

    /// Resolve a mutable reference to the leaf "Global" (G) flag when PS is
    /// set.
    #[inline]
    pub const fn global_mut(&mut self) -> EntryGlobalMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryGlobalMut::wrap(target_value)
    }

    /// Determine the large-page PAT selector when PS is set.
    #[inline]
    #[must_use]
    pub const fn page_attribute_table(&self) -> EntryPatLarge<'_> {
        let &Self(ref target_value) = self;

        EntryPatLarge::wrap(target_value)
    }

    /// Resolve a mutable reference to the large-page PAT selector when PS is
    /// set.
    #[inline]
    pub const fn page_attribute_table_mut(&mut self) -> EntryPatLargeMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryPatLargeMut::wrap(target_value)
    }

    /// Determine the protection-key field when this entry is a 1 GiB leaf.
    #[inline]
    #[must_use]
    pub const fn protection_key(&self) -> EntryProtectionKey<'_> {
        let &Self(ref target_value) = self;

        EntryProtectionKey::wrap(target_value)
    }

    /// Resolve a mutable reference to the protection-key field of a 1 GiB leaf.
    #[inline]
    pub const fn protection_key_mut(&mut self) -> EntryProtectionKeyMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryProtectionKeyMut::wrap(target_value)
    }

    /// Determine reserved bits 13 through 29 of a 1 GiB leaf.
    #[inline]
    #[must_use]
    pub const fn reserved_1gib(&self) -> EntryReserved1GiB<'_> {
        let &Self(ref target_value) = self;

        EntryReserved1GiB::wrap(target_value)
    }
}

impl Pde {
    /// Construct an all-zero non-present entry.
    #[inline]
    #[must_use]
    pub const fn empty() -> Self {
        Self(u64::MIN)
    }

    /// Returns the little-endian hardware memory image of this entry.
    #[inline]
    #[must_use]
    pub const fn to_le_bytes(self) -> [u8; 8] {
        let Self(value) = self;

        value.to_le_bytes()
    }

    /// Determine the "Present" (P) flag.
    #[inline]
    #[must_use]
    pub const fn present(&self) -> EntryPresent<'_> {
        let &Self(ref target_value) = self;

        EntryPresent::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Present" (P) flag.
    #[inline]
    pub const fn present_mut(&mut self) -> EntryPresentMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryPresentMut::wrap(target_value)
    }

    /// Determine the "Read/Write" (R/W) flag.
    #[inline]
    #[must_use]
    pub const fn writable(&self) -> EntryWritable<'_> {
        let &Self(ref target_value) = self;

        EntryWritable::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Read/Write" (R/W) flag.
    #[inline]
    pub const fn writable_mut(&mut self) -> EntryWritableMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryWritableMut::wrap(target_value)
    }

    /// Determine the "User/Supervisor" (U/S) flag.
    #[inline]
    #[must_use]
    pub const fn user(&self) -> EntryUser<'_> {
        let &Self(ref target_value) = self;

        EntryUser::wrap(target_value)
    }

    /// Resolve a mutable reference to the "User/Supervisor" (U/S) flag.
    #[inline]
    pub const fn user_mut(&mut self) -> EntryUserMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryUserMut::wrap(target_value)
    }

    /// Determine the page-level write-through (PWT) flag.
    #[inline]
    #[must_use]
    pub const fn write_through(&self) -> EntryWriteThrough<'_> {
        let &Self(ref target_value) = self;

        EntryWriteThrough::wrap(target_value)
    }

    /// Resolve a mutable reference to the page-level write-through (PWT) flag.
    #[inline]
    pub const fn write_through_mut(&mut self) -> EntryWriteThroughMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryWriteThroughMut::wrap(target_value)
    }

    /// Determine the page-level cache-disable (PCD) flag.
    #[inline]
    #[must_use]
    pub const fn cache_disable(&self) -> EntryCacheDisable<'_> {
        let &Self(ref target_value) = self;

        EntryCacheDisable::wrap(target_value)
    }

    /// Resolve a mutable reference to the page-level cache-disable (PCD) flag.
    #[inline]
    pub const fn cache_disable_mut(&mut self) -> EntryCacheDisableMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryCacheDisableMut::wrap(target_value)
    }

    /// Determine the processor-maintained "Accessed" (A) flag.
    #[inline]
    #[must_use]
    pub const fn accessed(&self) -> EntryAccessed<'_> {
        let &Self(ref target_value) = self;

        EntryAccessed::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Accessed" (A) flag.
    #[inline]
    pub const fn accessed_mut(&mut self) -> EntryAccessedMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryAccessedMut::wrap(target_value)
    }

    /// Determine the low software-available field.
    #[inline]
    #[must_use]
    pub const fn available_low(&self) -> EntryAvailableLow<'_> {
        let &Self(ref target_value) = self;

        EntryAvailableLow::wrap(target_value)
    }

    /// Resolve a mutable reference to the low software-available field.
    #[inline]
    pub const fn available_low_mut(&mut self) -> EntryAvailableLowMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryAvailableLowMut::wrap(target_value)
    }

    /// Determine the high software-available field.
    #[inline]
    #[must_use]
    pub const fn available_high(&self) -> EntryAvailableHigh<'_> {
        let &Self(ref target_value) = self;

        EntryAvailableHigh::wrap(target_value)
    }

    /// Resolve a mutable reference to the high software-available field.
    #[inline]
    pub const fn available_high_mut(&mut self) -> EntryAvailableHighMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryAvailableHighMut::wrap(target_value)
    }

    /// Determine the execute-disable (XD/NX) flag.
    #[inline]
    #[must_use]
    pub const fn no_execute(&self) -> EntryNoExecute<'_> {
        let &Self(ref target_value) = self;

        EntryNoExecute::wrap(target_value)
    }

    /// Resolve a mutable reference to the execute-disable (XD/NX) flag.
    #[inline]
    pub const fn no_execute_mut(&mut self) -> EntryNoExecuteMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryNoExecuteMut::wrap(target_value)
    }

    /// Create a present page-table pointer entry.
    ///
    /// Returns `None` when `address` is not 4 KiB-aligned.
    #[inline]
    #[must_use]
    pub const fn table(address: Pa) -> Option<Self> {
        match address.offset_4kib().const_value() {
            0 => {
                let mut entry = Self::empty();

                entry.present_mut().const_set(State::Set);
                entry
                    .table_address_mut()
                    .const_merge(address.frame_4kib().const_value());

                Some(entry)
            },
            _ => None,
        }
    }

    /// Create a present 2 MiB leaf entry.
    ///
    /// Returns `None` when `address` is not 2 MiB-aligned.
    #[inline]
    #[must_use]
    pub const fn page_2mib(address: Pa) -> Option<Self> {
        match address.offset_2mib().const_value() {
            0 => {
                let mut entry = Self::empty();

                entry.present_mut().const_set(State::Set);
                entry.page_size_mut().const_set(State::Set);
                entry.page_address_mut().const_merge(address.frame_2mib().const_value());

                Some(entry)
            },
            _ => None,
        }
    }

    /// Determine the "Page Size" (PS) flag.
    #[inline]
    #[must_use]
    pub const fn page_size(&self) -> EntryPageSize<'_> {
        let &Self(ref target_value) = self;

        EntryPageSize::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Page Size" (PS) flag.
    #[inline]
    pub const fn page_size_mut(&mut self) -> EntryPageSizeMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryPageSizeMut::wrap(target_value)
    }

    /// Determine the page-table physical page-number field when PS is clear.
    #[inline]
    #[must_use]
    pub const fn table_address(&self) -> EntryAddress4KiB<'_> {
        let &Self(ref target_value) = self;

        EntryAddress4KiB::wrap(target_value)
    }

    /// Resolve a mutable reference to the page-table physical page-number field
    /// when PS is clear.
    #[inline]
    pub const fn table_address_mut(&mut self) -> EntryAddress4KiBMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryAddress4KiBMut::wrap(target_value)
    }

    /// Determine the 2 MiB physical page-number field when PS is set.
    #[inline]
    #[must_use]
    pub const fn page_address(&self) -> EntryAddress2MiB<'_> {
        let &Self(ref target_value) = self;

        EntryAddress2MiB::wrap(target_value)
    }

    /// Resolve a mutable reference to the 2 MiB physical page-number field when
    /// PS is set.
    #[inline]
    pub const fn page_address_mut(&mut self) -> EntryAddress2MiBMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryAddress2MiBMut::wrap(target_value)
    }

    /// Determine the leaf "Dirty" (D) flag when PS is set.
    #[inline]
    #[must_use]
    pub const fn dirty(&self) -> EntryDirty<'_> {
        let &Self(ref target_value) = self;

        EntryDirty::wrap(target_value)
    }

    /// Resolve a mutable reference to the leaf "Dirty" (D) flag when PS is set.
    #[inline]
    pub const fn dirty_mut(&mut self) -> EntryDirtyMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryDirtyMut::wrap(target_value)
    }

    /// Determine the leaf "Global" (G) flag when PS is set.
    #[inline]
    #[must_use]
    pub const fn global(&self) -> EntryGlobal<'_> {
        let &Self(ref target_value) = self;

        EntryGlobal::wrap(target_value)
    }

    /// Resolve a mutable reference to the leaf "Global" (G) flag when PS is
    /// set.
    #[inline]
    pub const fn global_mut(&mut self) -> EntryGlobalMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryGlobalMut::wrap(target_value)
    }

    /// Determine the large-page PAT selector when PS is set.
    #[inline]
    #[must_use]
    pub const fn page_attribute_table(&self) -> EntryPatLarge<'_> {
        let &Self(ref target_value) = self;

        EntryPatLarge::wrap(target_value)
    }

    /// Resolve a mutable reference to the large-page PAT selector when PS is
    /// set.
    #[inline]
    pub const fn page_attribute_table_mut(&mut self) -> EntryPatLargeMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryPatLargeMut::wrap(target_value)
    }

    /// Determine the protection-key field when this entry is a 2 MiB leaf.
    #[inline]
    #[must_use]
    pub const fn protection_key(&self) -> EntryProtectionKey<'_> {
        let &Self(ref target_value) = self;

        EntryProtectionKey::wrap(target_value)
    }

    /// Resolve a mutable reference to the protection-key field of a 2 MiB leaf.
    #[inline]
    pub const fn protection_key_mut(&mut self) -> EntryProtectionKeyMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryProtectionKeyMut::wrap(target_value)
    }

    /// Determine reserved bits 13 through 20 of a 2 MiB leaf.
    #[inline]
    #[must_use]
    pub const fn reserved_2mib(&self) -> EntryReserved2MiB<'_> {
        let &Self(ref target_value) = self;

        EntryReserved2MiB::wrap(target_value)
    }
}

impl Pte {
    /// Construct an all-zero non-present entry.
    #[inline]
    #[must_use]
    pub const fn empty() -> Self {
        Self(u64::MIN)
    }

    /// Returns the little-endian hardware memory image of this entry.
    #[inline]
    #[must_use]
    pub const fn to_le_bytes(self) -> [u8; 8] {
        let Self(value) = self;

        value.to_le_bytes()
    }

    /// Determine the "Present" (P) flag.
    #[inline]
    #[must_use]
    pub const fn present(&self) -> EntryPresent<'_> {
        let &Self(ref target_value) = self;

        EntryPresent::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Present" (P) flag.
    #[inline]
    pub const fn present_mut(&mut self) -> EntryPresentMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryPresentMut::wrap(target_value)
    }

    /// Determine the "Read/Write" (R/W) flag.
    #[inline]
    #[must_use]
    pub const fn writable(&self) -> EntryWritable<'_> {
        let &Self(ref target_value) = self;

        EntryWritable::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Read/Write" (R/W) flag.
    #[inline]
    pub const fn writable_mut(&mut self) -> EntryWritableMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryWritableMut::wrap(target_value)
    }

    /// Determine the "User/Supervisor" (U/S) flag.
    #[inline]
    #[must_use]
    pub const fn user(&self) -> EntryUser<'_> {
        let &Self(ref target_value) = self;

        EntryUser::wrap(target_value)
    }

    /// Resolve a mutable reference to the "User/Supervisor" (U/S) flag.
    #[inline]
    pub const fn user_mut(&mut self) -> EntryUserMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryUserMut::wrap(target_value)
    }

    /// Determine the page-level write-through (PWT) flag.
    #[inline]
    #[must_use]
    pub const fn write_through(&self) -> EntryWriteThrough<'_> {
        let &Self(ref target_value) = self;

        EntryWriteThrough::wrap(target_value)
    }

    /// Resolve a mutable reference to the page-level write-through (PWT) flag.
    #[inline]
    pub const fn write_through_mut(&mut self) -> EntryWriteThroughMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryWriteThroughMut::wrap(target_value)
    }

    /// Determine the page-level cache-disable (PCD) flag.
    #[inline]
    #[must_use]
    pub const fn cache_disable(&self) -> EntryCacheDisable<'_> {
        let &Self(ref target_value) = self;

        EntryCacheDisable::wrap(target_value)
    }

    /// Resolve a mutable reference to the page-level cache-disable (PCD) flag.
    #[inline]
    pub const fn cache_disable_mut(&mut self) -> EntryCacheDisableMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryCacheDisableMut::wrap(target_value)
    }

    /// Determine the processor-maintained "Accessed" (A) flag.
    #[inline]
    #[must_use]
    pub const fn accessed(&self) -> EntryAccessed<'_> {
        let &Self(ref target_value) = self;

        EntryAccessed::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Accessed" (A) flag.
    #[inline]
    pub const fn accessed_mut(&mut self) -> EntryAccessedMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryAccessedMut::wrap(target_value)
    }

    /// Determine the low software-available field.
    #[inline]
    #[must_use]
    pub const fn available_low(&self) -> EntryAvailableLow<'_> {
        let &Self(ref target_value) = self;

        EntryAvailableLow::wrap(target_value)
    }

    /// Resolve a mutable reference to the low software-available field.
    #[inline]
    pub const fn available_low_mut(&mut self) -> EntryAvailableLowMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryAvailableLowMut::wrap(target_value)
    }

    /// Determine the high software-available field.
    #[inline]
    #[must_use]
    pub const fn available_high(&self) -> EntryAvailableHigh<'_> {
        let &Self(ref target_value) = self;

        EntryAvailableHigh::wrap(target_value)
    }

    /// Resolve a mutable reference to the high software-available field.
    #[inline]
    pub const fn available_high_mut(&mut self) -> EntryAvailableHighMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryAvailableHighMut::wrap(target_value)
    }

    /// Determine the execute-disable (XD/NX) flag.
    #[inline]
    #[must_use]
    pub const fn no_execute(&self) -> EntryNoExecute<'_> {
        let &Self(ref target_value) = self;

        EntryNoExecute::wrap(target_value)
    }

    /// Resolve a mutable reference to the execute-disable (XD/NX) flag.
    #[inline]
    pub const fn no_execute_mut(&mut self) -> EntryNoExecuteMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryNoExecuteMut::wrap(target_value)
    }

    /// Create a present 4 KiB leaf entry.
    ///
    /// Returns `None` when `address` is not 4 KiB-aligned.
    #[inline]
    #[must_use]
    pub const fn page_4kib(address: Pa) -> Option<Self> {
        match address.offset_4kib().const_value() {
            0 => {
                let mut entry = Self::empty();

                entry.present_mut().const_set(State::Set);
                entry.address_mut().const_merge(address.frame_4kib().const_value());

                Some(entry)
            },
            _ => None,
        }
    }

    /// Determine the 4 KiB physical page-number field.
    #[inline]
    #[must_use]
    pub const fn address(&self) -> EntryAddress4KiB<'_> {
        let &Self(ref target_value) = self;

        EntryAddress4KiB::wrap(target_value)
    }

    /// Resolve a mutable reference to the 4 KiB physical page-number field.
    #[inline]
    pub const fn address_mut(&mut self) -> EntryAddress4KiBMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryAddress4KiBMut::wrap(target_value)
    }

    /// Determine the processor-maintained "Dirty" (D) flag.
    #[inline]
    #[must_use]
    pub const fn dirty(&self) -> EntryDirty<'_> {
        let &Self(ref target_value) = self;

        EntryDirty::wrap(target_value)
    }

    /// Resolve a mutable reference to the processor-maintained "Dirty" (D)
    /// flag.
    #[inline]
    pub const fn dirty_mut(&mut self) -> EntryDirtyMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryDirtyMut::wrap(target_value)
    }

    /// Determine the "Global" (G) flag.
    #[inline]
    #[must_use]
    pub const fn global(&self) -> EntryGlobal<'_> {
        let &Self(ref target_value) = self;

        EntryGlobal::wrap(target_value)
    }

    /// Resolve a mutable reference to the "Global" (G) flag.
    #[inline]
    pub const fn global_mut(&mut self) -> EntryGlobalMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryGlobalMut::wrap(target_value)
    }

    /// Determine the 4 KiB PAT selector.
    #[inline]
    #[must_use]
    pub const fn page_attribute_table(&self) -> EntryPat4K<'_> {
        let &Self(ref target_value) = self;

        EntryPat4K::wrap(target_value)
    }

    /// Resolve a mutable reference to the 4 KiB PAT selector.
    #[inline]
    pub const fn page_attribute_table_mut(&mut self) -> EntryPat4KMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryPat4KMut::wrap(target_value)
    }

    /// Determine the protection-key field.
    #[inline]
    #[must_use]
    pub const fn protection_key(&self) -> EntryProtectionKey<'_> {
        let &Self(ref target_value) = self;

        EntryProtectionKey::wrap(target_value)
    }

    /// Resolve a mutable reference to the protection-key field.
    #[inline]
    pub const fn protection_key_mut(&mut self) -> EntryProtectionKeyMut<'_> {
        let &mut Self(ref mut target_value) = self;

        EntryProtectionKeyMut::wrap(target_value)
    }
}

const _: () = {
    assert!(
        mem::size_of::<Pml5e>() == mem::size_of::<u64>(),
        "Pml5e must match the architectural u64 entry width"
    );
    assert!(
        mem::size_of::<Pml4e>() == mem::size_of::<u64>(),
        "Pml4e must match the architectural u64 entry width"
    );
    assert!(
        mem::size_of::<Pdpte>() == mem::size_of::<u64>(),
        "Pdpte must match the architectural u64 entry width"
    );
    assert!(
        mem::size_of::<Pde>() == mem::size_of::<u64>(),
        "Pde must match the architectural u64 entry width"
    );
    assert!(
        mem::size_of::<Pte>() == mem::size_of::<u64>(),
        "Pte must match the architectural u64 entry width"
    );
};

#[cfg(test)]
mod tests {
    use nekor_bitwise::prelude::State;

    use super::{Pde, Pdpte, Pml4e, Pml5e, Pte};
    use crate::x86_64::paging::address::Pa;

    #[test]
    fn empty_entries_are_architecturally_non_present() {
        assert_eq!(Pml5e::empty().present().state(), State::Cleared);
        assert_eq!(Pml4e::empty().present().state(), State::Cleared);
        assert_eq!(Pdpte::empty().present().state(), State::Cleared);
        assert_eq!(Pde::empty().present().state(), State::Cleared);
        assert_eq!(Pte::empty().present().state(), State::Cleared);
    }

    #[test]
    fn table_pointer_constructor_sets_presence_and_frame() {
        let fields = Pa::new(0x2000)
            .and_then(Pml4e::table)
            .map(|entry| (entry.present().state(), entry.address().const_value()));

        assert_eq!(fields, Some((State::Set, 2)));
    }

    #[test]
    fn large_leaf_constructors_select_the_correct_level_shape() {
        let two_mib = Pa::new(0x3fe0_0000).and_then(Pde::page_2mib).map(|entry| {
            (
                entry.page_size().state(),
                entry.page_address().const_value(),
                entry.reserved_2mib().const_value(),
            )
        });
        let one_gib = Pa::new(0x4000_0000).and_then(Pdpte::page_1gib).map(|entry| {
            (
                entry.page_size().state(),
                entry.page_address().const_value(),
                entry.reserved_1gib().const_value(),
            )
        });

        assert_eq!(two_mib, Some((State::Set, 0x1ff, 0)));
        assert_eq!(one_gib, Some((State::Set, 1, 0)));
    }

    #[test]
    fn paging_entry_byte_transport_matches_hardware_layout() {
        let pml4 = Pa::new(0x2000).and_then(Pml4e::table).map(Pml4e::to_le_bytes);
        let pdpt = Pa::new(0x3000).and_then(Pdpte::table).map(Pdpte::to_le_bytes);
        let page = Pa::new(0).and_then(Pde::page_2mib).map(Pde::to_le_bytes);

        assert_eq!(pml4, Some(0x2001_u64.to_le_bytes()));
        assert_eq!(pdpt, Some(0x3001_u64.to_le_bytes()));
        assert_eq!(page, Some(0x81_u64.to_le_bytes()));
    }

    #[test]
    fn leaf_construction_rejects_misaligned_addresses() {
        let entry = Pa::new(0x20_1000).and_then(Pde::page_2mib);

        assert_eq!(entry, None);
    }
}

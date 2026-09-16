//! Exact page-table page layouts for x86-64 four-level and five-level paging.
//!
//! Each table page contains exactly 512 eight-byte entries and is aligned to
//! the 4096-byte boundary required by the architecture. The Rust arrays are the
//! hardware tables themselves. There is no `Option`, sparse side structure, or
//! later materialization step.
//!
//! The five index types mirror the hierarchy selectors carried by a
//! [`La`]. PML5 is
//! consumed only by LA57. Keeping every selector distinct prevents a selector
//! from one hierarchy level being used to index a different table type.

use core::{
    mem,
    ops::{Index, IndexMut},
};

use super::{
    address::La,
    entry::{Pde, Pdpte, Pml4e, Pml5e, Pte},
};

/// Number of entries in every x86-64 paging-structure page.
pub const ENTRY_COUNT: usize = 512;

/// Largest value representable by a nine-bit hierarchy selector.
const LAST_ENTRY_INDEX: u16 = 511;

/// A nine-bit PML5 index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Pml5Index(
    // NOTE(invariant): The value is in the inclusive range 0 through 511.
    u16,
);

impl Pml5Index {
    /// Create a PML5 selector for an entry in the five-level table.
    ///
    /// Returns `None` when `value` does not fit the nine-bit table index.
    #[inline]
    #[must_use]
    pub const fn new(value: u16) -> Option<Self> {
        match value {
            0..=LAST_ENTRY_INDEX => Some(Self(value)),
            _ => None,
        }
    }

    /// Derive the PML5 selector from a linear address.
    #[inline]
    #[must_use]
    pub const fn from_address(address: &La) -> Self {
        Self(address.pml5_index().const_value())
    }

    /// Convert this selector into its array index.
    #[inline]
    const fn array_index(self) -> usize {
        let Self(value) = self;

        value as usize
    }
}

/// A nine-bit PML4 index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Pml4Index(
    // NOTE(invariant): The value is in the inclusive range 0 through 511.
    u16,
);

impl Pml4Index {
    /// Create a PML4 selector for an entry in a paging table.
    ///
    /// Returns `None` when `value` does not fit the nine-bit table index.
    #[inline]
    #[must_use]
    pub const fn new(value: u16) -> Option<Self> {
        match value {
            0..=LAST_ENTRY_INDEX => Some(Self(value)),
            _ => None,
        }
    }

    /// Derive the PML4 selector from a linear address.
    #[inline]
    #[must_use]
    pub const fn from_address(address: &La) -> Self {
        Self(address.pml4_index().const_value())
    }

    /// Convert this selector into its array index.
    #[inline]
    const fn array_index(self) -> usize {
        let Self(value) = self;

        value as usize
    }
}

/// A nine-bit PDPT index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct PdptIndex(
    // NOTE(invariant): The value is in the inclusive range 0 through 511.
    u16,
);

impl PdptIndex {
    /// Create a PDPT selector for an entry in a paging table.
    ///
    /// Returns `None` when `value` does not fit the nine-bit table index.
    #[inline]
    #[must_use]
    pub const fn new(value: u16) -> Option<Self> {
        match value {
            0..=LAST_ENTRY_INDEX => Some(Self(value)),
            _ => None,
        }
    }

    /// Derive the PDPT selector from a linear address.
    #[inline]
    #[must_use]
    pub const fn from_address(address: &La) -> Self {
        Self(address.pdpt_index().const_value())
    }

    /// Convert this selector into its array index.
    #[inline]
    const fn array_index(self) -> usize {
        let Self(value) = self;

        value as usize
    }
}

/// A nine-bit page-directory index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct PdIndex(
    // NOTE(invariant): The value is in the inclusive range 0 through 511.
    u16,
);

impl PdIndex {
    /// Create a page-directory selector for an entry in a paging table.
    ///
    /// Returns `None` when `value` does not fit the nine-bit table index.
    #[inline]
    #[must_use]
    pub const fn new(value: u16) -> Option<Self> {
        match value {
            0..=LAST_ENTRY_INDEX => Some(Self(value)),
            _ => None,
        }
    }

    /// Derive the page-directory selector from a linear address.
    #[inline]
    #[must_use]
    pub const fn from_address(address: &La) -> Self {
        Self(address.page_directory_index().const_value())
    }

    /// Convert this selector into its array index.
    #[inline]
    const fn array_index(self) -> usize {
        let Self(value) = self;

        value as usize
    }
}

/// A nine-bit page-table index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct PtIndex(
    // NOTE(invariant): The value is in the inclusive range 0 through 511.
    u16,
);

impl PtIndex {
    /// Create a page-table selector for an entry in a paging table.
    ///
    /// Returns `None` when `value` does not fit the nine-bit table index.
    #[inline]
    #[must_use]
    pub const fn new(value: u16) -> Option<Self> {
        match value {
            0..=LAST_ENTRY_INDEX => Some(Self(value)),
            _ => None,
        }
    }

    /// Derive the page-table selector from a linear address.
    #[inline]
    #[must_use]
    pub const fn from_address(address: &La) -> Self {
        Self(address.page_table_index().const_value())
    }

    /// Convert this selector into its array index.
    #[inline]
    const fn array_index(self) -> usize {
        let Self(value) = self;

        value as usize
    }
}

/// One exact PML5 page.
///
/// This is the top-level page-map level-five table used by LA57. Its
/// representation is one 4096-byte page containing 512 contiguous [`Pml5e`]
/// hardware entries.
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(C, align(4096))]
pub struct Pml5([Pml5e; ENTRY_COUNT]);

impl Pml5 {
    /// Construct a table page containing 512 non-present zeroed entries.
    #[inline]
    #[must_use]
    pub const fn empty() -> Self {
        Self([Pml5e::empty(); ENTRY_COUNT])
    }

    /// Borrow every typed entry in hardware index order.
    #[inline]
    #[must_use]
    pub const fn entries(&self) -> &[Pml5e; ENTRY_COUNT] {
        let &Self(ref entries) = self;

        entries
    }

    /// Mutably borrow every typed entry in hardware index order.
    #[inline]
    pub const fn entries_mut(&mut self) -> &mut [Pml5e; ENTRY_COUNT] {
        let &mut Self(ref mut entries) = self;

        entries
    }
}

impl Index<Pml5Index> for Pml5 {
    type Output = Pml5e;

    #[inline]
    #[expect(
        clippy::indexing_slicing,
        reason = "the typed page-table index is bounded to ENTRY_COUNT by construction"
    )]
    fn index(&self, index: Pml5Index) -> &Self::Output {
        let &Self(ref entries) = self;

        &entries[index.array_index()]
    }
}

impl IndexMut<Pml5Index> for Pml5 {
    #[inline]
    #[expect(
        clippy::indexing_slicing,
        reason = "the typed page-table index is bounded to ENTRY_COUNT by construction"
    )]
    fn index_mut(&mut self, index: Pml5Index) -> &mut Self::Output {
        let &mut Self(ref mut entries) = self;

        &mut entries[index.array_index()]
    }
}

/// One exact PML4 page.
///
/// This is the LA48 root table and the second table level under LA57. Its
/// representation is one
/// 4096-byte page containing 512 contiguous [`Pml4e`] hardware entries.
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(C, align(4096))]
pub struct Pml4([Pml4e; ENTRY_COUNT]);

impl Pml4 {
    /// Construct a table page containing 512 non-present zeroed entries.
    #[inline]
    #[must_use]
    pub const fn empty() -> Self {
        Self([Pml4e::empty(); ENTRY_COUNT])
    }

    /// Borrow every typed entry in hardware index order.
    #[inline]
    #[must_use]
    pub const fn entries(&self) -> &[Pml4e; ENTRY_COUNT] {
        let &Self(ref entries) = self;

        entries
    }

    /// Mutably borrow every typed entry in hardware index order.
    #[inline]
    pub const fn entries_mut(&mut self) -> &mut [Pml4e; ENTRY_COUNT] {
        let &mut Self(ref mut entries) = self;

        entries
    }
}

impl Index<Pml4Index> for Pml4 {
    type Output = Pml4e;

    #[inline]
    #[expect(
        clippy::indexing_slicing,
        reason = "the typed page-table index is bounded to ENTRY_COUNT by construction"
    )]
    fn index(&self, index: Pml4Index) -> &Self::Output {
        let &Self(ref entries) = self;

        &entries[index.array_index()]
    }
}

impl IndexMut<Pml4Index> for Pml4 {
    #[inline]
    #[expect(
        clippy::indexing_slicing,
        reason = "the typed page-table index is bounded to ENTRY_COUNT by construction"
    )]
    fn index_mut(&mut self, index: Pml4Index) -> &mut Self::Output {
        let &mut Self(ref mut entries) = self;

        &mut entries[index.array_index()]
    }
}

/// One exact page-directory-pointer table page.
///
/// This is the second-level page-directory-pointer table. Its representation is
/// one 4096-byte page containing 512 contiguous [`Pdpte`] hardware entries.
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(C, align(4096))]
pub struct Pdpt([Pdpte; ENTRY_COUNT]);

impl Pdpt {
    /// Construct a table page containing 512 non-present zeroed entries.
    #[inline]
    #[must_use]
    pub const fn empty() -> Self {
        Self([Pdpte::empty(); ENTRY_COUNT])
    }

    /// Borrow every typed entry in hardware index order.
    #[inline]
    #[must_use]
    pub const fn entries(&self) -> &[Pdpte; ENTRY_COUNT] {
        let &Self(ref entries) = self;

        entries
    }

    /// Mutably borrow every typed entry in hardware index order.
    #[inline]
    pub const fn entries_mut(&mut self) -> &mut [Pdpte; ENTRY_COUNT] {
        let &mut Self(ref mut entries) = self;

        entries
    }
}

impl Index<PdptIndex> for Pdpt {
    type Output = Pdpte;

    #[inline]
    #[expect(
        clippy::indexing_slicing,
        reason = "the typed page-table index is bounded to ENTRY_COUNT by construction"
    )]
    fn index(&self, index: PdptIndex) -> &Self::Output {
        let &Self(ref entries) = self;

        &entries[index.array_index()]
    }
}

impl IndexMut<PdptIndex> for Pdpt {
    #[inline]
    #[expect(
        clippy::indexing_slicing,
        reason = "the typed page-table index is bounded to ENTRY_COUNT by construction"
    )]
    fn index_mut(&mut self, index: PdptIndex) -> &mut Self::Output {
        let &mut Self(ref mut entries) = self;

        &mut entries[index.array_index()]
    }
}

/// One exact page directory page.
///
/// This is the third-level page directory. Its representation is one 4096-byte
/// page containing 512 contiguous [`Pde`] hardware entries.
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(C, align(4096))]
pub struct PageDirectory([Pde; ENTRY_COUNT]);

impl PageDirectory {
    /// Construct a table page containing 512 non-present zeroed entries.
    #[inline]
    #[must_use]
    pub const fn empty() -> Self {
        Self([Pde::empty(); ENTRY_COUNT])
    }

    /// Borrow every typed entry in hardware index order.
    #[inline]
    #[must_use]
    pub const fn entries(&self) -> &[Pde; ENTRY_COUNT] {
        let &Self(ref entries) = self;

        entries
    }

    /// Mutably borrow every typed entry in hardware index order.
    #[inline]
    pub const fn entries_mut(&mut self) -> &mut [Pde; ENTRY_COUNT] {
        let &mut Self(ref mut entries) = self;

        entries
    }
}

impl Index<PdIndex> for PageDirectory {
    type Output = Pde;

    #[inline]
    #[expect(
        clippy::indexing_slicing,
        reason = "the typed page-table index is bounded to ENTRY_COUNT by construction"
    )]
    fn index(&self, index: PdIndex) -> &Self::Output {
        let &Self(ref entries) = self;

        &entries[index.array_index()]
    }
}

impl IndexMut<PdIndex> for PageDirectory {
    #[inline]
    #[expect(
        clippy::indexing_slicing,
        reason = "the typed page-table index is bounded to ENTRY_COUNT by construction"
    )]
    fn index_mut(&mut self, index: PdIndex) -> &mut Self::Output {
        let &mut Self(ref mut entries) = self;

        &mut entries[index.array_index()]
    }
}

/// One exact page table page.
///
/// This is the final-level page table. Its representation is one 4096-byte page
/// containing 512 contiguous [`Pte`] hardware entries.
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(C, align(4096))]
pub struct PageTable([Pte; ENTRY_COUNT]);

impl PageTable {
    /// Construct a table page containing 512 non-present zeroed entries.
    #[inline]
    #[must_use]
    pub const fn empty() -> Self {
        Self([Pte::empty(); ENTRY_COUNT])
    }

    /// Borrow every typed entry in hardware index order.
    #[inline]
    #[must_use]
    pub const fn entries(&self) -> &[Pte; ENTRY_COUNT] {
        let &Self(ref entries) = self;

        entries
    }

    /// Mutably borrow every typed entry in hardware index order.
    #[inline]
    pub const fn entries_mut(&mut self) -> &mut [Pte; ENTRY_COUNT] {
        let &mut Self(ref mut entries) = self;

        entries
    }
}

impl Index<PtIndex> for PageTable {
    type Output = Pte;

    #[inline]
    #[expect(
        clippy::indexing_slicing,
        reason = "the typed page-table index is bounded to ENTRY_COUNT by construction"
    )]
    fn index(&self, index: PtIndex) -> &Self::Output {
        let &Self(ref entries) = self;

        &entries[index.array_index()]
    }
}

impl IndexMut<PtIndex> for PageTable {
    #[inline]
    #[expect(
        clippy::indexing_slicing,
        reason = "the typed page-table index is bounded to ENTRY_COUNT by construction"
    )]
    fn index_mut(&mut self, index: PtIndex) -> &mut Self::Output {
        let &mut Self(ref mut entries) = self;

        &mut entries[index.array_index()]
    }
}

const _: () = {
    assert!(mem::size_of::<Pml5>() == 4096, "PML5 must occupy one page");
    assert!(mem::size_of::<Pml4>() == 4096, "PML4 must occupy one page");
    assert!(mem::size_of::<Pdpt>() == 4096, "PDPT must occupy one page");
    assert!(
        mem::size_of::<PageDirectory>() == 4096,
        "page directory must occupy one page"
    );
    assert!(mem::size_of::<PageTable>() == 4096, "page table must occupy one page");

    assert!(mem::align_of::<Pml5>() == 4096, "PML5 must be page aligned");
    assert!(mem::align_of::<Pml4>() == 4096, "PML4 must be page aligned");
    assert!(mem::align_of::<Pdpt>() == 4096, "PDPT must be page aligned");
    assert!(
        mem::align_of::<PageDirectory>() == 4096,
        "page directory must be page aligned"
    );
    assert!(mem::align_of::<PageTable>() == 4096, "page table must be page aligned");
};

#[cfg(test)]
mod tests {
    use nekor_bitwise::prelude::State;

    use super::{Pml4, Pml4Index, Pml5, Pml5Index};
    use crate::x86_64::paging::{
        address::{La, La57, Pa},
        entry::{Pml4e, Pml5e},
    };

    #[test]
    fn linear_address_indices_select_the_matching_hardware_levels() {
        let state = La::new::<La57>(0x00f2_7abc_def1_2345).map(|address| {
            let pml5_index = Pml5Index::from_address(&address);
            let pml4_index = Pml4Index::from_address(&address);
            let pml5 = Pa::new(0x4000).and_then(Pml5e::table).map(|entry| {
                let mut table = Pml5::empty();

                table[pml5_index] = entry;

                (
                    table[pml5_index].present().state(),
                    table[pml5_index].address().const_value(),
                )
            });
            let pml4 = Pa::new(0x8000).and_then(Pml4e::table).map(|entry| {
                let mut table = Pml4::empty();

                table[pml4_index] = entry;

                (
                    table[pml4_index].present().state(),
                    table[pml4_index].address().const_value(),
                )
            });

            (pml5, pml4)
        });

        assert_eq!(state, Some((Some((State::Set, 4)), Some((State::Set, 8)))));
    }
}

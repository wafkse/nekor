//! x86-64 long-mode paging architecture.
//!
//! This module models both architectural hierarchy depths available to long
//! mode. LA48 uses four paging-structure levels rooted at PML4. LA57 adds a
//! PML5 root above the otherwise identical lower four levels.
//!
//! # Representation
//!
//! Each hardware paging entry is an exact transparent `u64`. Each paging-
//! structure page is exactly 512 level-specific entries with 4096-byte size and
//! alignment. There is no secondary encoded form or materialization step.
//!
//! # Linear address decomposition
//!
//! The lower four selectors are shared by LA48 and LA57.
//!
//! | Bits | Meaning |
//! |---|---|
//! | 56..48 | PML5 index under LA57, sign extension under LA48 |
//! | 47..39 | PML4 index |
//! | 38..30 | PDPT index |
//! | 29..21 | Page-directory index |
//! | 20..12 | Page-table index |
//! | 11..0 | 4 KiB page offset |
//!
//! LA48 requires bits 63 through 48 to sign-extend bit 47. LA57 instead uses
//! bits 48 through 56 for PML5 selection and requires bits 63 through 57 to
//! sign-extend bit 56. [`address::La`] therefore does not claim
//! canonicality by itself. [`address::La48`] and [`address::La57`] select the
//! mode at the [`address::La::new`] call site.
//!
//! # Hierarchy
//!
//! An LA48 4 KiB mapping follows this walk.
//!
//! ```text
//! CR3 -> PML4E -> PDPTE -> PDE -> PTE -> 4 KiB page
//! ```
//!
//! An LA57 4 KiB mapping adds PML5 above the same lower hierarchy.
//!
//! ```text
//! CR3 -> PML5E -> PML4E -> PDPTE -> PDE -> PTE -> 4 KiB page
//! ```
//!
//! PDPT and page-directory entries may terminate either walk early with 1 GiB
//! and 2 MiB leaves respectively. Their page-size flag changes the meaning of
//! several lower address bits, including the PAT position.
//!
//! # Presence
//!
//! Presence is architectural bit zero inside every entry. A non-present entry
//! remains a hardware entry and may retain software-owned bits. Nekor
//! does not reinterpret absence as a Rust enum niche.
//!
//! # Physical address width
//!
//! The layouts model address fields through bit 51. A concrete processor may
//! advertise a narrower implemented physical-address width. Enforcing that
//! runtime limit is separate from representing the architectural layout.
//!
//! # Safety boundary
//!
//! These types describe architectural memory layout only. They do not allocate
//! memory, translate Rust references to physical addresses, install CR3, choose
//! LA48 versus LA57, or promise that a represented mapping is valid for a
//! particular processor. Those operations belong to the owner of processor and
//! page-table state.

/// Architectural linear and physical address representations.
pub mod address;

pub use address::{La, La48, La48Page, La57, LaMode, Pa};

/// Level-specific hardware page-table entry representations.
pub mod entry;

/// Exact 4 KiB page-table page representations and hierarchy indices.
pub mod table;

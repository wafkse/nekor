//! Paging address representations for x86-64 translation.
//!
//! [`Pa`] represents the architectural address domain carried by paging-entry
//! address fields. Nekor currently models the architectural maximum through bit
//! 51. A concrete processor may implement fewer physical address bits and must
//! impose that narrower limit separately.
//!
//! [`La`] preserves a 64-bit linear address. Construction accepts
//! only values canonical for the selected mode, but that mode is not retained
//! by the type.
//! [`La48`] and [`La57`] are compile-time mode markers used by [`La::new`]
//! without depending on a control-register representation.
//!
//! # Intel SDM references
//!
//! Intel SDM Vol. 3A, Section 4.5 defines 48-bit and 57-bit paging
//! canonicality. Section 5.1.4 defines MAXPHYADDR. Sections 5.5 and 5.5.4
//! define the linear-address fields consumed by four-level and five-level
//! translation.

use nekor_bitwise::prelude::{Bit, Field, State};

/// Bits 63 through 52 above the paging-entry physical-address envelope.
///
/// See Intel SDM Vol. 3A, Sections 5.1.4 and 5.5.4.
pub type PaUpper<'value> = Field<'value, 52, 63, u64>;

/// Offset inside an aligned 4 KiB physical page.
pub type PaOffset4K<'value> = Field<'value, 0, 11, u64>;

/// Offset inside an aligned 2 MiB physical page.
pub type PaOffset2M<'value> = Field<'value, 0, 20, u64>;

/// Offset inside an aligned 1 GiB physical page.
pub type PaOffset1G<'value> = Field<'value, 0, 29, u64>;

/// Physical page-number field used by 4 KiB mappings and table pointers.
pub type PaFrame4K<'value> = Field<'value, 12, 51, u64>;

/// Physical page-number field used by 2 MiB leaf mappings.
pub type PaFrame2M<'value> = Field<'value, 21, 51, u64>;

/// Physical page-number field used by 1 GiB leaf mappings.
pub type PaFrame1G<'value> = Field<'value, 30, 51, u64>;

/// PML5 selector in bits 56 through 48 under LA57.
///
/// See Intel SDM Vol. 3A, Section 5.5.4.
pub type LaPml5<'value> = Field<'value, 48, 56, u64>;

/// PML4 selector in bits 47 through 39 under LA48 and LA57.
///
/// See Intel SDM Vol. 3A, Section 5.5.4.
pub type LaPml4<'value> = Field<'value, 39, 47, u64>;

/// PDPT selector in bits 38 through 30 under LA48 and LA57.
///
/// See Intel SDM Vol. 3A, Section 5.5.4.
pub type LaPdpt<'value> = Field<'value, 30, 38, u64>;

/// Page-directory selector in bits 29 through 21 under LA48 and LA57.
///
/// See Intel SDM Vol. 3A, Section 5.5.4.
pub type LaPd<'value> = Field<'value, 21, 29, u64>;

/// Page-table selector in bits 20 through 12 under LA48 and LA57.
///
/// See Intel SDM Vol. 3A, Section 5.5.4.
pub type LaPt<'value> = Field<'value, 12, 20, u64>;

/// Offset inside the selected 4 KiB page.
pub type LaOffset4K<'value> = Field<'value, 0, 11, u64>;

/// Sign bit for LA48 canonicality.
///
/// See Intel SDM Vol. 3A, Section 4.5.
pub type La48Sign<'value> = Bit<'value, u64, 47>;

/// Sign bit for LA57 canonicality.
///
/// See Intel SDM Vol. 3A, Section 4.5.
pub type La57Sign<'value> = Bit<'value, u64, 56>;

/// LA48 untranslated upper field in bits 63 through 48.
///
/// See Intel SDM Vol. 3A, Section 4.5.
pub type La48Upper<'value> = Field<'value, 48, 63, u64>;

/// LA57 untranslated upper field in bits 63 through 57.
///
/// See Intel SDM Vol. 3A, Section 4.5.
pub type La57Upper<'value> = Field<'value, 57, 63, u64>;

/// An x86-64 physical address representable by the paging structures.
///
/// This type does not describe ownership, host memory, guest memory, or the
/// implemented MAXPHYADDR of a concrete processor. It only guarantees that the
/// value fits the 52-bit address envelope used by these architectural layouts.
// NOTE(invariant): The stored value always fits the architectural 52-bit
// paging-entry physical-address envelope. `Pa::new` rejects any value with
// bits 52 through 63 set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Pa(u64);

impl Pa {
    /// Create a physical address in the paging-entry address domain.
    ///
    /// Returns `None` when the value sets any bit above the architectural
    /// 52-bit physical-address envelope.
    #[inline]
    #[must_use]
    pub const fn new(value: u64) -> Option<Self> {
        if let 0 = PaUpper::wrap(&value).const_value() {
            Some(Self(value))
        } else {
            None
        }
    }

    /// Determine the offset inside an aligned 4 KiB page.
    #[inline]
    #[must_use]
    pub const fn offset_4kib(&self) -> PaOffset4K<'_> {
        let &Self(ref target_value) = self;

        PaOffset4K::wrap(target_value)
    }

    /// Determine the offset inside an aligned 2 MiB page.
    #[inline]
    #[must_use]
    pub const fn offset_2mib(&self) -> PaOffset2M<'_> {
        let &Self(ref target_value) = self;

        PaOffset2M::wrap(target_value)
    }

    /// Determine the offset inside an aligned 1 GiB page.
    #[inline]
    #[must_use]
    pub const fn offset_1gib(&self) -> PaOffset1G<'_> {
        let &Self(ref target_value) = self;

        PaOffset1G::wrap(target_value)
    }

    /// Determine the physical page number used by a 4 KiB mapping or table
    /// pointer.
    #[inline]
    #[must_use]
    pub const fn frame_4kib(&self) -> PaFrame4K<'_> {
        let &Self(ref target_value) = self;

        PaFrame4K::wrap(target_value)
    }

    /// Determine the physical page number used by a 2 MiB leaf mapping.
    #[inline]
    #[must_use]
    pub const fn frame_2mib(&self) -> PaFrame2M<'_> {
        let &Self(ref target_value) = self;

        PaFrame2M::wrap(target_value)
    }

    /// Determine the physical page number used by a 1 GiB leaf mapping.
    #[inline]
    #[must_use]
    pub const fn frame_1gib(&self) -> PaFrame1G<'_> {
        let &Self(ref target_value) = self;

        PaFrame1G::wrap(target_value)
    }
}
/// A compile-time linear-address translation mode.
///
/// Implementations are sealed to the architectural modes represented by this
/// module. The marker is selected by the owner of processor state rather than
/// inferred from a control-register value.
pub trait LaMode: private::Sealed {
    /// Number of low linear-address bits translated by this mode.
    #[doc(hidden)]
    const WIDTH: u8;
}

/// Four-level translation with 48-bit paging canonicality.
///
/// Intel SDM Vol. 3A, Section 4.5 defines 48-bit paging canonicality. Section
/// 5.5 describes four-level paging when CR4.LA57 is clear.
pub enum La48 {}

/// Five-level translation with 57-bit paging canonicality.
///
/// Intel SDM Vol. 3A, Section 4.5 defines 57-bit paging canonicality. Section
/// 5.5 describes five-level paging when CR4.LA57 is set.
pub enum La57 {}

impl private::Sealed for La48 {}

impl LaMode for La48 {
    const WIDTH: u8 = 48;
}

impl private::Sealed for La57 {}

impl LaMode for La57 {
    const WIDTH: u8 = 57;
}

/// Sealing implementation for supported linear-address modes.
mod private {
    /// Seal for supported linear-address mode markers.
    pub trait Sealed {}
}

/// An x86-64 linear address value.
///
/// The type preserves all 64 bits because canonicality depends on the active
/// translation mode. LA48 requires bits 63 through 48 to sign-extend bit 47.
/// LA57 instead consumes bits 48 through 56 as the PML5 selector and requires
/// bits 63 through 57 to sign-extend bit 56.
// NOTE(invariant): The stored value is canonical for at least one supported
// x86-64 paging mode. `La::new` checks the selected mode before construction
// and no mutation API can replace the representation afterward.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct La(u64);

impl La {
    /// Return the linear-address representation.
    #[inline]
    #[must_use]
    pub const fn bits(self) -> u64 {
        let Self(value) = self;

        value
    }

    /// Create a linear address when `value` is canonical for `M`.
    ///
    /// The selected mode is used to classify the upper bits and is not stored
    /// in the resulting address. Returns `None` when the upper bits do not
    /// sign-extend the mode's address bit. Intel SDM Vol. 3A, Section 4.5
    /// defines the LA48 and LA57 canonicality rules.
    #[inline]
    #[must_use]
    pub const fn new<M>(value: u64) -> Option<Self>
    where
        M: LaMode,
    {
        Self::canonical::<M>(value)
    }

    /// Determine whether this address is paging-canonical for `M`.
    ///
    /// Intel SDM Vol. 3A, Section 4.5 defines paging canonicality relative to
    /// the active four-level or five-level paging mode.
    #[inline]
    #[must_use]
    pub const fn is_canonical<M>(self) -> bool
    where
        M: LaMode,
    {
        let Self(value) = self;

        Self::canonical::<M>(value).is_some()
    }

    /// Create the canonical linear-address representation for `M`, if possible.
    #[inline]
    const fn canonical<M>(target_address: u64) -> Option<Self>
    where
        M: LaMode,
    {
        match M::WIDTH {
            <La48 as LaMode>::WIDTH => {
                let target_value = La48Upper::wrap(&target_address).const_value();

                if let State::Cleared = La48Sign::wrap(&target_address).const_state() {
                    if let 0b0000_0000_0000_0000 = target_value {
                        Some(Self(target_address))
                    } else {
                        None
                    }
                } else {
                    if let 0b1111_1111_1111_1111 = target_value {
                        Some(Self(target_address))
                    } else {
                        None
                    }
                }
            },
            <La57 as LaMode>::WIDTH => {
                let target_value = La57Upper::wrap(&target_address).const_value();

                if let State::Cleared = La57Sign::wrap(&target_address).const_state() {
                    if let 0b0000_0000 = target_value {
                        Some(Self(target_address))
                    } else {
                        None
                    }
                } else {
                    if let 0b0111_1111 = target_value {
                        Some(Self(target_address))
                    } else {
                        None
                    }
                }
            },
            _ => unreachable!(),
        }
    }

    /// Determine the PML5 selector carried in bits 56 through 48.
    ///
    /// This field participates in translation only when LA57 is active. Under
    /// LA48 the same bits belong to the canonical sign extension.
    #[inline]
    #[must_use]
    pub const fn pml5_index(&self) -> LaPml5<'_> {
        let &Self(ref target_value) = self;

        LaPml5::wrap(target_value)
    }

    /// Determine the PML4 selector carried in bits 47 through 39.
    #[inline]
    #[must_use]
    pub const fn pml4_index(&self) -> LaPml4<'_> {
        let &Self(ref target_value) = self;

        LaPml4::wrap(target_value)
    }

    /// Determine the PDPT selector carried in bits 38 through 30.
    #[inline]
    #[must_use]
    pub const fn pdpt_index(&self) -> LaPdpt<'_> {
        let &Self(ref target_value) = self;

        LaPdpt::wrap(target_value)
    }

    /// Determine the page-directory selector carried in bits 29 through 21.
    #[inline]
    #[must_use]
    pub const fn page_directory_index(&self) -> LaPd<'_> {
        let &Self(ref target_value) = self;

        LaPd::wrap(target_value)
    }

    /// Determine the page-table selector carried in bits 20 through 12.
    #[inline]
    #[must_use]
    pub const fn page_table_index(&self) -> LaPt<'_> {
        let &Self(ref target_value) = self;

        LaPt::wrap(target_value)
    }

    /// Determine the offset carried in bits 11 through 0.
    #[inline]
    #[must_use]
    pub const fn offset_4kib(&self) -> LaOffset4K<'_> {
        let &Self(ref target_value) = self;

        LaOffset4K::wrap(target_value)
    }

    /// Determine the bits that must sign-extend bit 47 under LA48.
    #[inline]
    #[must_use]
    pub const fn la48_upper(&self) -> La48Upper<'_> {
        let &Self(ref target_value) = self;

        La48Upper::wrap(target_value)
    }

    /// Determine the bits that must sign-extend bit 56 under LA57.
    #[inline]
    #[must_use]
    pub const fn la57_upper(&self) -> La57Upper<'_> {
        let &Self(ref target_value) = self;

        La57Upper::wrap(target_value)
    }
}
/// A page-aligned linear address in the LA48 canonical domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The private linear address is canonical under LA48 and has a zero 4 KiB page
// offset. It therefore identifies exactly one base-page virtual position in four-level paging.
pub struct La48Page(La);

impl La48Page {
    /// Create an LA48 page base from a canonical, 4 KiB-aligned address.
    ///
    /// Returns `None` when `address` is non-canonical under LA48 or has a
    /// nonzero page offset.
    #[inline]
    #[must_use]
    pub const fn new(address: La) -> Option<Self> {
        let is_canonical = address.is_canonical::<La48>();
        let page_aligned = address.offset_4kib().const_value() == 0;

        if is_canonical && page_aligned {
            Some(Self(address))
        } else {
            None
        }
    }

    /// Create an LA48 page base from its raw address image.
    ///
    /// Returns `None` when `bits` is non-canonical under LA48 or is not
    /// 4 KiB-aligned.
    #[inline]
    #[must_use]
    pub const fn raw(target_bits: u64) -> Option<Self> {
        match La::new::<La48>(target_bits) {
            Some(target_address) => Self::new(target_address),
            None => None,
        }
    }

    /// Return the represented linear page base.
    #[inline]
    #[must_use]
    pub const fn address(self) -> La {
        let Self(address) = self;

        address
    }

    /// Return the linear page-base image.
    #[inline]
    #[must_use]
    pub const fn bits(self) -> u64 {
        let Self(address) = self;

        address.bits()
    }
}

#[cfg(test)]
mod tests {
    use super::{La, La48, La48Page, La57, Pa};

    #[test]
    fn physical_addresses_respect_the_architectural_entry_envelope() {
        let maximum = Pa::new((1_u64 << 52) - 1);
        let outside = Pa::new(1_u64 << 52);

        assert!(maximum.is_some());
        assert!(outside.is_none());
    }

    #[test]
    fn construction_validates_the_selected_linear_address_mode() {
        let la48 = La::new::<La48>(0xffff_8abc_def1_2345);
        let la57_only = La::new::<La57>(0x00ff_8abc_def1_2345);
        let invalid_la57 = La::new::<La57>(0x01ff_8abc_def1_2345);
        let la57_under_la48 = la57_only.map(La::is_canonical::<La48>);

        assert!(la48.is_some());
        assert!(la57_only.is_some());
        assert!(invalid_la57.is_none());
        assert_eq!(la57_under_la48, Some(false));
    }

    #[test]
    fn la48_page_rejects_unaligned_and_la57_only_addresses() {
        let aligned = La48Page::raw(0xffff_ffff_8000_1000);
        let unaligned = La48Page::raw(0xffff_ffff_8000_1001);
        let la57_only = La::new::<La57>(0x00ff_8000_0000_0000).and_then(La48Page::new);

        assert_eq!(aligned.map(La48Page::bits), Some(0xffff_ffff_8000_1000));
        assert!(unaligned.is_none());
        assert!(la57_only.is_none());
    }

    #[test]
    fn linear_address_fields_cover_the_five_level_walk() {
        let fields = La::new::<La57>(0x00f2_7abc_def1_2345).map(|address| {
            (
                address.pml5_index().const_value(),
                address.pml4_index().const_value(),
                address.pdpt_index().const_value(),
                address.page_directory_index().const_value(),
                address.page_table_index().const_value(),
                address.offset_4kib().const_value(),
            )
        });

        assert_eq!(fields, Some((0xf2, 0xf5, 0xf3, 0xf7, 0x112, 0x345)));
    }
}

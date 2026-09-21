//! In-memory descriptor tables.

use core::num::NonZero;

use nekor_bitwise::prelude::{Bit, Counterpart, Field, State};
use zerocopy::{Immutable, IntoBytes};

use crate::x86::{
    address::Address,
    mode::{Mode, Native},
    privilege::PrivilegeLevel,
};

/// Sealing implementation for architectural descriptor-table markers.
mod private {
    /// A trait to act as a seal supertrait to [`DescriptorTable`].
    ///
    /// [`DescriptorTable`]: super::DescriptorTable
    pub trait Sealed {}
}

/// An enumeration of all distinct *Descriptor Table*s.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum DescriptorTableType {
    /// A *Interrupt Descriptor Table* (*IDT*).
    Idt,

    /// A *Global Descriptor Table* (*GDT*).
    Gdt,

    /// A *Local Descriptor Table* (*LDT*).
    Ldt,
}

/// A marker trait that encodes a descriptor table.
pub trait DescriptorTable: private::Sealed {
    /// The type of this descriptor table.
    const TABLE_TYPE: DescriptorTableType;
}

/// A *Interrupt Descriptor Table* (*IDT*) marker type.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum Idt {}

/// A *Global Descriptor Table* (*GDT*) marker type.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum Gdt {}

/// A *Local Descriptor Table* (*LDT*) marker type.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum Ldt {}

/// Implement the [`DescriptorTable`] trait for a target marker type.
macro_rules! table {
    (
        $(
            $target_name:ident
        ),+
    ) => {
        $(
            impl private::Sealed for $target_name {}

            impl DescriptorTable for $target_name {
                const TABLE_TYPE: DescriptorTableType = DescriptorTableType::$target_name;
            }
        )+
    };
}

table!(Idt, Gdt, Ldt);

/// A [`Mode`]-agnostic pointer to a [`DescriptorTable`].
#[repr(C, packed)]
pub struct DescriptorTablePointer<T, B = Native>
where
    T: DescriptorTable,
    B: Mode,
{
    /// The size of this descriptor table.
    ///
    /// The value stored here is 1-biased, actual size is determined by the
    /// expression `table size + 1`.
    pub table_size: u16,

    /// The mode-dependent address of the target descriptor table.
    pub table_address: Address<T, B>,
}

/// A `13`-bit index into a *Global Descriptor Table* or a *Local Descriptor
/// Table*.
///
/// This can be for either a *Segment* or *System* selector. For
/// multi-descriptor structures, the first in line should be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The nonzero index never names the mandated null descriptor and never exceeds
// the thirteen-bit descriptor-table index domain.
pub struct DescriptorIndex(NonZero<u16>);

impl DescriptorIndex {
    /// The bits used as the underlying index.
    pub const BITS: u32 = 13;
    /// The maximum value for a [`DescriptorIndex`].
    pub const MAX: Self = Self(match NonZero::new((1 << Self::BITS) - 1) {
        Some(target_value) => target_value,
        None => unreachable!(),
    });
    /// The minimum value for a [`DescriptorIndex`].
    ///
    /// This is the first non-null entry in a descriptor table. The null
    /// descriptor is index zero and is intentionally not representable.
    pub const MIN: Self = Self(NonZero::<u16>::MIN);
}

impl DescriptorIndex {
    /// Create a descriptor index from its raw selector index.
    ///
    /// Returns `None` for the null index or for a value outside the
    /// 13-bit descriptor-table index range.
    #[inline]
    #[must_use]
    pub const fn lift(target_index: u16) -> Option<Self> {
        match NonZero::new(target_index) {
            Some(target_index) => Self::new(target_index),
            None => None,
        }
    }

    /// Create a descriptor index for use in a segment selector.
    ///
    /// Returns `None` when the nonzero index exceeds the 13-bit
    /// descriptor-table index range.
    #[inline]
    #[must_use]
    pub const fn new(target_index: NonZero<u16>) -> Option<Self> {
        const MAX: u16 = const {
            let DescriptorIndex(target_max) = DescriptorIndex::MAX;

            target_max.get()
        };

        match target_index.get() {
            1..=MAX => Some(Self(target_index)),
            0 => unreachable!(),
            _ => None,
        }
    }

    /// Similar to [`DescriptorIndex::new`], but construct the index without any
    /// preliminary validity checks.
    ///
    /// # Safety
    ///
    /// The provided index must be in the inclusive range:
    /// [`DescriptorIndex::MIN`] and [`DescriptorIndex::MAX`].
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(target_index: u16) -> Self {
        Self(
            // SAFETY: `DescriptorIndex::MIN` mirrors `Nonzero::MIN`.
            unsafe { NonZero::new_unchecked(target_index) },
        )
    }

    /// Retrieve the raw [`u16`] value corresponding to this descriptor index.
    #[inline]
    #[must_use]
    pub const fn raw(&self) -> u16 {
        let &Self(target_value) = self;

        NonZero::get(target_value)
    }
}

/// Low part of a reconstructed thirty-two-bit segment base.
pub type SegmentBaseValueLow<'value> = Field<'value, 0, 15, u32>;

/// Mutable counterpart to [`SegmentBaseValueLow`].
pub type SegmentBaseValueLowMut<'value> = <SegmentBaseValueLow<'value> as Counterpart>::Mut;

/// Middle part of a reconstructed thirty-two-bit segment base.
pub type SegmentBaseValueMiddle<'value> = Field<'value, 16, 23, u32>;

/// Mutable counterpart to [`SegmentBaseValueMiddle`].
pub type SegmentBaseValueMiddleMut<'value> = <SegmentBaseValueMiddle<'value> as Counterpart>::Mut;

/// High part of a reconstructed thirty-two-bit segment base.
pub type SegmentBaseValueHigh<'value> = Field<'value, 24, 31, u32>;

/// Mutable counterpart to [`SegmentBaseValueHigh`].
pub type SegmentBaseValueHighMut<'value> = <SegmentBaseValueHigh<'value> as Counterpart>::Mut;

/// Low part of a reconstructed twenty-bit segment limit.
pub type SegmentLimitValueLow<'value> = Field<'value, 0, 15, u32>;

/// Mutable counterpart to [`SegmentLimitValueLow`].
pub type SegmentLimitValueLowMut<'value> = <SegmentLimitValueLow<'value> as Counterpart>::Mut;

/// High part of a reconstructed twenty-bit segment limit.
pub type SegmentLimitValueHigh<'value> = Field<'value, 16, 19, u32>;

/// Mutable counterpart to [`SegmentLimitValueHigh`].
pub type SegmentLimitValueHighMut<'value> = <SegmentLimitValueHigh<'value> as Counterpart>::Mut;

/// The *Limit* field (low part, bits 0-15) in a *Segment Descriptor*.
///
/// This is an alias to a [`Field`] monomorphization.
pub type SegmentLimitLow<'value> = Field<'value, 0, 15, u64>;

/// The *Limit* field (high part, bits 16-19) in a *Segment Descriptor*.
///
/// This is an alias to a [`Field`] monomorphization.
pub type SegmentLimitHigh<'value> = Field<'value, 48, 51, u64>;

/// The *Base* field (low part, bits 16-31) in a *Segment Descriptor*.
///
/// This is an alias to a [`Field`] monomorphization.
pub type SegmentBaseLow<'value> = Field<'value, 16, 31, u64>;

/// The *Base* field (middle part, bits 32-39) in a *Segment Descriptor*.
///
/// This is an alias to a [`Field`] monomorphization.
pub type SegmentBaseMiddle<'value> = Field<'value, 32, 39, u64>;

/// The *Base* field (high part, bits 56-63) in a *Segment Descriptor*.
///
/// This is an alias to a [`Field`] monomorphization.
pub type SegmentBaseHigh<'value> = Field<'value, 56, 63, u64>;

/// The *Flags* field in a *Segment Descriptor*.
///
/// This is an alias to a [`Field`] monomorphization.
pub type SegmentFlags<'value> = Field<'value, 52, 55, u64>;

/// The available-for-system-software flag in a segment descriptor.
pub type SegmentAvailable<'value> = Bit<'value, u64, 52>;

/// The 64-bit code-segment flag in a segment descriptor.
pub type SegmentLongMode<'value> = Bit<'value, u64, 53>;

/// Mutable counterpart to [`SegmentLongMode`].
pub type SegmentLongModeMut<'value> = <SegmentLongMode<'value> as Counterpart>::Mut;

/// The default operand-size flag in a segment descriptor.
pub type SegmentDefaultSize<'value> = Bit<'value, u64, 54>;

/// Mutable counterpart to [`SegmentDefaultSize`].
pub type SegmentDefaultSizeMut<'value> = <SegmentDefaultSize<'value> as Counterpart>::Mut;

/// The limit-granularity flag in a segment descriptor.
pub type SegmentGranularity<'value> = Bit<'value, u64, 55>;

/// Mutable counterpart to [`SegmentGranularity`].
pub type SegmentGranularityMut<'value> = <SegmentGranularity<'value> as Counterpart>::Mut;

/// The *Access Byte* field in a *Segment Descriptor*.
///
/// This is an alias to a [`Field`] monomorphization.
pub type SegmentAccessByte<'value> = Field<'value, 40, 47, u64>;

/// The *Limit* field (low part, bits 0-15) in a *Segment Descriptor*.
///
/// This is an alias to the mutable counterpart of [`SegmentLimitLow`].
pub type SegmentLimitLowMut<'value> = <SegmentLimitLow<'value> as Counterpart>::Mut;

/// The *Limit* field (high part, bits 16-19) in a *Segment Descriptor*.
///
/// This is an alias to the mutable counterpart of [`SegmentLimitHigh`].
pub type SegmentLimitHighMut<'value> = <SegmentLimitHigh<'value> as Counterpart>::Mut;

/// The *Base* field (low part, bits 16-31) in a *Segment Descriptor*.
///
/// This is an alias to the mutable counterpart of [`SegmentBaseLow`].
pub type SegmentBaseLowMut<'value> = <SegmentBaseLow<'value> as Counterpart>::Mut;

/// The *Base* field (middle part, bits 32-39) in a *Segment Descriptor*.
///
/// This is an alias to the mutable counterpart of [`SegmentBaseMiddle`].
pub type SegmentBaseMiddleMut<'value> = <SegmentBaseMiddle<'value> as Counterpart>::Mut;

/// The *Base* field (high part, bits 56-63) in a *Segment Descriptor*.
///
/// This is an alias to the mutable counterpart of [`SegmentBaseHigh`].
pub type SegmentBaseHighMut<'value> = <SegmentBaseHigh<'value> as Counterpart>::Mut;

/// The *Flags* field in a *Segment Descriptor*.
///
/// This is an alias to the mutable counterpart of [`SegmentFlags`].
pub type SegmentFlagsMut<'value> = <SegmentFlags<'value> as Counterpart>::Mut;

/// The *Access Byte* field in a *Segment Descriptor*.
///
/// This is an alias to the mutable counterpart of [`SegmentAccessByte`].
pub type SegmentAccessByteMut<'value> = <SegmentAccessByte<'value> as Counterpart>::Mut;

/// A raw *Segment Descriptor* representation.
///
/// Reference: <https://wiki.osdev.org/Global_Descriptor_Table>
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, IntoBytes, Immutable)]
#[repr(C, align(8), /* must be 8-byte aligned */)]
// NOTE(invariant): The private scalar is one complete eight-byte architectural segment descriptor
// image and the explicit alignment matches descriptor-table storage requirements.
pub struct RawSegmentDescriptor(u64);

impl RawSegmentDescriptor {
    /// The mandated null segment descriptor.
    pub const NULL: Self = Self(u64::MIN);

    /// Constructs a raw segment-descriptor image.
    ///
    /// Every `u64` is representable as a raw descriptor image. This constructor does not prove
    /// any segment or system-descriptor semantics.
    #[inline]
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Constructs a flat readable long-mode code descriptor.
    #[inline]
    #[must_use]
    pub const fn long_mode_code(privilege: PrivilegeLevel) -> Self {
        let target_limit = u32::MAX;
        let limit_low = SegmentLimitValueLow::wrap(&target_limit).const_value();
        let limit_high = SegmentLimitValueHigh::wrap(&target_limit).const_value();
        let mut access_byte = RawAccessByte::new(u8::MIN);

        access_byte.present_mut().const_set(State::Set);
        access_byte.privilege_mut().const_merge(privilege.raw());
        access_byte.system_mut().const_set(State::Set);
        access_byte.executable_mut().const_set(State::Set);
        access_byte.read_write_mut().const_set(State::Set);
        access_byte.accessed_mut().const_set(State::Set);

        let mut descriptor = Self::NULL;

        descriptor.limit_low_mut().const_merge(limit_low);
        descriptor.limit_high_mut().const_merge(limit_high);
        descriptor
            .access_byte_mut()
            .const_merge(RawAccessByte::raw(access_byte));
        descriptor.long_mode_mut().const_set(State::Set);
        descriptor.granularity_mut().const_set(State::Set);

        descriptor
    }

    /// Constructs a flat writable data descriptor.
    #[inline]
    #[must_use]
    pub const fn flat_data(privilege: PrivilegeLevel) -> Self {
        let target_limit = u32::MAX;
        let limit_low = SegmentLimitValueLow::wrap(&target_limit).const_value();
        let limit_high = SegmentLimitValueHigh::wrap(&target_limit).const_value();
        let mut access = RawAccessByte::new(u8::MIN);

        access.present_mut().const_set(State::Set);
        access.privilege_mut().const_merge(privilege.raw());
        access.system_mut().const_set(State::Set);
        access.read_write_mut().const_set(State::Set);
        access.accessed_mut().const_set(State::Set);

        let mut descriptor = Self::NULL;

        descriptor.limit_low_mut().const_merge(limit_low);
        descriptor.limit_high_mut().const_merge(limit_high);
        descriptor.access_byte_mut().const_merge(access.raw());
        descriptor.default_size_mut().const_set(State::Set);
        descriptor.granularity_mut().const_set(State::Set);

        descriptor
    }

    /// Returns the encoded descriptor value.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u64 {
        let Self(value) = self;

        value
    }

    /// Returns the 32-bit base encoded by this descriptor.
    #[inline]
    #[must_use]
    pub const fn base(&self) -> u32 {
        let low = self.base_low().const_value();
        let middle = self.base_middle().const_value();
        let high = self.base_high().const_value();
        let mut value = u32::MIN;
        let mut low_field = SegmentBaseValueLowMut::wrap(&mut value);

        low_field.const_merge(low);

        let mut middle_field = SegmentBaseValueMiddleMut::wrap(&mut value);

        middle_field.const_merge(middle);

        let mut high_field = SegmentBaseValueHighMut::wrap(&mut value);

        high_field.const_merge(high);

        value
    }

    /// Returns the effective byte limit described by this descriptor.
    #[inline]
    #[must_use]
    pub const fn effective_limit(&self) -> u32 {
        let low = self.limit_low().const_value();
        let high = self.limit_high().const_value();
        let mut encoded = u32::MIN;
        let mut low_field = SegmentLimitValueLowMut::wrap(&mut encoded);

        low_field.const_merge(low);

        let mut high_field = SegmentLimitValueHighMut::wrap(&mut encoded);

        high_field.const_merge(high);

        match self.granularity().const_state() {
            State::Cleared => encoded,
            State::Set => (encoded << 12) | 0x0fff,
        }
    }

    /// Returns the raw access byte encoded by this descriptor.
    #[inline]
    #[must_use]
    pub const fn access(&self) -> RawAccessByte {
        let value = self.access_byte().const_value();

        RawAccessByte::new(value)
    }

    /// Determines the available-for-system-software flag.
    #[inline]
    #[must_use]
    pub const fn available(&self) -> SegmentAvailable<'_> {
        let &Self(ref value) = self;

        SegmentAvailable::wrap(value)
    }

    /// Determines the 64-bit code-segment flag.
    #[inline]
    #[must_use]
    pub const fn long_mode(&self) -> SegmentLongMode<'_> {
        let &Self(ref value) = self;

        SegmentLongMode::wrap(value)
    }

    /// Mutably accesses the 64-bit code-segment flag.
    #[inline]
    pub const fn long_mode_mut(&mut self) -> SegmentLongModeMut<'_> {
        let &mut Self(ref mut target_value) = self;

        SegmentLongModeMut::wrap(target_value)
    }

    /// Determines the default operand-size flag.
    #[inline]
    #[must_use]
    pub const fn default_size(&self) -> SegmentDefaultSize<'_> {
        let &Self(ref value) = self;

        SegmentDefaultSize::wrap(value)
    }

    /// Mutably accesses the default operand-size flag.
    #[inline]
    pub const fn default_size_mut(&mut self) -> SegmentDefaultSizeMut<'_> {
        let &mut Self(ref mut target_value) = self;

        SegmentDefaultSizeMut::wrap(target_value)
    }

    /// Determines the limit-granularity flag.
    #[inline]
    #[must_use]
    pub const fn granularity(&self) -> SegmentGranularity<'_> {
        let &Self(ref value) = self;

        SegmentGranularity::wrap(value)
    }

    /// Mutably accesses the limit-granularity flag.
    #[inline]
    pub const fn granularity_mut(&mut self) -> SegmentGranularityMut<'_> {
        let &mut Self(ref mut target_value) = self;

        SegmentGranularityMut::wrap(target_value)
    }

    /// Determines whether the available-for-system-software flag is set.
    #[inline]
    #[must_use]
    pub const fn is_available(&self) -> bool {
        matches!(self.available().const_state(), State::Set)
    }

    /// Determines whether this descriptor selects 64-bit code semantics.
    #[inline]
    #[must_use]
    pub const fn is_long_mode(&self) -> bool {
        matches!(self.long_mode().const_state(), State::Set)
    }

    /// Determines whether the default operand-size flag is set.
    #[inline]
    #[must_use]
    pub const fn has_default_size(&self) -> bool {
        matches!(self.default_size().const_state(), State::Set)
    }

    /// Determines whether the limit uses 4 KiB granularity.
    #[inline]
    #[must_use]
    pub const fn has_page_granularity(&self) -> bool {
        matches!(self.granularity().const_state(), State::Set)
    }

    /// Access the *Limit* field (low part) in this *Segment Descriptor*.
    #[inline]
    #[must_use]
    pub const fn limit_low(&self) -> SegmentLimitLow<'_> {
        let &Self(ref target_value) = self;

        SegmentLimitLow::wrap(target_value)
    }

    /// Mutably access the *Limit* field (low part) in this *Segment
    /// Descriptor*.
    #[inline]
    pub const fn limit_low_mut(&mut self) -> SegmentLimitLowMut<'_> {
        let &mut Self(ref mut target_value) = self;

        SegmentLimitLowMut::wrap(target_value)
    }

    /// Access the *Limit* field (high part) in this *Segment Descriptor*.
    #[inline]
    #[must_use]
    pub const fn limit_high(&self) -> SegmentLimitHigh<'_> {
        let &Self(ref target_value) = self;

        SegmentLimitHigh::wrap(target_value)
    }

    /// Mutably access the *Limit* field (high part) in this *Segment
    /// Descriptor*.
    #[inline]
    pub const fn limit_high_mut(&mut self) -> SegmentLimitHighMut<'_> {
        let &mut Self(ref mut target_value) = self;

        SegmentLimitHighMut::wrap(target_value)
    }

    /// Access the *Base* field (low part) in this *Segment Descriptor*.
    #[inline]
    #[must_use]
    pub const fn base_low(&self) -> SegmentBaseLow<'_> {
        let &Self(ref target_value) = self;

        SegmentBaseLow::wrap(target_value)
    }

    /// Mutably access the *Base* field (low part) in this *Segment Descriptor*.
    #[inline]
    pub const fn base_low_mut(&mut self) -> SegmentBaseLowMut<'_> {
        let &mut Self(ref mut target_value) = self;

        SegmentBaseLowMut::wrap(target_value)
    }

    /// Access the *Base* field (middle part) in this *Segment Descriptor*.
    #[inline]
    #[must_use]
    pub const fn base_middle(&self) -> SegmentBaseMiddle<'_> {
        let &Self(ref target_value) = self;

        SegmentBaseMiddle::wrap(target_value)
    }

    /// Mutably access the *Base* field (middle part) in this *Segment
    /// Descriptor*.
    #[inline]
    pub const fn base_middle_mut(&mut self) -> SegmentBaseMiddleMut<'_> {
        let &mut Self(ref mut target_value) = self;

        SegmentBaseMiddleMut::wrap(target_value)
    }

    /// Access the *Base* field (high part) in this *Segment Descriptor*.
    #[inline]
    #[must_use]
    pub const fn base_high(&self) -> SegmentBaseHigh<'_> {
        let &Self(ref target_value) = self;

        SegmentBaseHigh::wrap(target_value)
    }

    /// Mutably access the *Base* field (high part) in this *Segment
    /// Descriptor*.
    #[inline]
    pub const fn base_high_mut(&mut self) -> SegmentBaseHighMut<'_> {
        let &mut Self(ref mut target_value) = self;

        SegmentBaseHighMut::wrap(target_value)
    }

    /// Access the *Flags* field in this *Segment Descriptor*.
    #[inline]
    #[must_use]
    pub const fn flags(&self) -> SegmentFlags<'_> {
        let &Self(ref target_value) = self;

        SegmentFlags::wrap(target_value)
    }

    /// Mutably access the *Flags* field in this *Segment Descriptor*.
    #[inline]
    pub const fn flags_mut(&mut self) -> SegmentFlagsMut<'_> {
        let &mut Self(ref mut target_value) = self;

        SegmentFlagsMut::wrap(target_value)
    }

    /// Access the *Access Byte* field in this *Segment Descriptor*.
    #[inline]
    #[must_use]
    pub const fn access_byte(&self) -> SegmentAccessByte<'_> {
        let &Self(ref target_value) = self;

        SegmentAccessByte::wrap(target_value)
    }

    /// Mutably access the *Access Byte* field in this *Segment Descriptor*.
    #[inline]
    pub const fn access_byte_mut(&mut self) -> SegmentAccessByteMut<'_> {
        let &mut Self(ref mut target_value) = self;

        SegmentAccessByteMut::wrap(target_value)
    }
}

/// The *Present* bit in an *Access Byte*.
///
/// This is an alias to a [`Bit`] monomorphization.
pub type AccessBytePresent<'value> = Bit<'value, u8, 7>;

/// The *Present* bit in an *Access Byte*.
///
/// This is an alias to a [`Field`] monomorphization.
pub type AccessByteDpl<'value> = Field<'value, 5, 6, u8>;

/// The *Present* bit in an *Access Byte*.
///
/// This is an alias to a [`Bit`] monomorphization.
pub type AccessByteSystem<'value> = Bit<'value, u8, 4>;

/// The *Executable* bit in an *Access Byte*.
///
/// Determines whether the descriptor is a *Code Segment* or *Data Segment*
/// descriptor. This alters the behavior of [`AccessByteDc`] and
/// [`AccessByteRw`].
///
/// This is an alias to a [`Bit`] monomorphization.
pub type AccessByteExecutable<'value> = Bit<'value, u8, 3>;

/// The *Direction/Conforming* bit in an *Access Byte*.
///
/// This is an alias to a [`Bit`] monomorphization.
pub type AccessByteDc<'value> = Bit<'value, u8, 2>;

/// The *Read/Write* bit in an *Access Byte*.
///
/// This is an alias to a [`Bit`] monomorphization.
pub type AccessByteRw<'value> = Bit<'value, u8, 1>;

/// The *Accessed* bit in an *Access Byte*.
///
/// This is an alias to a [`Bit`] monomorphization.
pub type AccessByteAccessed<'value> = Bit<'value, u8, 0>;

/// The *Type* field in an *Access Byte* (for system descriptors).
///
/// This 4-bit field encodes the type of system descriptor (TSS, LDT, Gate,
/// etc.). Only valid when the [`AccessByteSystem`] bitfield is `0`.
///
/// This is an alias to a [`Field`] monomorphization.
pub type AccessByteDescriptorType<'value> = Field<'value, 0, 3, u8>;

/// The *Present* bit in an *Access Byte*.
///
/// This is an alias to the mutable counterpart of [`AccessBytePresent`].
pub type AccessBytePresentMut<'value> = <AccessBytePresent<'value> as Counterpart>::Mut;

/// The *Descriptor Privilege Level* field in an *Access Byte*.
///
/// This is an alias to the mutable counterpart of [`AccessByteDpl`].
pub type AccessByteDplMut<'value> = <AccessByteDpl<'value> as Counterpart>::Mut;

/// The *System* bit in an *Access Byte*.
///
/// This is an alias to the mutable counterpart of [`AccessByteSystem`].
pub type AccessByteSystemMut<'value> = <AccessByteSystem<'value> as Counterpart>::Mut;

/// The *Executable* bit in an *Access Byte*.
///
/// This is an alias to the mutable counterpart of [`AccessByteExecutable`].
pub type AccessByteExecutableMut<'value> = <AccessByteExecutable<'value> as Counterpart>::Mut;

/// The *Direction/Conforming* bit in an *Access Byte*.
///
/// This is an alias to the mutable counterpart of [`AccessByteDc`].
pub type AccessByteDcMut<'value> = <AccessByteDc<'value> as Counterpart>::Mut;

/// The *Read/Write* bit in an *Access Byte*.
///
/// This is an alias to the mutable counterpart of [`AccessByteRw`].
pub type AccessByteRwMut<'value> = <AccessByteRw<'value> as Counterpart>::Mut;

/// The *Accessed* bit in an *Access Byte*.
///
/// This is an alias to the mutable counterpart of [`AccessByteAccessed`].
pub type AccessByteAccessedMut<'value> = <AccessByteAccessed<'value> as Counterpart>::Mut;

/// The *Type* field in an *Access Byte* (for system descriptors).
///
/// This is an alias to the mutable counterpart of [`AccessByteDescriptorType`].
pub type AccessByteDescriptorTypeMut<'value> = <AccessByteDescriptorType<'value> as Counterpart>::Mut;

/// System Descriptor Type values for the *Type* field in an *Access Byte*.
///
/// This provides named enumeration variants for the
/// [`AccessByteDescriptorType`] field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
#[non_exhaustive]
pub enum SystemDescriptorType {
    /// 16-bit Task State Segment (Available).
    ///
    /// **Note**: Not available in Long Mode.
    TssAvailable16 = 0x1,

    /// Local Descriptor Table.
    ///
    /// Valid in both Protected Mode and Long Mode.
    Ldt = 0x2,

    /// 16-bit Task State Segment (Busy).
    ///
    /// **Note**: Not available in Long Mode.
    TssBusy16 = 0x3,

    /// 16-bit Call Gate.
    ///
    /// **Note**: Not available in Long Mode.
    CallGate16 = 0x4,

    /// Task Gate.
    ///
    /// Used for hardware task switching in Protected Mode.
    ///
    /// **Note**: Not available in Long Mode.
    TaskGate = 0x5,

    /// 16-bit Interrupt Gate.
    ///
    /// **Note**: Not available in Long Mode.
    InterruptGate16 = 0x6,

    /// 16-bit Trap Gate.
    ///
    /// **Note**: Not available in Long Mode.
    TrapGate16 = 0x7,

    /// 32-bit/64-bit Task State Segment (Available).
    ///
    /// In Protected Mode, this is a 32-bit TSS.
    /// In Long Mode, this is a 64-bit TSS.
    TssAvailable32 = 0x9,

    /// 32-bit/64-bit Task State Segment (Busy).
    ///
    /// In Protected Mode, this is a 32-bit TSS.
    /// In Long Mode, this is a 64-bit TSS.
    TssBusy32 = 0xB,

    /// 32-bit Call Gate.
    ///
    /// **Note**: Not available in Long Mode.
    CallGate32 = 0xC,

    /// 32-bit/64-bit Interrupt Gate.
    ///
    /// In Protected Mode, this is a 32-bit Interrupt Gate.
    /// In Long Mode, this is a 64-bit Interrupt Gate.
    InterruptGate32 = 0xE,

    /// 32-bit/64-bit Trap Gate.
    ///
    /// In Protected Mode, this is a 32-bit Trap Gate.
    /// In Long Mode, this is a 64-bit Trap Gate.
    TrapGate32 = 0xF,
}

/// An *Access Byte* in a *Segment Descriptor*.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): Every stored bit pattern is preserved as one complete architectural segment
// access byte without imposing a higher-level descriptor-class interpretation.
pub struct RawAccessByte(u8);

impl RawAccessByte {
    /// Constructs a raw access byte from its architectural image.
    #[inline]
    #[must_use]
    pub const fn new(value: u8) -> Self {
        Self(value)
    }

    /// Returns the architectural access-byte image.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u8 {
        let Self(value) = self;

        value
    }

    /// Returns the four descriptor type bits without class interpretation.
    #[inline]
    #[must_use]
    pub const fn type_bits(&self) -> u8 {
        self.descriptor_type().const_value()
    }

    /// Returns the descriptor privilege level.
    #[inline]
    #[must_use]
    pub const fn dpl(&self) -> u8 {
        self.privilege().const_value()
    }

    /// Determines whether this descriptor is present.
    #[inline]
    #[must_use]
    pub const fn is_present(&self) -> bool {
        matches!(self.present().const_state(), State::Set)
    }

    /// Determines whether the descriptor-class bit selects code or data.
    #[inline]
    #[must_use]
    pub const fn is_code_or_data(&self) -> bool {
        matches!(self.system().const_state(), State::Set)
    }

    /// Access the *Present* bit in this *Access Byte*.
    #[inline]
    #[must_use]
    pub const fn present(&self) -> AccessBytePresent<'_> {
        let &Self(ref target_value) = self;

        AccessBytePresent::wrap(target_value)
    }

    /// Mutably access the *Present* bit in this *Access Byte*.
    #[inline]
    pub const fn present_mut(&mut self) -> AccessBytePresentMut<'_> {
        let &mut Self(ref mut target_value) = self;

        AccessBytePresentMut::wrap(target_value)
    }

    /// Access the *Described Privilege Level* field in this *Access Byte*.
    #[inline]
    #[must_use]
    pub const fn privilege(&self) -> AccessByteDpl<'_> {
        let &Self(ref target_value) = self;

        AccessByteDpl::wrap(target_value)
    }

    /// Mutably access the *Described Privilege Level* field in this *Access
    /// Byte*.
    #[inline]
    pub const fn privilege_mut(&mut self) -> AccessByteDplMut<'_> {
        let &mut Self(ref mut target_value) = self;

        AccessByteDplMut::wrap(target_value)
    }

    /// Access the *System* bit in this *Access Byte*.
    #[inline]
    #[must_use]
    pub const fn system(&self) -> AccessByteSystem<'_> {
        let &Self(ref target_value) = self;

        AccessByteSystem::wrap(target_value)
    }

    /// Mutably access the *System* bit in this *Access Byte*.
    #[inline]
    pub const fn system_mut(&mut self) -> AccessByteSystemMut<'_> {
        let &mut Self(ref mut target_value) = self;

        AccessByteSystemMut::wrap(target_value)
    }

    /// Access the *Executable* bit in this *Access Byte*.
    #[inline]
    #[must_use]
    pub const fn executable(&self) -> AccessByteExecutable<'_> {
        let &Self(ref target_value) = self;

        AccessByteExecutable::wrap(target_value)
    }

    /// Mutably access the *Executable* bit in this *Access Byte*.
    #[inline]
    pub const fn executable_mut(&mut self) -> AccessByteExecutableMut<'_> {
        let &mut Self(ref mut target_value) = self;

        AccessByteExecutableMut::wrap(target_value)
    }

    /// Access the *Direction/Conforming* bit in this *Access Byte*.
    #[inline]
    #[doc(alias = "conforming")]
    #[must_use]
    pub const fn direction(&self) -> AccessByteDc<'_> {
        let &Self(ref target_value) = self;

        AccessByteDc::wrap(target_value)
    }

    /// Mutably access the *Direction/Conforming* bit in this *Access Byte*.
    #[inline]
    #[doc(alias = "conforming_mut")]
    pub const fn direction_mut(&mut self) -> AccessByteDcMut<'_> {
        let &mut Self(ref mut target_value) = self;

        AccessByteDcMut::wrap(target_value)
    }

    /// Access the *Read/Write* (*RW*) bit in this *Access Byte*.
    #[inline]
    #[must_use]
    pub const fn read_write(&self) -> AccessByteRw<'_> {
        let &Self(ref target_value) = self;

        AccessByteRw::wrap(target_value)
    }

    /// Mutably access the *Read/Write* (*RW*) bit in this *Access Byte*.
    #[inline]
    pub const fn read_write_mut(&mut self) -> AccessByteRwMut<'_> {
        let &mut Self(ref mut target_value) = self;

        AccessByteRwMut::wrap(target_value)
    }

    /// Access the *Accessed* bit in this *Access Byte*.
    #[inline]
    #[must_use]
    pub const fn accessed(&self) -> AccessByteAccessed<'_> {
        let &Self(ref target_value) = self;

        AccessByteAccessed::wrap(target_value)
    }

    /// Mutably access the *Accessed* bit in this *Access Byte*.
    #[inline]
    pub const fn accessed_mut(&mut self) -> AccessByteAccessedMut<'_> {
        let &mut Self(ref mut target_value) = self;

        AccessByteAccessedMut::wrap(target_value)
    }

    /// Access the *Type* field in this *Access Byte* (for system descriptors).
    ///
    /// This determines what kind of system descriptor this *Access Byte* is
    /// part of.
    ///
    /// # Note
    ///
    /// This field overlaps with [`Self::executable`], [`Self::direction`],
    /// [`Self::read_write`], and [`Self::accessed`]. Use this accessor for
    /// system descriptors only.
    #[inline]
    #[must_use]
    pub const fn descriptor_type(&self) -> AccessByteDescriptorType<'_> {
        let &Self(ref target_value) = self;

        AccessByteDescriptorType::wrap(target_value)
    }

    /// Mutably access the *Type* field in this *Access Byte* (for system
    /// descriptors).
    ///
    /// This determines what kind of system descriptor this *Access Byte* is
    /// part of.
    ///
    /// # Note
    ///
    /// This field overlaps with [`Self::executable`], [`Self::direction`],
    /// [`Self::read_write`], and [`Self::accessed`]. Use this accessor for
    /// system descriptors only.
    #[inline]
    pub const fn descriptor_type_mut(&mut self) -> AccessByteDescriptorTypeMut<'_> {
        let &mut Self(ref mut target_value) = self;

        AccessByteDescriptorTypeMut::wrap(target_value)
    }
}

#[cfg(test)]
mod representation_tests {
    use zerocopy::IntoBytes;

    use super::RawSegmentDescriptor;
    use crate::x86::privilege::PrivilegeLevel;

    #[test]
    fn long_mode_code_descriptor_matches_architectural_encoding() {
        let descriptor = RawSegmentDescriptor::long_mode_code(PrivilegeLevel::Ring0);

        assert_eq!(descriptor.raw(), 0x00af_9b00_0000_ffff);
        assert_eq!(descriptor.as_bytes(), [0xff, 0xff, 0, 0, 0, 0x9b, 0xaf, 0]);
    }

    #[test]
    fn flat_data_descriptor_matches_architectural_encoding() {
        let descriptor = RawSegmentDescriptor::flat_data(PrivilegeLevel::Ring0);

        assert_eq!(descriptor.raw(), 0x00cf_9300_0000_ffff);
    }
}

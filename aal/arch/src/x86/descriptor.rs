//! In-memory descriptor tables.

use core::num::NonZero;

use nekor_bitwise::prelude::{Bit, Counterpart, Field};

use crate::x86::{
    address::Address,
    mode::{Mode, Native},
};

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
pub struct DescriptorIndex(
    // NOTE(invariant): Can never index into mandated null descriptor, nor
    // exceed the limit imposed by a `DescriptorTablePointer`.
    NonZero<u16>,
);

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
    /// This corresponds to the *null segment descriptor* in a *descriptor
    /// table*.
    pub const MIN: Self = Self(NonZero::<u16>::MIN);
}

impl DescriptorIndex {
    /// Construct a new [`DescriptorIndex`] to be used as a *Segment Selector*
    /// index.
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

/// The *Limit* field (low part, bits 0-15) in a *Segment Descriptor*.
///
/// This is an alias to a [`Field`] monomorphization.
pub type SegmentLimitLow<'a> = Field<'a, 0, 15, u64>;

/// The *Limit* field (high part, bits 16-19) in a *Segment Descriptor*.
///
/// This is an alias to a [`Field`] monomorphization.
pub type SegmentLimitHigh<'a> = Field<'a, 48, 51, u64>;

/// The *Base* field (low part, bits 16-31) in a *Segment Descriptor*.
///
/// This is an alias to a [`Field`] monomorphization.
pub type SegmentBaseLow<'a> = Field<'a, 16, 31, u64>;

/// The *Base* field (middle part, bits 32-39) in a *Segment Descriptor*.
///
/// This is an alias to a [`Field`] monomorphization.
pub type SegmentBaseMiddle<'a> = Field<'a, 32, 39, u64>;

/// The *Base* field (high part, bits 56-63) in a *Segment Descriptor*.
///
/// This is an alias to a [`Field`] monomorphization.
pub type SegmentBaseHigh<'a> = Field<'a, 56, 63, u64>;

/// The *Flags* field in a *Segment Descriptor*.
///
/// This is an alias to a [`Field`] monomorphization.
pub type SegmentFlags<'a> = Field<'a, 52, 55, u64>;

/// The *Access Byte* field in a *Segment Descriptor*.
///
/// This is an alias to a [`Field`] monomorphization.
pub type SegmentAccessByte<'a> = Field<'a, 40, 47, u64>;

/// The *Limit* field (low part, bits 0-15) in a *Segment Descriptor*.
///
/// This is an alias to the mutable counterpart of [`SegmentLimitLow`].
pub type SegmentLimitLowMut<'a> = <SegmentLimitLow<'a> as Counterpart>::Mut;

/// The *Limit* field (high part, bits 16-19) in a *Segment Descriptor*.
///
/// This is an alias to the mutable counterpart of [`SegmentLimitHigh`].
pub type SegmentLimitHighMut<'a> = <SegmentLimitHigh<'a> as Counterpart>::Mut;

/// The *Base* field (low part, bits 16-31) in a *Segment Descriptor*.
///
/// This is an alias to the mutable counterpart of [`SegmentBaseLow`].
pub type SegmentBaseLowMut<'a> = <SegmentBaseLow<'a> as Counterpart>::Mut;

/// The *Base* field (middle part, bits 32-39) in a *Segment Descriptor*.
///
/// This is an alias to the mutable counterpart of [`SegmentBaseMiddle`].
pub type SegmentBaseMiddleMut<'a> = <SegmentBaseMiddle<'a> as Counterpart>::Mut;

/// The *Base* field (high part, bits 56-63) in a *Segment Descriptor*.
///
/// This is an alias to the mutable counterpart of [`SegmentBaseHigh`].
pub type SegmentBaseHighMut<'a> = <SegmentBaseHigh<'a> as Counterpart>::Mut;

/// The *Flags* field in a *Segment Descriptor*.
///
/// This is an alias to the mutable counterpart of [`SegmentFlags`].
pub type SegmentFlagsMut<'a> = <SegmentFlags<'a> as Counterpart>::Mut;

/// The *Access Byte* field in a *Segment Descriptor*.
///
/// This is an alias to the mutable counterpart of [`SegmentAccessByte`].
pub type SegmentAccessByteMut<'a> = <SegmentAccessByte<'a> as Counterpart>::Mut;

/// A raw *Segment Descriptor* representation.
///
/// Reference: <https://wiki.osdev.org/Global_Descriptor_Table>
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C, align(8), /* must be 8-byte aligned */)]
pub struct RawSegmentDescriptor(u64);

impl RawSegmentDescriptor {
    /// Access the *Limit* field (low part) in this *Segment Descriptor*.
    #[inline]
    #[must_use]
    pub const fn limit_low(&self) -> SegmentLimitLow<'_> {
        let Self(target_value) = self;

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
        let Self(target_value) = self;

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
        let Self(target_value) = self;

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
        let Self(target_value) = self;

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
        let Self(target_value) = self;

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
        let Self(target_value) = self;

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
        let Self(target_value) = self;

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
pub type AccessBytePresent<'a> = Bit<'a, u8, 7>;

/// The *Present* bit in an *Access Byte*.
///
/// This is an alias to a [`Field`] monomorphization.
pub type AccessByteDpl<'a> = Field<'a, 5, 6, u8>;

/// The *Present* bit in an *Access Byte*.
///
/// This is an alias to a [`Bit`] monomorphization.
pub type AccessByteSystem<'a> = Bit<'a, u8, 4>;

/// The *Executable* bit in an *Access Byte*.
///
/// Determines whether the descriptor is a *Code Segment* or *Data Segment*
/// descriptor. This alters the behavior of [`AccessByteDc`] and
/// [`AccessByteRw`].
///
/// This is an alias to a [`Bit`] monomorphization.
pub type AccessByteExecutable<'a> = Bit<'a, u8, 3>;

/// The *Direction/Conforming* bit in an *Access Byte*.
///
/// This is an alias to a [`Bit`] monomorphization.
pub type AccessByteDc<'a> = Bit<'a, u8, 2>;

/// The *Read/Write* bit in an *Access Byte*.
///
/// This is an alias to a [`Bit`] monomorphization.
pub type AccessByteRw<'a> = Bit<'a, u8, 1>;

/// The *Accessed* bit in an *Access Byte*.
///
/// This is an alias to a [`Bit`] monomorphization.
pub type AccessByteAccessed<'a> = Bit<'a, u8, 0>;

/// The *Type* field in an *Access Byte* (for system descriptors).
///
/// This 4-bit field encodes the type of system descriptor (TSS, LDT, Gate,
/// etc.). Only valid when the [`AccessByteSystem`] bitfield is `0`.
///
/// This is an alias to a [`Field`] monomorphization.
pub type AccessByteDescriptorType<'a> = Field<'a, 0, 3, u8>;

/// The *Present* bit in an *Access Byte*.
///
/// This is an alias to a [`BitMut`] monomorphization.
///
/// [`BitMut`]: nekor_bitwise::handle::BitMut
pub type AccessBytePresentMut<'a> = <AccessBytePresent<'a> as Counterpart>::Mut;

/// The *Descriptor Privilege Level* field in an *Access Byte*.
///
/// This is an alias to the mutable counterpart of [`AccessByteDpl`].
pub type AccessByteDplMut<'a> = <AccessByteDpl<'a> as Counterpart>::Mut;

/// The *System* bit in an *Access Byte*.
///
/// This is an alias to the mutable counterpart of [`AccessByteSystem`].
pub type AccessByteSystemMut<'a> = <AccessByteSystem<'a> as Counterpart>::Mut;

/// The *Executable* bit in an *Access Byte*.
///
/// This is an alias to the mutable counterpart of [`AccessByteExecutable`].
pub type AccessByteExecutableMut<'a> = <AccessByteExecutable<'a> as Counterpart>::Mut;

/// The *Direction/Conforming* bit in an *Access Byte*.
///
/// This is an alias to the mutable counterpart of [`AccessByteDc`].
pub type AccessByteDcMut<'a> = <AccessByteDc<'a> as Counterpart>::Mut;

/// The *Read/Write* bit in an *Access Byte*.
///
/// This is an alias to the mutable counterpart of [`AccessByteRw`].
pub type AccessByteRwMut<'a> = <AccessByteRw<'a> as Counterpart>::Mut;

/// The *Accessed* bit in an *Access Byte*.
///
/// This is an alias to the mutable counterpart of [`AccessByteAccessed`].
pub type AccessByteAccessedMut<'a> = <AccessByteAccessed<'a> as Counterpart>::Mut;

/// The *Type* field in an *Access Byte* (for system descriptors).
///
/// This is an alias to the mutable counterpart of [`AccessByteDescriptorType`].
pub type AccessByteDescriptorTypeMut<'a> = <AccessByteDescriptorType<'a> as Counterpart>::Mut;

/// System Descriptor Type values for the *Type* field in an *Access Byte*.
///
/// This provides named enumeration variants for the [`AccessByteType`] field.
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
pub struct RawAccessByte(u8);

impl RawAccessByte {
    /// Access the *Present* bit in this *Access Byte*.
    #[inline]
    #[must_use]
    pub const fn present(&self) -> AccessBytePresent<'_> {
        let Self(target_value) = self;

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
        let Self(target_value) = self;

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
        let Self(target_value) = self;

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
        let Self(target_value) = self;

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
        let Self(target_value) = self;

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
        let Self(target_value) = self;

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
        let Self(target_value) = self;

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
        let Self(target_value) = self;

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

//! Long-mode descriptor-table registers and system descriptors.

use core::{marker::PhantomData, num::NonZero};

use nekor_aal_agnostic::partitioned::Partitioned;
use nekor_bitwise::prelude::{Bit, Counterpart, Field, State};
use zerocopy::{Immutable, IntoBytes};

use crate::{
    x86::{
        descriptor::{DescriptorIndex, DescriptorTable, Gdt, Idt, SystemDescriptorType},
        privilege::PrivilegeLevel,
    },
    x86_64::{paging::La, task::RawTaskStateSegment},
};

/// A long-mode descriptor-table register image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
// NOTE(invariant): `base` is a canonical linear address, `limit` is the one-biased byte-length
// image accepted by x86, and `table` preserves the descriptor-table kind.
pub struct DescriptorTableRegister<T>
where
    T: DescriptorTable,
{
    /// Canonical linear base address.
    base: La,

    /// Architectural byte limit with a bias of 1.
    limit: u16,

    /// Descriptor-table kind carried only at the type level.
    table: PhantomData<fn() -> T>,
}

/// Long-mode global-descriptor-table register state.
pub type GdtRegister = DescriptorTableRegister<Gdt>;

/// Long-mode interrupt-descriptor-table register state.
pub type IdtRegister = DescriptorTableRegister<Idt>;

/// Fixed Nekor supervisor code descriptor index.
pub const SUPERVISOR_CODE_INDEX: DescriptorIndex = const {
    // SAFETY: One is inside the architectural nonzero thirteen-bit descriptor index domain.
    unsafe { DescriptorIndex::new_unchecked(1) }
};

/// Fixed Nekor supervisor data descriptor index.
pub const SUPERVISOR_DATA_INDEX: DescriptorIndex = const {
    // SAFETY: Two is inside the architectural nonzero thirteen-bit descriptor index domain.
    unsafe { DescriptorIndex::new_unchecked(2) }
};

/// Fixed Nekor long-mode task descriptor index.
pub const TASK_INDEX: DescriptorIndex = const {
    // SAFETY: Three is inside the architectural nonzero thirteen-bit descriptor index domain.
    unsafe { DescriptorIndex::new_unchecked(3) }
};

/// Low sixteen bits of a long-mode system-descriptor base value.
pub type SystemDescriptorBaseValueLow<'value> = Field<'value, 0, 15, u64>;

/// Mutable counterpart to [`SystemDescriptorBaseValueLow`].
pub type SystemDescriptorBaseValueLowMut<'value> = <SystemDescriptorBaseValueLow<'value> as Counterpart>::Mut;

/// Middle-low byte of a long-mode system-descriptor base value.
pub type SystemDescriptorBaseValueMiddle<'value> = Field<'value, 16, 23, u64>;

/// Mutable counterpart to [`SystemDescriptorBaseValueMiddle`].
pub type SystemDescriptorBaseValueMiddleMut<'value> = <SystemDescriptorBaseValueMiddle<'value> as Counterpart>::Mut;

/// Middle-high byte of a long-mode system-descriptor base value.
pub type SystemDescriptorBaseValueHigh<'value> = Field<'value, 24, 31, u64>;

/// Mutable counterpart to [`SystemDescriptorBaseValueHigh`].
pub type SystemDescriptorBaseValueHighMut<'value> = <SystemDescriptorBaseValueHigh<'value> as Counterpart>::Mut;

/// Upper thirty-two bits of a long-mode system-descriptor base value.
pub type SystemDescriptorBaseValueUpper<'value> = Field<'value, 32, 63, u64>;

/// Mutable counterpart to [`SystemDescriptorBaseValueUpper`].
pub type SystemDescriptorBaseValueUpperMut<'value> = <SystemDescriptorBaseValueUpper<'value> as Counterpart>::Mut;

/// Low sixteen bits of a long-mode system-descriptor limit value.
pub type SystemDescriptorLimitValueLow<'value> = Field<'value, 0, 15, u64>;

/// Mutable counterpart to [`SystemDescriptorLimitValueLow`].
pub type SystemDescriptorLimitValueLowMut<'value> = <SystemDescriptorLimitValueLow<'value> as Counterpart>::Mut;

/// High four bits of a long-mode system-descriptor limit value.
pub type SystemDescriptorLimitValueHigh<'value> = Field<'value, 16, 19, u64>;

/// Mutable counterpart to [`SystemDescriptorLimitValueHigh`].
pub type SystemDescriptorLimitValueHighMut<'value> = <SystemDescriptorLimitValueHigh<'value> as Counterpart>::Mut;

/// Low limit field in a long-mode TSS or LDT descriptor.
pub type LongModeSystemDescriptorLimitLow<'value> = Field<'value, 0, 15, u64>;

/// Mutable counterpart to [`LongModeSystemDescriptorLimitLow`].
pub type LongModeSystemDescriptorLimitLowMut<'value> = <LongModeSystemDescriptorLimitLow<'value> as Counterpart>::Mut;

/// Low base field in a long-mode TSS or LDT descriptor.
pub type LongModeSystemDescriptorBaseLow<'value> = Field<'value, 16, 31, u64>;

/// Mutable counterpart to [`LongModeSystemDescriptorBaseLow`].
pub type LongModeSystemDescriptorBaseLowMut<'value> = <LongModeSystemDescriptorBaseLow<'value> as Counterpart>::Mut;

/// Middle base field in a long-mode TSS or LDT descriptor.
pub type LongModeSystemDescriptorBaseMiddle<'value> = Field<'value, 32, 39, u64>;

/// Mutable counterpart to [`LongModeSystemDescriptorBaseMiddle`].
pub type LongModeSystemDescriptorBaseMiddleMut<'value> =
    <LongModeSystemDescriptorBaseMiddle<'value> as Counterpart>::Mut;

/// System descriptor type field in a long-mode TSS, LDT, or gate descriptor.
pub type LongModeSystemDescriptorType<'value> = Field<'value, 40, 43, u64>;

/// Mutable counterpart to [`LongModeSystemDescriptorType`].
pub type LongModeSystemDescriptorTypeMut<'value> = <LongModeSystemDescriptorType<'value> as Counterpart>::Mut;

/// Descriptor privilege field in a long-mode TSS, LDT, or gate descriptor.
pub type LongModeSystemDescriptorPrivilege<'value> = Field<'value, 45, 46, u64>;

/// Mutable counterpart to [`LongModeSystemDescriptorPrivilege`].
pub type LongModeSystemDescriptorPrivilegeMut<'value> = <LongModeSystemDescriptorPrivilege<'value> as Counterpart>::Mut;

/// Present bit in a long-mode TSS, LDT, or gate descriptor.
pub type LongModeSystemDescriptorPresent<'value> = Bit<'value, u64, 47>;

/// Mutable counterpart to [`LongModeSystemDescriptorPresent`].
pub type LongModeSystemDescriptorPresentMut<'value> = <LongModeSystemDescriptorPresent<'value> as Counterpart>::Mut;

/// High limit field in a long-mode TSS or LDT descriptor.
pub type LongModeSystemDescriptorLimitHigh<'value> = Field<'value, 48, 51, u64>;

/// Mutable counterpart to [`LongModeSystemDescriptorLimitHigh`].
pub type LongModeSystemDescriptorLimitHighMut<'value> = <LongModeSystemDescriptorLimitHigh<'value> as Counterpart>::Mut;

/// High byte of the low-word base in a long-mode TSS or LDT descriptor.
pub type LongModeSystemDescriptorBaseHigh<'value> = Field<'value, 56, 63, u64>;

/// Mutable counterpart to [`LongModeSystemDescriptorBaseHigh`].
pub type LongModeSystemDescriptorBaseHighMut<'value> = <LongModeSystemDescriptorBaseHigh<'value> as Counterpart>::Mut;

/// Upper thirty-two bits of the base in a long-mode TSS or LDT descriptor.
pub type LongModeSystemDescriptorBaseUpper<'value> = Field<'value, 0, 31, u64>;

/// Mutable counterpart to [`LongModeSystemDescriptorBaseUpper`].
pub type LongModeSystemDescriptorBaseUpperMut<'value> = <LongModeSystemDescriptorBaseUpper<'value> as Counterpart>::Mut;

/// Segment-class bit in a long-mode system descriptor.
///
/// The bit is cleared for TSS, LDT, and gate descriptors.
pub type LongModeSystemDescriptorClass<'value> = Bit<'value, u64, 44>;

/// Mutable counterpart to [`LongModeSystemDescriptorClass`].
pub type LongModeSystemDescriptorClassMut<'value> = <LongModeSystemDescriptorClass<'value> as Counterpart>::Mut;

/// Available-for-system-software bit in a long-mode TSS or LDT descriptor.
pub type LongModeSystemDescriptorAvailable<'value> = Bit<'value, u64, 52>;

/// Mutable counterpart to [`LongModeSystemDescriptorAvailable`].
pub type LongModeSystemDescriptorAvailableMut<'value> = <LongModeSystemDescriptorAvailable<'value> as Counterpart>::Mut;

/// Granularity bit in a long-mode TSS or LDT descriptor.
pub type LongModeSystemDescriptorGranularity<'value> = Bit<'value, u64, 55>;

/// Mutable counterpart to [`LongModeSystemDescriptorGranularity`].
pub type LongModeSystemDescriptorGranularityMut<'value> =
    <LongModeSystemDescriptorGranularity<'value> as Counterpart>::Mut;

/// Low sixty-four bits of a long-mode system-descriptor image.
pub type LongModeSystemDescriptorLow = Partitioned<0, 63, u128>;

/// High sixty-four bits of a long-mode system-descriptor image.
pub type LongModeSystemDescriptorHigh = Partitioned<64, 127, u128>;

/// TSS-specific name for [`LongModeSystemDescriptorLimitLow`].
pub type TssDescriptorLimitLow<'value> = LongModeSystemDescriptorLimitLow<'value>;

/// TSS-specific name for [`LongModeSystemDescriptorLimitLowMut`].
pub type TssDescriptorLimitLowMut<'value> = LongModeSystemDescriptorLimitLowMut<'value>;

/// TSS-specific name for [`LongModeSystemDescriptorBaseLow`].
pub type TssDescriptorBaseLow<'value> = LongModeSystemDescriptorBaseLow<'value>;

/// TSS-specific name for [`LongModeSystemDescriptorBaseLowMut`].
pub type TssDescriptorBaseLowMut<'value> = LongModeSystemDescriptorBaseLowMut<'value>;

/// TSS-specific name for [`LongModeSystemDescriptorBaseMiddle`].
pub type TssDescriptorBaseMiddle<'value> = LongModeSystemDescriptorBaseMiddle<'value>;

/// TSS-specific name for [`LongModeSystemDescriptorBaseMiddleMut`].
pub type TssDescriptorBaseMiddleMut<'value> = LongModeSystemDescriptorBaseMiddleMut<'value>;

/// TSS-specific name for [`LongModeSystemDescriptorType`].
pub type TssDescriptorType<'value> = LongModeSystemDescriptorType<'value>;

/// TSS-specific name for [`LongModeSystemDescriptorTypeMut`].
pub type TssDescriptorTypeMut<'value> = LongModeSystemDescriptorTypeMut<'value>;

/// TSS-specific name for [`LongModeSystemDescriptorPrivilege`].
pub type TssDescriptorPrivilege<'value> = LongModeSystemDescriptorPrivilege<'value>;

/// TSS-specific name for [`LongModeSystemDescriptorPrivilegeMut`].
pub type TssDescriptorPrivilegeMut<'value> = LongModeSystemDescriptorPrivilegeMut<'value>;

/// TSS-specific name for [`LongModeSystemDescriptorPresent`].
pub type TssDescriptorPresent<'value> = LongModeSystemDescriptorPresent<'value>;

/// TSS-specific name for [`LongModeSystemDescriptorPresentMut`].
pub type TssDescriptorPresentMut<'value> = LongModeSystemDescriptorPresentMut<'value>;

/// TSS-specific name for [`LongModeSystemDescriptorLimitHigh`].
pub type TssDescriptorLimitHigh<'value> = LongModeSystemDescriptorLimitHigh<'value>;

/// TSS-specific name for [`LongModeSystemDescriptorLimitHighMut`].
pub type TssDescriptorLimitHighMut<'value> = LongModeSystemDescriptorLimitHighMut<'value>;

/// TSS-specific name for [`LongModeSystemDescriptorBaseHigh`].
pub type TssDescriptorBaseHigh<'value> = LongModeSystemDescriptorBaseHigh<'value>;

/// TSS-specific name for [`LongModeSystemDescriptorBaseHighMut`].
pub type TssDescriptorBaseHighMut<'value> = LongModeSystemDescriptorBaseHighMut<'value>;

/// TSS-specific name for [`LongModeSystemDescriptorBaseUpper`].
pub type TssDescriptorBaseUpper<'value> = LongModeSystemDescriptorBaseUpper<'value>;

/// TSS-specific name for [`LongModeSystemDescriptorBaseUpperMut`].
pub type TssDescriptorBaseUpperMut<'value> = LongModeSystemDescriptorBaseUpperMut<'value>;

/// TSS-specific name for [`LongModeSystemDescriptorLow`].
pub type TssDescriptorLow = LongModeSystemDescriptorLow;

/// TSS-specific name for [`LongModeSystemDescriptorHigh`].
pub type TssDescriptorHigh = LongModeSystemDescriptorHigh;

/// A raw 128-bit long-mode TSS or LDT descriptor image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, IntoBytes, Immutable)]
#[repr(C)]
// NOTE(invariant): The two partitions form one complete sixteen-byte architectural
// system-descriptor image. Every 128-bit image remains representable.
pub struct RawLongModeSystemDescriptor(LongModeSystemDescriptorLow, LongModeSystemDescriptorHigh);

impl RawLongModeSystemDescriptor {
    /// Constructs a raw descriptor image.
    #[inline]
    #[must_use]
    pub const fn new(low: u64, high: u64) -> Self {
        Self(
            LongModeSystemDescriptorLow::raw(low),
            LongModeSystemDescriptorHigh::raw(high),
        )
    }

    /// Returns the low descriptor word.
    #[inline]
    #[must_use]
    pub const fn low(&self) -> &LongModeSystemDescriptorLow {
        let &Self(ref low, ..) = self;

        low
    }

    /// Returns mutable access to the low descriptor word.
    #[inline]
    pub const fn low_mut(&mut self) -> &mut LongModeSystemDescriptorLow {
        let &mut Self(ref mut low, ..) = self;

        low
    }

    /// Returns the high descriptor word.
    #[inline]
    #[must_use]
    pub const fn high(&self) -> &LongModeSystemDescriptorHigh {
        let &Self(_, ref high) = self;

        high
    }

    /// Returns mutable access to the high descriptor word.
    #[inline]
    pub const fn high_mut(&mut self) -> &mut LongModeSystemDescriptorHigh {
        let &mut Self(_, ref mut high) = self;

        high
    }

    /// Returns the low limit field.
    #[inline]
    #[must_use]
    pub const fn limit_low(&self) -> LongModeSystemDescriptorLimitLow<'_> {
        let &Self(ref low, ..) = self;

        LongModeSystemDescriptorLimitLow::wrap(low.value())
    }

    /// Returns mutable access to the low limit field.
    #[inline]
    pub const fn limit_low_mut(&mut self) -> LongModeSystemDescriptorLimitLowMut<'_> {
        let &mut Self(ref mut low, ..) = self;

        LongModeSystemDescriptorLimitLowMut::wrap(low.value_mut())
    }

    /// Returns the low base field.
    #[inline]
    #[must_use]
    pub const fn base_low(&self) -> LongModeSystemDescriptorBaseLow<'_> {
        let &Self(ref low, ..) = self;

        LongModeSystemDescriptorBaseLow::wrap(low.value())
    }

    /// Returns mutable access to the low base field.
    #[inline]
    pub const fn base_low_mut(&mut self) -> LongModeSystemDescriptorBaseLowMut<'_> {
        let &mut Self(ref mut low, ..) = self;

        LongModeSystemDescriptorBaseLowMut::wrap(low.value_mut())
    }

    /// Returns the middle base field.
    #[inline]
    #[must_use]
    pub const fn base_middle(&self) -> LongModeSystemDescriptorBaseMiddle<'_> {
        let &Self(ref low, ..) = self;

        LongModeSystemDescriptorBaseMiddle::wrap(low.value())
    }

    /// Returns mutable access to the middle base field.
    #[inline]
    pub const fn base_middle_mut(&mut self) -> LongModeSystemDescriptorBaseMiddleMut<'_> {
        let &mut Self(ref mut low, ..) = self;

        LongModeSystemDescriptorBaseMiddleMut::wrap(low.value_mut())
    }

    /// Returns the system-descriptor type field.
    #[inline]
    #[must_use]
    pub const fn descriptor_type(&self) -> LongModeSystemDescriptorType<'_> {
        let &Self(ref low, ..) = self;

        LongModeSystemDescriptorType::wrap(low.value())
    }

    /// Returns mutable access to the system-descriptor type field.
    #[inline]
    pub const fn descriptor_type_mut(&mut self) -> LongModeSystemDescriptorTypeMut<'_> {
        let &mut Self(ref mut low, ..) = self;

        LongModeSystemDescriptorTypeMut::wrap(low.value_mut())
    }

    /// Returns the descriptor privilege field.
    #[inline]
    #[must_use]
    pub const fn privilege(&self) -> LongModeSystemDescriptorPrivilege<'_> {
        let &Self(ref low, ..) = self;

        LongModeSystemDescriptorPrivilege::wrap(low.value())
    }

    /// Returns mutable access to the descriptor privilege field.
    #[inline]
    pub const fn privilege_mut(&mut self) -> LongModeSystemDescriptorPrivilegeMut<'_> {
        let &mut Self(ref mut low, ..) = self;

        LongModeSystemDescriptorPrivilegeMut::wrap(low.value_mut())
    }

    /// Returns the descriptor-present bit.
    #[inline]
    #[must_use]
    pub const fn present(&self) -> LongModeSystemDescriptorPresent<'_> {
        let &Self(ref low, ..) = self;

        LongModeSystemDescriptorPresent::wrap(low.value())
    }

    /// Returns mutable access to the descriptor-present bit.
    #[inline]
    pub const fn present_mut(&mut self) -> LongModeSystemDescriptorPresentMut<'_> {
        let &mut Self(ref mut low, ..) = self;

        LongModeSystemDescriptorPresentMut::wrap(low.value_mut())
    }

    /// Returns the high limit field.
    #[inline]
    #[must_use]
    pub const fn limit_high(&self) -> LongModeSystemDescriptorLimitHigh<'_> {
        let &Self(ref low, ..) = self;

        LongModeSystemDescriptorLimitHigh::wrap(low.value())
    }

    /// Returns mutable access to the high limit field.
    #[inline]
    pub const fn limit_high_mut(&mut self) -> LongModeSystemDescriptorLimitHighMut<'_> {
        let &mut Self(ref mut low, ..) = self;

        LongModeSystemDescriptorLimitHighMut::wrap(low.value_mut())
    }

    /// Returns the high byte of the low-word base.
    #[inline]
    #[must_use]
    pub const fn base_high(&self) -> LongModeSystemDescriptorBaseHigh<'_> {
        let &Self(ref low, ..) = self;

        LongModeSystemDescriptorBaseHigh::wrap(low.value())
    }

    /// Returns mutable access to the high byte of the low-word base.
    #[inline]
    pub const fn base_high_mut(&mut self) -> LongModeSystemDescriptorBaseHighMut<'_> {
        let &mut Self(ref mut low, ..) = self;

        LongModeSystemDescriptorBaseHighMut::wrap(low.value_mut())
    }

    /// Returns the upper thirty-two bits of the base.
    #[inline]
    #[must_use]
    pub const fn base_upper(&self) -> LongModeSystemDescriptorBaseUpper<'_> {
        let &Self(_, ref high) = self;

        LongModeSystemDescriptorBaseUpper::wrap(high.value())
    }

    /// Returns mutable access to the upper thirty-two bits of the base.
    #[inline]
    pub const fn base_upper_mut(&mut self) -> LongModeSystemDescriptorBaseUpperMut<'_> {
        let &mut Self(_, ref mut high) = self;

        LongModeSystemDescriptorBaseUpperMut::wrap(high.value_mut())
    }

    /// Returns the segment-class bit.
    #[inline]
    #[must_use]
    pub const fn class(&self) -> LongModeSystemDescriptorClass<'_> {
        let &Self(ref low, ..) = self;

        LongModeSystemDescriptorClass::wrap(low.value())
    }

    /// Returns mutable access to the segment-class bit.
    #[inline]
    pub const fn class_mut(&mut self) -> LongModeSystemDescriptorClassMut<'_> {
        let &mut Self(ref mut low, ..) = self;

        LongModeSystemDescriptorClassMut::wrap(low.value_mut())
    }

    /// Returns the available-for-system-software bit.
    #[inline]
    #[must_use]
    pub const fn available(&self) -> LongModeSystemDescriptorAvailable<'_> {
        let &Self(ref low, ..) = self;

        LongModeSystemDescriptorAvailable::wrap(low.value())
    }

    /// Returns mutable access to the available-for-system-software bit.
    #[inline]
    pub const fn available_mut(&mut self) -> LongModeSystemDescriptorAvailableMut<'_> {
        let &mut Self(ref mut low, ..) = self;

        LongModeSystemDescriptorAvailableMut::wrap(low.value_mut())
    }

    /// Returns the limit-granularity bit.
    #[inline]
    #[must_use]
    pub const fn granularity(&self) -> LongModeSystemDescriptorGranularity<'_> {
        let &Self(ref low, ..) = self;

        LongModeSystemDescriptorGranularity::wrap(low.value())
    }

    /// Returns mutable access to the limit-granularity bit.
    #[inline]
    pub const fn granularity_mut(&mut self) -> LongModeSystemDescriptorGranularityMut<'_> {
        let &mut Self(ref mut low, ..) = self;

        LongModeSystemDescriptorGranularityMut::wrap(low.value_mut())
    }

    /// Returns the base bits without checking canonicality.
    #[inline]
    #[must_use]
    pub const fn base(&self) -> u64 {
        let low = self.base_low().const_value();
        let middle = self.base_middle().const_value();
        let high = self.base_high().const_value();
        let upper = self.base_upper().const_value();
        let mut target_value = u64::MIN;

        SystemDescriptorBaseValueLowMut::wrap(&mut target_value).const_merge(low);
        SystemDescriptorBaseValueMiddleMut::wrap(&mut target_value).const_merge(middle);
        SystemDescriptorBaseValueHighMut::wrap(&mut target_value).const_merge(high);
        SystemDescriptorBaseValueUpperMut::wrap(&mut target_value).const_merge(upper);

        target_value
    }

    /// Returns the encoded limit without applying its granularity.
    #[inline]
    #[must_use]
    pub const fn limit(&self) -> u64 {
        let low = self.limit_low().const_value();
        let high = self.limit_high().const_value();
        let mut target_value = u64::MIN;

        SystemDescriptorLimitValueLowMut::wrap(&mut target_value).const_merge(low);
        SystemDescriptorLimitValueHighMut::wrap(&mut target_value).const_merge(high);

        target_value
    }
}

/// A raw 128-bit long-mode task-state-segment descriptor image.
pub type RawLongModeTssDescriptor = RawLongModeSystemDescriptor;

/// A raw 128-bit long-mode local-descriptor-table descriptor image.
pub type RawLongModeLdtDescriptor = RawLongModeSystemDescriptor;

/// An available ring-zero long-mode task-state-segment descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The raw image describes a present available TSS at ring zero
// with a canonical base and a limit equal to the supplied task-state area.
pub struct LongModeTssDescriptor(RawLongModeTssDescriptor);

impl LongModeTssDescriptor {
    /// Returns the low limit field.
    #[inline]
    #[must_use]
    pub const fn limit_low(&self) -> TssDescriptorLimitLow<'_> {
        let &Self(ref raw) = self;

        raw.limit_low()
    }

    /// Returns the low base field.
    #[inline]
    #[must_use]
    pub const fn base_low(&self) -> TssDescriptorBaseLow<'_> {
        let &Self(ref raw) = self;

        raw.base_low()
    }

    /// Returns the middle base field.
    #[inline]
    #[must_use]
    pub const fn base_middle(&self) -> TssDescriptorBaseMiddle<'_> {
        let &Self(ref raw) = self;

        raw.base_middle()
    }

    /// Returns the system-descriptor type field.
    #[inline]
    #[must_use]
    pub const fn descriptor_type(&self) -> TssDescriptorType<'_> {
        let &Self(ref raw) = self;

        raw.descriptor_type()
    }

    /// Returns the descriptor privilege field.
    #[inline]
    #[must_use]
    pub const fn privilege(&self) -> TssDescriptorPrivilege<'_> {
        let &Self(ref raw) = self;

        raw.privilege()
    }

    /// Returns the descriptor-present bit.
    #[inline]
    #[must_use]
    pub const fn present(&self) -> TssDescriptorPresent<'_> {
        let &Self(ref raw) = self;

        raw.present()
    }

    /// Returns the high limit field.
    #[inline]
    #[must_use]
    pub const fn limit_high(&self) -> TssDescriptorLimitHigh<'_> {
        let &Self(ref raw) = self;

        raw.limit_high()
    }

    /// Returns the high byte of the low-word base.
    #[inline]
    #[must_use]
    pub const fn base_high(&self) -> TssDescriptorBaseHigh<'_> {
        let &Self(ref raw) = self;

        raw.base_high()
    }

    /// Returns the upper thirty-two bits of the base.
    #[inline]
    #[must_use]
    pub const fn base_upper(&self) -> TssDescriptorBaseUpper<'_> {
        let &Self(ref raw) = self;

        raw.base_upper()
    }

    /// Constructs an available ring-zero descriptor for a task-state segment.
    ///
    /// The descriptor extent is derived from the task-state area so the
    /// architectural limit includes exactly the trailing I/O permission-map bytes.
    /// Returns `None` when that extent exceeds the twenty-bit descriptor limit.
    #[inline]
    #[must_use]
    pub const fn new<const N: usize>(base: La, state: &RawTaskStateSegment<N>) -> Option<Self> {
        let limit = state.limit();

        match limit {
            Some(limit) => {
                let target_value = base.bits();
                let base_low = SystemDescriptorBaseValueLow::wrap(&target_value).const_value();
                let base_middle = SystemDescriptorBaseValueMiddle::wrap(&target_value).const_value();
                let base_high = SystemDescriptorBaseValueHigh::wrap(&target_value).const_value();
                let base_upper = SystemDescriptorBaseValueUpper::wrap(&target_value).const_value();
                let limit_low = SystemDescriptorLimitValueLow::wrap(&limit).const_value();
                let limit_high = SystemDescriptorLimitValueHigh::wrap(&limit).const_value();
                let descriptor_kind = SystemDescriptorType::TssAvailable32 as u8;
                let mut low = u64::MIN;
                let mut low_limit = TssDescriptorLimitLowMut::wrap(&mut low);

                low_limit.const_merge(limit_low);

                let mut low_base = TssDescriptorBaseLowMut::wrap(&mut low);

                low_base.const_merge(base_low);

                let mut middle_base = TssDescriptorBaseMiddleMut::wrap(&mut low);

                middle_base.const_merge(base_middle);

                let mut descriptor_type = TssDescriptorTypeMut::wrap(&mut low);

                descriptor_type.const_merge(descriptor_kind);

                let mut descriptor_privilege = TssDescriptorPrivilegeMut::wrap(&mut low);

                descriptor_privilege.const_merge(PrivilegeLevel::Ring0.raw());

                let mut present = TssDescriptorPresentMut::wrap(&mut low);

                present.const_set(State::Set);

                let mut high_limit = TssDescriptorLimitHighMut::wrap(&mut low);

                high_limit.const_merge(limit_high);

                let mut high_base = TssDescriptorBaseHighMut::wrap(&mut low);

                high_base.const_merge(base_high);

                let mut high = u64::MIN;

                let mut upper_base = TssDescriptorBaseUpperMut::wrap(&mut high);

                upper_base.const_merge(base_upper);

                let raw = RawLongModeSystemDescriptor::new(low, high);

                Some(Self(raw))
            },
            None => None,
        }
    }

    /// Lowers this checked descriptor into its raw image.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> RawLongModeTssDescriptor {
        let Self(raw) = self;

        raw
    }
}

/// A present long-mode local-descriptor-table descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The raw image names a present LDT with a canonical base, the requested DPL,
// and an exact byte extent representable by the architectural limit and granularity fields.
pub struct LongModeLdtDescriptor(RawLongModeSystemDescriptor);

impl LongModeLdtDescriptor {
    /// Constructs an LDT descriptor for an exact table byte length.
    ///
    /// The requested length is encoded with byte granularity when it is at most one mebibyte.
    /// Larger lengths must be an exact multiple of four kibibytes and may not exceed four
    /// gibibytes. Returns `None` when the requested length cannot be represented by the
    /// descriptor's twenty-bit limit and granularity fields.
    #[inline]
    #[must_use]
    pub const fn new(base: La, byte_length: NonZero<u64>, privilege: PrivilegeLevel) -> Option<Self> {
        const MAX_BYTE_LIMIT: u64 = 0x000f_ffff;
        const FIRST_PAGE_LIMIT: u64 = MAX_BYTE_LIMIT + 1;
        const MAX_EFFECTIVE_LIMIT: u64 = u32::MAX as u64;
        const PAGE_BYTES: u64 = 4096;
        const PAGE_LAST_BYTE: u64 = PAGE_BYTES - 1;

        let target_limit = byte_length.get() - 1;
        let encoded_limit = match target_limit {
            0..=MAX_BYTE_LIMIT => Some((target_limit, State::Cleared)),
            FIRST_PAGE_LIMIT..=MAX_EFFECTIVE_LIMIT if target_limit % PAGE_BYTES == PAGE_LAST_BYTE => {
                Some((target_limit / PAGE_BYTES, State::Set))
            },
            _ => None,
        };

        match encoded_limit {
            Some((encoded_limit, granularity)) => {
                let target_value = base.bits();
                let base_low = SystemDescriptorBaseValueLow::wrap(&target_value).const_value();
                let base_middle = SystemDescriptorBaseValueMiddle::wrap(&target_value).const_value();
                let base_high = SystemDescriptorBaseValueHigh::wrap(&target_value).const_value();
                let base_upper = SystemDescriptorBaseValueUpper::wrap(&target_value).const_value();
                let limit_low = SystemDescriptorLimitValueLow::wrap(&encoded_limit).const_value();
                let limit_high = SystemDescriptorLimitValueHigh::wrap(&encoded_limit).const_value();
                let mut low = u64::MIN;

                LongModeSystemDescriptorLimitLowMut::wrap(&mut low).const_merge(limit_low);
                LongModeSystemDescriptorBaseLowMut::wrap(&mut low).const_merge(base_low);
                LongModeSystemDescriptorBaseMiddleMut::wrap(&mut low).const_merge(base_middle);
                LongModeSystemDescriptorTypeMut::wrap(&mut low).const_merge(SystemDescriptorType::Ldt as u8);
                LongModeSystemDescriptorClassMut::wrap(&mut low).const_set(State::Cleared);
                LongModeSystemDescriptorPrivilegeMut::wrap(&mut low).const_merge(privilege.raw());
                LongModeSystemDescriptorPresentMut::wrap(&mut low).const_set(State::Set);
                LongModeSystemDescriptorLimitHighMut::wrap(&mut low).const_merge(limit_high);
                LongModeSystemDescriptorBaseHighMut::wrap(&mut low).const_merge(base_high);
                LongModeSystemDescriptorAvailableMut::wrap(&mut low).const_set(State::Cleared);
                LongModeSystemDescriptorGranularityMut::wrap(&mut low).const_set(granularity);

                let mut high = u64::MIN;

                LongModeSystemDescriptorBaseUpperMut::wrap(&mut high).const_merge(base_upper);

                let raw = RawLongModeSystemDescriptor::new(low, high);

                Some(Self(raw))
            },
            None => None,
        }
    }

    /// Lowers this checked descriptor into its raw image.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> RawLongModeLdtDescriptor {
        let Self(raw) = self;

        raw
    }
}

impl<T> DescriptorTableRegister<T>
where
    T: DescriptorTable,
{
    /// Constructs a register from a canonical base and architectural limit with a bias of 1.
    #[inline]
    #[must_use]
    pub const fn new(base: La, limit: u16) -> Self {
        Self {
            base,
            limit,
            table: PhantomData,
        }
    }

    /// Fits a descriptor-table register to a canonical base and table length.
    ///
    /// Returns `None` when the architectural limit derived from
    /// `byte_length` does not fit in the register's 16-bit limit field.
    #[inline]
    #[must_use]
    pub const fn fit(base: La, byte_length: NonZero<u32>) -> Option<Self> {
        const MAX_LIMIT: u32 = u16::MAX as u32;

        // NOTE: NonZero<u32> guarantees a byte length of at least one,
        // so converting the architectural one-biased length into a limit cannot underflow.
        let target_value = byte_length.get() - 1;

        match target_value {
            0..=MAX_LIMIT => {
                let limit = target_value as u16;

                Some(Self::new(base, limit))
            },
            _ => None,
        }
    }

    /// Returns the canonical table base.
    #[inline]
    #[must_use]
    pub const fn base(&self) -> La {
        let &Self { base, .. } = self;

        base
    }

    /// Returns the architectural byte limit with a bias of 1.
    #[inline]
    #[must_use]
    pub const fn limit(&self) -> u16 {
        let &Self { limit, .. } = self;

        limit
    }
}

#[cfg(test)]
mod tests {
    use core::{mem, num::NonZero};

    use nekor_bitwise::prelude::State;
    use zerocopy::IntoBytes;

    use super::{GdtRegister, LongModeTssDescriptor, RawLongModeTssDescriptor};
    use crate::x86_64::{
        paging::{La, La48},
        task::TaskStateSegment,
    };

    #[test]
    fn descriptor_table_register_tracks_architectural_limit() {
        let base = La::new::<La48>(0x6000).expect("test table base must be canonical");
        let one = NonZero::<u32>::new(1).expect("one is nonzero");
        let maximum = NonZero::<u32>::new(0x0001_0000).expect("maximum table length is nonzero");
        let oversized = NonZero::<u32>::new(0x0001_0001).expect("oversized table length is nonzero");
        let smallest = GdtRegister::fit(base, one).expect("one byte table must fit");
        let largest = GdtRegister::fit(base, maximum).expect("maximum table must fit");

        assert_eq!(smallest.base(), base);
        assert_eq!(smallest.limit(), 0);
        assert_eq!(largest.limit(), u16::MAX);
        assert!(GdtRegister::fit(base, oversized).is_none());
    }

    #[test]
    fn long_mode_tss_descriptor_matches_the_architectural_layout() {
        let base = La::new::<La48>(0x1234_5678).expect("test TSS base must be canonical");
        let state = TaskStateSegment::<0>::new();
        let raw = state.raw();
        let descriptor = LongModeTssDescriptor::new(base, &raw)
            .expect("bare TSS extent must fit the architectural descriptor limit");

        assert_eq!(mem::size_of::<LongModeTssDescriptor>(), 16);
        assert_eq!(mem::align_of::<LongModeTssDescriptor>(), 8);
        assert_eq!(descriptor.limit_low().const_value(), 0x67);
        assert_eq!(descriptor.limit_high().const_value(), 0);
        assert_eq!(descriptor.base_low().const_value(), 0x5678);
        assert_eq!(descriptor.base_middle().const_value(), 0x34);
        assert_eq!(descriptor.base_high().const_value(), 0x12);
        assert_eq!(descriptor.base_upper().const_value(), 0);
        assert_eq!(descriptor.descriptor_type().const_value(), 9);
        assert_eq!(descriptor.privilege().const_value(), 0);
        assert_eq!(descriptor.present().const_state(), State::Set);
        let raw = descriptor.raw();

        assert_eq!(
            raw.as_bytes(),
            [0x67, 0, 0x78, 0x56, 0x34, 0x89, 0, 0x12, 0, 0, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn raw_tss_descriptor_exposes_named_mutable_fields() {
        let mut descriptor = RawLongModeTssDescriptor::new(u64::MIN, u64::MIN);

        descriptor.limit_low_mut().const_merge(0x1234);
        descriptor.base_low_mut().const_merge(0x5678);
        descriptor.base_middle_mut().const_merge(0x9a);
        descriptor.descriptor_type_mut().const_merge(0x9);
        descriptor.privilege_mut().const_merge(0x3);
        descriptor.present_mut().const_set(State::Set);
        descriptor.limit_high_mut().const_merge(0xb);
        descriptor.base_high_mut().const_merge(0xcd);
        descriptor.base_upper_mut().const_merge(0x1234_5678);

        assert_eq!(descriptor.limit_low().const_value(), 0x1234);
        assert_eq!(descriptor.base_low().const_value(), 0x5678);
        assert_eq!(descriptor.base_middle().const_value(), 0x9a);
        assert_eq!(descriptor.descriptor_type().const_value(), 0x9);
        assert_eq!(descriptor.privilege().const_value(), 0x3);
        assert_eq!(descriptor.present().const_state(), State::Set);
        assert_eq!(descriptor.limit_high().const_value(), 0xb);
        assert_eq!(descriptor.base_high().const_value(), 0xcd);
        assert_eq!(descriptor.base_upper().const_value(), 0x1234_5678);
    }

    #[test]
    fn long_mode_tss_descriptor_includes_the_io_map_extent() {
        let base = La::new::<La48>(0x1234_5678).expect("test TSS base must be canonical");
        let state = TaskStateSegment::<3>::new();
        let raw = state.raw();
        let descriptor =
            LongModeTssDescriptor::new(base, &raw).expect("small I/O map must fit the architectural descriptor limit");

        assert_eq!(&descriptor.raw().as_bytes()[..2], [0x6a, 0]);
    }
}

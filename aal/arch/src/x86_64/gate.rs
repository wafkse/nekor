//! Gate descriptors for `x86-64` long mode.

use nekor_aal_agnostic::partitioned::Partitioned;
use nekor_bitwise::prelude::{Counterpart, Field, State};
use zerocopy::{Immutable, IntoBytes};

use crate::{
    x86::{descriptor::SystemDescriptorType, gate::GateType32, privilege::PrivilegeLevel, segmentation::CodeSegment},
    x86_64::{
        descriptor::{
            LongModeSystemDescriptorClass, LongModeSystemDescriptorClassMut, LongModeSystemDescriptorPresent,
            LongModeSystemDescriptorPresentMut, LongModeSystemDescriptorPrivilege,
            LongModeSystemDescriptorPrivilegeMut, LongModeSystemDescriptorType, LongModeSystemDescriptorTypeMut,
        },
        paging::La,
    },
};

/// An interrupt [`GateType32`] intended for *Long Mode* code execution.
///
/// This *Gate Type* is exclusively supported in a *Long Mode* context, any
/// other type is disallowed.
///
/// # Effects on Interrupt Service Routine Dispatch
///
/// The only effects depend on whether it is an *Interrupt Gate* or *Trap Gate*.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
#[non_exhaustive]
pub enum GateType64 {
    /// A *Long Mode* 64-bit interrupt mode.
    ///
    /// # Remarks
    ///
    /// This value is inherited from a [`GateType32::Interrupt`].
    Interrupt = GateType32::Interrupt as u8,

    /// A *Long Mode* 64-bit interrupt trap mode.
    ///
    /// # Remarks
    ///
    /// This value is inherited from a [`GateType32::Trap`].
    Trap = GateType32::Trap as u8,
}

/// Low sixteen bits of a call-gate target offset value.
pub type LongModeCallGateOffsetValueLow<'value> = Field<'value, 0, 15, u64>;

/// Mutable counterpart to [`LongModeCallGateOffsetValueLow`].
pub type LongModeCallGateOffsetValueLowMut<'value> = <LongModeCallGateOffsetValueLow<'value> as Counterpart>::Mut;

/// Middle sixteen bits of a call-gate target offset value.
pub type LongModeCallGateOffsetValueMiddle<'value> = Field<'value, 16, 31, u64>;

/// Mutable counterpart to [`LongModeCallGateOffsetValueMiddle`].
pub type LongModeCallGateOffsetValueMiddleMut<'value> = <LongModeCallGateOffsetValueMiddle<'value> as Counterpart>::Mut;

/// High thirty-two bits of a call-gate target offset value.
pub type LongModeCallGateOffsetValueHigh<'value> = Field<'value, 32, 63, u64>;

/// Mutable counterpart to [`LongModeCallGateOffsetValueHigh`].
pub type LongModeCallGateOffsetValueHighMut<'value> = <LongModeCallGateOffsetValueHigh<'value> as Counterpart>::Mut;

/// Low sixty-four bits of a long-mode call-gate descriptor image.
pub type LongModeCallGateLow = Partitioned<0, 63, u128>;

/// High sixty-four bits of a long-mode call-gate descriptor image.
pub type LongModeCallGateHigh = Partitioned<64, 127, u128>;

/// Low sixteen bits of a long-mode call-gate target offset.
pub type LongModeCallGateOffsetLow<'value> = Field<'value, 0, 15, u64>;

/// Mutable counterpart to [`LongModeCallGateOffsetLow`].
pub type LongModeCallGateOffsetLowMut<'value> = <LongModeCallGateOffsetLow<'value> as Counterpart>::Mut;

/// Target code-segment selector in a long-mode call-gate descriptor.
pub type LongModeCallGateSelector<'value> = Field<'value, 16, 31, u64>;

/// Mutable counterpart to [`LongModeCallGateSelector`].
pub type LongModeCallGateSelectorMut<'value> = <LongModeCallGateSelector<'value> as Counterpart>::Mut;

/// Reserved bits 32 through 39 in a long-mode call-gate descriptor.
pub type LongModeCallGateReserved32_39<'value> = Field<'value, 32, 39, u64>;

/// Mutable counterpart to [`LongModeCallGateReserved32_39`].
pub type LongModeCallGateReserved32_39Mut<'value> = <LongModeCallGateReserved32_39<'value> as Counterpart>::Mut;

/// Middle sixteen bits of a long-mode call-gate target offset.
pub type LongModeCallGateOffsetMiddle<'value> = Field<'value, 48, 63, u64>;

/// Mutable counterpart to [`LongModeCallGateOffsetMiddle`].
pub type LongModeCallGateOffsetMiddleMut<'value> = <LongModeCallGateOffsetMiddle<'value> as Counterpart>::Mut;

/// High thirty-two bits of a long-mode call-gate target offset.
pub type LongModeCallGateOffsetHigh<'value> = Field<'value, 0, 31, u64>;

/// Mutable counterpart to [`LongModeCallGateOffsetHigh`].
pub type LongModeCallGateOffsetHighMut<'value> = <LongModeCallGateOffsetHigh<'value> as Counterpart>::Mut;

/// Reserved high dword in a long-mode call-gate descriptor.
pub type LongModeCallGateReservedHigh<'value> = Field<'value, 32, 63, u64>;

/// Mutable counterpart to [`LongModeCallGateReservedHigh`].
pub type LongModeCallGateReservedHighMut<'value> = <LongModeCallGateReservedHigh<'value> as Counterpart>::Mut;

/// A raw 128-bit long-mode call-gate descriptor image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, IntoBytes, Immutable)]
#[repr(C, align(8))]
// NOTE(invariant): The two partitions form one exact sixteen-byte call-gate image and preserve
// every raw bit pattern, including reserved bits.
pub struct RawLongModeCallGateDescriptor(LongModeCallGateLow, LongModeCallGateHigh);

impl RawLongModeCallGateDescriptor {
    /// Constructs a raw call-gate descriptor image.
    #[inline]
    #[must_use]
    pub const fn new(low: u64, high: u64) -> Self {
        Self(LongModeCallGateLow::raw(low), LongModeCallGateHigh::raw(high))
    }

    /// Returns the low descriptor word.
    #[inline]
    #[must_use]
    pub const fn low(&self) -> &LongModeCallGateLow {
        let &Self(ref low, ..) = self;

        low
    }

    /// Returns mutable access to the low descriptor word.
    #[inline]
    pub const fn low_mut(&mut self) -> &mut LongModeCallGateLow {
        let &mut Self(ref mut low, ..) = self;

        low
    }

    /// Returns the high descriptor word.
    #[inline]
    #[must_use]
    pub const fn high(&self) -> &LongModeCallGateHigh {
        let &Self(_, ref high) = self;

        high
    }

    /// Returns mutable access to the high descriptor word.
    #[inline]
    pub const fn high_mut(&mut self) -> &mut LongModeCallGateHigh {
        let &mut Self(_, ref mut high) = self;

        high
    }

    /// Returns the low target-offset field.
    #[inline]
    #[must_use]
    pub const fn offset_low(&self) -> LongModeCallGateOffsetLow<'_> {
        let &Self(ref low, ..) = self;

        LongModeCallGateOffsetLow::wrap(low.value())
    }

    /// Returns mutable access to the low target-offset field.
    #[inline]
    pub const fn offset_low_mut(&mut self) -> LongModeCallGateOffsetLowMut<'_> {
        let &mut Self(ref mut low, ..) = self;

        LongModeCallGateOffsetLowMut::wrap(low.value_mut())
    }

    /// Returns the target code-segment selector.
    #[inline]
    #[must_use]
    pub const fn selector(&self) -> LongModeCallGateSelector<'_> {
        let &Self(ref low, ..) = self;

        LongModeCallGateSelector::wrap(low.value())
    }

    /// Returns mutable access to the target code-segment selector.
    #[inline]
    pub const fn selector_mut(&mut self) -> LongModeCallGateSelectorMut<'_> {
        let &mut Self(ref mut low, ..) = self;

        LongModeCallGateSelectorMut::wrap(low.value_mut())
    }

    /// Returns reserved bits 32 through 39.
    #[inline]
    #[must_use]
    pub const fn reserved_32_39(&self) -> LongModeCallGateReserved32_39<'_> {
        let &Self(ref low, ..) = self;

        LongModeCallGateReserved32_39::wrap(low.value())
    }

    /// Returns mutable access to reserved bits 32 through 39.
    #[inline]
    pub const fn reserved_32_39_mut(&mut self) -> LongModeCallGateReserved32_39Mut<'_> {
        let &mut Self(ref mut low, ..) = self;

        LongModeCallGateReserved32_39Mut::wrap(low.value_mut())
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

    /// Returns the system-vs-code/data class bit.
    #[inline]
    #[must_use]
    pub const fn class(&self) -> LongModeSystemDescriptorClass<'_> {
        let &Self(ref low, ..) = self;

        LongModeSystemDescriptorClass::wrap(low.value())
    }

    /// Returns mutable access to the system-vs-code/data class bit.
    #[inline]
    pub const fn class_mut(&mut self) -> LongModeSystemDescriptorClassMut<'_> {
        let &mut Self(ref mut low, ..) = self;

        LongModeSystemDescriptorClassMut::wrap(low.value_mut())
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

    /// Returns the middle target-offset field.
    #[inline]
    #[must_use]
    pub const fn offset_middle(&self) -> LongModeCallGateOffsetMiddle<'_> {
        let &Self(ref low, ..) = self;

        LongModeCallGateOffsetMiddle::wrap(low.value())
    }

    /// Returns mutable access to the middle target-offset field.
    #[inline]
    pub const fn offset_middle_mut(&mut self) -> LongModeCallGateOffsetMiddleMut<'_> {
        let &mut Self(ref mut low, ..) = self;

        LongModeCallGateOffsetMiddleMut::wrap(low.value_mut())
    }

    /// Returns the high target-offset field.
    #[inline]
    #[must_use]
    pub const fn offset_high(&self) -> LongModeCallGateOffsetHigh<'_> {
        let &Self(_, ref high) = self;

        LongModeCallGateOffsetHigh::wrap(high.value())
    }

    /// Returns mutable access to the high target-offset field.
    #[inline]
    pub const fn offset_high_mut(&mut self) -> LongModeCallGateOffsetHighMut<'_> {
        let &mut Self(_, ref mut high) = self;

        LongModeCallGateOffsetHighMut::wrap(high.value_mut())
    }

    /// Returns the reserved high dword.
    #[inline]
    #[must_use]
    pub const fn reserved_high(&self) -> LongModeCallGateReservedHigh<'_> {
        let &Self(_, ref high) = self;

        LongModeCallGateReservedHigh::wrap(high.value())
    }

    /// Returns mutable access to the reserved high dword.
    #[inline]
    pub const fn reserved_high_mut(&mut self) -> LongModeCallGateReservedHighMut<'_> {
        let &mut Self(_, ref mut high) = self;

        LongModeCallGateReservedHighMut::wrap(high.value_mut())
    }
}

/// A present long-mode call gate targeting a checked 64-bit code segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The image encodes a present type-0xC gate with a canonical target offset and a
// selector validated against present 64-bit code semantics.
pub struct LongModeCallGateDescriptor(RawLongModeCallGateDescriptor);

impl LongModeCallGateDescriptor {
    /// Constructs a present call gate for a checked 64-bit code segment.
    ///
    /// The `target` must be canonical under the active LA48 or LA57 mode. [`La`] does not retain
    /// the mode marker used during construction. The caller must ensure the address was checked
    /// for the active mode.
    ///
    /// The [`CodeSegment`] supplies the target selector and proves that the supplied descriptor
    /// has present 64-bit code semantics. The descriptor-table owner must install that code
    /// descriptor and this call gate.
    #[inline]
    #[must_use]
    pub const fn new(target: La, code_segment: CodeSegment, privilege: PrivilegeLevel) -> Self {
        let target_value = target.bits();
        let offset_low = LongModeCallGateOffsetValueLow::wrap(&target_value).const_value();
        let offset_middle = LongModeCallGateOffsetValueMiddle::wrap(&target_value).const_value();
        let offset_high = LongModeCallGateOffsetValueHigh::wrap(&target_value).const_value();
        let mut low = u64::MIN;

        LongModeCallGateOffsetLowMut::wrap(&mut low).const_merge(offset_low);
        LongModeCallGateSelectorMut::wrap(&mut low).const_merge(code_segment.raw().raw());
        LongModeSystemDescriptorTypeMut::wrap(&mut low).const_merge(SystemDescriptorType::CallGate32 as u8);
        LongModeSystemDescriptorClassMut::wrap(&mut low).const_set(State::Cleared);
        LongModeSystemDescriptorPrivilegeMut::wrap(&mut low).const_merge(privilege.raw());
        LongModeSystemDescriptorPresentMut::wrap(&mut low).const_set(State::Set);
        LongModeCallGateOffsetMiddleMut::wrap(&mut low).const_merge(offset_middle);

        let mut high = u64::MIN;

        LongModeCallGateOffsetHighMut::wrap(&mut high).const_merge(offset_high);

        Self(RawLongModeCallGateDescriptor::new(low, high))
    }

    /// Lowers this checked descriptor into its raw image.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> RawLongModeCallGateDescriptor {
        let Self(target_value) = self;

        target_value
    }
}

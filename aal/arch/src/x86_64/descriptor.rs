//! Long-mode descriptor-table register state.

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

/// Low sixteen bits of a task-state base value.
type TssBaseValueLow<'value> = Field<'value, 0, 15, u64>;

/// Middle-low byte of a task-state base value.
type TssBaseValueMiddle<'value> = Field<'value, 16, 23, u64>;

/// Middle-high byte of a task-state base value.
type TssBaseValueHigh<'value> = Field<'value, 24, 31, u64>;

/// Upper thirty-two bits of a task-state base value.
type TssBaseValueUpper<'value> = Field<'value, 32, 63, u64>;

/// Low sixteen bits of a task-state limit value.
type TssLimitValueLow<'value> = Field<'value, 0, 15, u64>;

/// High four bits of a task-state limit value.
type TssLimitValueHigh<'value> = Field<'value, 16, 19, u64>;

/// Low limit field in a long-mode task-state descriptor.
pub type TssDescriptorLimitLow<'value> = Field<'value, 0, 15, u64>;

/// Mutable counterpart to [`TssDescriptorLimitLow`].
pub type TssDescriptorLimitLowMut<'value> = <TssDescriptorLimitLow<'value> as Counterpart>::Mut;

/// Low base field in a long-mode task-state descriptor.
pub type TssDescriptorBaseLow<'value> = Field<'value, 16, 31, u64>;

/// Mutable counterpart to [`TssDescriptorBaseLow`].
pub type TssDescriptorBaseLowMut<'value> = <TssDescriptorBaseLow<'value> as Counterpart>::Mut;

/// Middle base field in a long-mode task-state descriptor.
pub type TssDescriptorBaseMiddle<'value> = Field<'value, 32, 39, u64>;

/// Mutable counterpart to [`TssDescriptorBaseMiddle`].
pub type TssDescriptorBaseMiddleMut<'value> = <TssDescriptorBaseMiddle<'value> as Counterpart>::Mut;

/// System descriptor type field in a long-mode task-state descriptor.
pub type TssDescriptorType<'value> = Field<'value, 40, 43, u64>;

/// Mutable counterpart to [`TssDescriptorType`].
pub type TssDescriptorTypeMut<'value> = <TssDescriptorType<'value> as Counterpart>::Mut;

/// Descriptor privilege field in a long-mode task-state descriptor.
pub type TssDescriptorPrivilege<'value> = Field<'value, 45, 46, u64>;

/// Mutable counterpart to [`TssDescriptorPrivilege`].
pub type TssDescriptorPrivilegeMut<'value> = <TssDescriptorPrivilege<'value> as Counterpart>::Mut;

/// Present bit in a long-mode task-state descriptor.
pub type TssDescriptorPresent<'value> = Bit<'value, u64, 47>;

/// Mutable counterpart to [`TssDescriptorPresent`].
pub type TssDescriptorPresentMut<'value> = <TssDescriptorPresent<'value> as Counterpart>::Mut;

/// High limit field in a long-mode task-state descriptor.
pub type TssDescriptorLimitHigh<'value> = Field<'value, 48, 51, u64>;

/// Mutable counterpart to [`TssDescriptorLimitHigh`].
pub type TssDescriptorLimitHighMut<'value> = <TssDescriptorLimitHigh<'value> as Counterpart>::Mut;

/// High byte of the low-word task-state base.
pub type TssDescriptorBaseHigh<'value> = Field<'value, 56, 63, u64>;

/// Mutable counterpart to [`TssDescriptorBaseHigh`].
pub type TssDescriptorBaseHighMut<'value> = <TssDescriptorBaseHigh<'value> as Counterpart>::Mut;

/// Upper thirty-two bits of the task-state base.
pub type TssDescriptorBaseUpper<'value> = Field<'value, 0, 31, u64>;

/// Mutable counterpart to [`TssDescriptorBaseUpper`].
pub type TssDescriptorBaseUpperMut<'value> = <TssDescriptorBaseUpper<'value> as Counterpart>::Mut;

/// Low sixty-four bits of a long-mode task-state descriptor image.
pub type TssDescriptorLow = Partitioned<0, 63, u128>;

/// High sixty-four bits of a long-mode task-state descriptor image.
pub type TssDescriptorHigh = Partitioned<64, 127, u128>;

/// A raw 128-bit long-mode task-state-segment descriptor image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, IntoBytes, Immutable)]
#[repr(C)]
// NOTE(invariant): The two partitions form one complete sixteen-byte architectural
// system-descriptor image. Every 128-bit image remains representable.
pub struct RawLongModeTssDescriptor(TssDescriptorLow, TssDescriptorHigh);

impl RawLongModeTssDescriptor {
    /// Constructs a raw descriptor image.
    #[inline]
    #[must_use]
    pub const fn new(low: u64, high: u64) -> Self {
        Self(TssDescriptorLow::raw(low), TssDescriptorHigh::raw(high))
    }

    /// Returns the low descriptor word.
    #[inline]
    #[must_use]
    pub const fn low(&self) -> &TssDescriptorLow {
        let &Self(ref low, ..) = self;

        low
    }

    /// Returns mutable access to the low descriptor word.
    #[inline]
    pub const fn low_mut(&mut self) -> &mut TssDescriptorLow {
        let &mut Self(ref mut low, ..) = self;

        low
    }

    /// Returns the high descriptor word.
    #[inline]
    #[must_use]
    pub const fn high(&self) -> &TssDescriptorHigh {
        let &Self(_, ref high) = self;

        high
    }

    /// Returns mutable access to the high descriptor word.
    #[inline]
    pub const fn high_mut(&mut self) -> &mut TssDescriptorHigh {
        let &mut Self(_, ref mut high) = self;

        high
    }

    /// Returns the low limit field.
    #[inline]
    #[must_use]
    pub const fn limit_low(&self) -> TssDescriptorLimitLow<'_> {
        let &Self(ref low, ..) = self;

        TssDescriptorLimitLow::wrap(low.value())
    }

    /// Returns mutable access to the low limit field.
    #[inline]
    pub const fn limit_low_mut(&mut self) -> TssDescriptorLimitLowMut<'_> {
        let &mut Self(ref mut low, ..) = self;

        TssDescriptorLimitLowMut::wrap(low.value_mut())
    }

    /// Returns the low base field.
    #[inline]
    #[must_use]
    pub const fn base_low(&self) -> TssDescriptorBaseLow<'_> {
        let &Self(ref low, ..) = self;

        TssDescriptorBaseLow::wrap(low.value())
    }

    /// Returns mutable access to the low base field.
    #[inline]
    pub const fn base_low_mut(&mut self) -> TssDescriptorBaseLowMut<'_> {
        let &mut Self(ref mut low, ..) = self;

        TssDescriptorBaseLowMut::wrap(low.value_mut())
    }

    /// Returns the middle base field.
    #[inline]
    #[must_use]
    pub const fn base_middle(&self) -> TssDescriptorBaseMiddle<'_> {
        let &Self(ref low, ..) = self;

        TssDescriptorBaseMiddle::wrap(low.value())
    }

    /// Returns mutable access to the middle base field.
    #[inline]
    pub const fn base_middle_mut(&mut self) -> TssDescriptorBaseMiddleMut<'_> {
        let &mut Self(ref mut low, ..) = self;

        TssDescriptorBaseMiddleMut::wrap(low.value_mut())
    }

    /// Returns the system-descriptor type field.
    #[inline]
    #[must_use]
    pub const fn descriptor_type(&self) -> TssDescriptorType<'_> {
        let &Self(ref low, ..) = self;

        TssDescriptorType::wrap(low.value())
    }

    /// Returns mutable access to the system-descriptor type field.
    #[inline]
    pub const fn descriptor_type_mut(&mut self) -> TssDescriptorTypeMut<'_> {
        let &mut Self(ref mut low, ..) = self;

        TssDescriptorTypeMut::wrap(low.value_mut())
    }

    /// Returns the descriptor privilege field.
    #[inline]
    #[must_use]
    pub const fn privilege(&self) -> TssDescriptorPrivilege<'_> {
        let &Self(ref low, ..) = self;

        TssDescriptorPrivilege::wrap(low.value())
    }

    /// Returns mutable access to the descriptor privilege field.
    #[inline]
    pub const fn privilege_mut(&mut self) -> TssDescriptorPrivilegeMut<'_> {
        let &mut Self(ref mut low, ..) = self;

        TssDescriptorPrivilegeMut::wrap(low.value_mut())
    }

    /// Returns the descriptor-present bit.
    #[inline]
    #[must_use]
    pub const fn present(&self) -> TssDescriptorPresent<'_> {
        let &Self(ref low, ..) = self;

        TssDescriptorPresent::wrap(low.value())
    }

    /// Returns mutable access to the descriptor-present bit.
    #[inline]
    pub const fn present_mut(&mut self) -> TssDescriptorPresentMut<'_> {
        let &mut Self(ref mut low, ..) = self;

        TssDescriptorPresentMut::wrap(low.value_mut())
    }

    /// Returns the high limit field.
    #[inline]
    #[must_use]
    pub const fn limit_high(&self) -> TssDescriptorLimitHigh<'_> {
        let &Self(ref low, ..) = self;

        TssDescriptorLimitHigh::wrap(low.value())
    }

    /// Returns mutable access to the high limit field.
    #[inline]
    pub const fn limit_high_mut(&mut self) -> TssDescriptorLimitHighMut<'_> {
        let &mut Self(ref mut low, ..) = self;

        TssDescriptorLimitHighMut::wrap(low.value_mut())
    }

    /// Returns the high byte of the low-word base.
    #[inline]
    #[must_use]
    pub const fn base_high(&self) -> TssDescriptorBaseHigh<'_> {
        let &Self(ref low, ..) = self;

        TssDescriptorBaseHigh::wrap(low.value())
    }

    /// Returns mutable access to the high byte of the low-word base.
    #[inline]
    pub const fn base_high_mut(&mut self) -> TssDescriptorBaseHighMut<'_> {
        let &mut Self(ref mut low, ..) = self;

        TssDescriptorBaseHighMut::wrap(low.value_mut())
    }

    /// Returns the upper thirty-two bits of the base.
    #[inline]
    #[must_use]
    pub const fn base_upper(&self) -> TssDescriptorBaseUpper<'_> {
        let &Self(_, ref high) = self;

        TssDescriptorBaseUpper::wrap(high.value())
    }

    /// Returns mutable access to the upper thirty-two bits of the base.
    #[inline]
    pub const fn base_upper_mut(&mut self) -> TssDescriptorBaseUpperMut<'_> {
        let &mut Self(_, ref mut high) = self;

        TssDescriptorBaseUpperMut::wrap(high.value_mut())
    }
}

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
                let base_low = TssBaseValueLow::wrap(&target_value).const_value();
                let base_middle = TssBaseValueMiddle::wrap(&target_value).const_value();
                let base_high = TssBaseValueHigh::wrap(&target_value).const_value();
                let base_upper = TssBaseValueUpper::wrap(&target_value).const_value();
                let limit_low = TssLimitValueLow::wrap(&limit).const_value();
                let limit_high = TssLimitValueHigh::wrap(&limit).const_value();
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

                let raw = RawLongModeTssDescriptor::new(low, high);

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

        // NOTE(invariant): NonZero<u32> guarantees a byte length of at least one,
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

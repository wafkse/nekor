//! Long-mode descriptor-table register state.

use core::{marker::PhantomData, mem, num::NonZeroU32};

use nekor_bitwise::prelude::{Bit, Counterpart, Field, State, U64Low32Mut};

use crate::{
    x86::{
        descriptor::{DescriptorIndex, DescriptorTable, Gdt, Idt},
        privilege::PrivilegeLevel,
    },
    x86_64::{paging::La, task::TaskStateSegment},
};

pub mod global;

pub mod local;

/// One long-mode descriptor-table register image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
// NOTE(invariant): `base` is a canonical linear address, `limit` is the one-biased byte-length
// image accepted by x86, and `table` preserves the descriptor-table kind.
pub struct DescriptorTableRegister<T>
where
    T: DescriptorTable,
{
    /// Canonical linear base address.
    base: La,

    /// Architectural one-biased byte limit.
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
type TssDescriptorLimitLow<'value> = Field<'value, 0, 15, u64>;

/// Mutable counterpart to [`TssDescriptorLimitLow`].
type TssDescriptorLimitLowMut<'value> = <TssDescriptorLimitLow<'value> as Counterpart>::Mut;

/// Low base field in a long-mode task-state descriptor.
type TssDescriptorBaseLow<'value> = Field<'value, 16, 31, u64>;

/// Mutable counterpart to [`TssDescriptorBaseLow`].
type TssDescriptorBaseLowMut<'value> = <TssDescriptorBaseLow<'value> as Counterpart>::Mut;

/// Middle base field in a long-mode task-state descriptor.
type TssDescriptorBaseMiddle<'value> = Field<'value, 32, 39, u64>;

/// Mutable counterpart to [`TssDescriptorBaseMiddle`].
type TssDescriptorBaseMiddleMut<'value> = <TssDescriptorBaseMiddle<'value> as Counterpart>::Mut;

/// System descriptor type field in a long-mode task-state descriptor.
type TssDescriptorType<'value> = Field<'value, 40, 43, u64>;

/// Mutable counterpart to [`TssDescriptorType`].
type TssDescriptorTypeMut<'value> = <TssDescriptorType<'value> as Counterpart>::Mut;

/// Descriptor privilege field in a long-mode task-state descriptor.
type TssDescriptorPrivilege<'value> = Field<'value, 45, 46, u64>;

/// Mutable counterpart to [`TssDescriptorPrivilege`].
type TssDescriptorPrivilegeMut<'value> = <TssDescriptorPrivilege<'value> as Counterpart>::Mut;

/// Present bit in a long-mode task-state descriptor.
type TssDescriptorPresent<'value> = Bit<'value, u64, 47>;

/// Mutable counterpart to [`TssDescriptorPresent`].
type TssDescriptorPresentMut<'value> = <TssDescriptorPresent<'value> as Counterpart>::Mut;

/// High limit field in a long-mode task-state descriptor.
type TssDescriptorLimitHigh<'value> = Field<'value, 48, 51, u64>;

/// Mutable counterpart to [`TssDescriptorLimitHigh`].
type TssDescriptorLimitHighMut<'value> = <TssDescriptorLimitHigh<'value> as Counterpart>::Mut;

/// High byte of the low-word task-state base.
type TssDescriptorBaseHigh<'value> = Field<'value, 56, 63, u64>;

/// Mutable counterpart to [`TssDescriptorBaseHigh`].
type TssDescriptorBaseHighMut<'value> = <TssDescriptorBaseHigh<'value> as Counterpart>::Mut;

/// One available 64-bit task-state-segment descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
// NOTE(invariant): `low` and `high` form one complete sixteen-byte available
// long-mode TSS descriptor with a canonical base and a twenty-bit byte limit.
pub struct LongModeTssDescriptor {
    /// Low eight bytes of the architectural system descriptor.
    low: u64,

    /// High eight bytes of the architectural system descriptor.
    high: u64,
}

impl LongModeTssDescriptor {
    /// Constructs an available long-mode descriptor for one bare task-state segment.
    ///
    /// The descriptor extent is derived from [`TaskStateSegment`] so safe callers cannot expose
    /// an undersized TSS or accidentally extend the limit into an adjacent I/O permission bitmap.
    #[inline]
    #[must_use]
    pub const fn new(base: La, privilege: PrivilegeLevel) -> Self {
        let limit = (mem::size_of::<TaskStateSegment>() - 1) as u64;
        let base = base.bits();
        let base_low = TssBaseValueLow::wrap(&base).const_value();
        let base_middle = TssBaseValueMiddle::wrap(&base).const_value();
        let base_high = TssBaseValueHigh::wrap(&base).const_value();
        let base_upper = TssBaseValueUpper::wrap(&base).const_value();
        let limit_low = TssLimitValueLow::wrap(&limit).const_value();
        let limit_high = TssLimitValueHigh::wrap(&limit).const_value();
        let mut low = u64::MIN;
        let mut low_limit = TssDescriptorLimitLowMut::wrap(&mut low);

        low_limit.const_merge(limit_low);

        let mut low_base = TssDescriptorBaseLowMut::wrap(&mut low);

        low_base.const_merge(base_low);

        let mut middle_base = TssDescriptorBaseMiddleMut::wrap(&mut low);

        middle_base.const_merge(base_middle);

        let mut descriptor_type = TssDescriptorTypeMut::wrap(&mut low);

        descriptor_type.const_merge(0x9);

        let mut descriptor_privilege = TssDescriptorPrivilegeMut::wrap(&mut low);

        descriptor_privilege.const_merge(privilege.raw());

        let mut present = TssDescriptorPresentMut::wrap(&mut low);

        present.const_set(State::Set);

        let mut high_limit = TssDescriptorLimitHighMut::wrap(&mut low);

        high_limit.const_merge(limit_high);

        let mut high_base = TssDescriptorBaseHighMut::wrap(&mut low);

        high_base.const_merge(base_high);

        let mut high = u64::MIN;
        let mut upper_base = U64Low32Mut::wrap(&mut high);

        upper_base.const_merge(base_upper);

        Self { low, high }
    }

    /// Returns the complete little-endian hardware descriptor image.
    #[inline]
    #[must_use]
    pub const fn to_le_bytes(self) -> [u8; 16] {
        let Self { low, high } = self;

        let [l0, l1, l2, l3, l4, l5, l6, l7] = low.to_le_bytes();

        let [h0, h1, h2, h3, h4, h5, h6, h7] = high.to_le_bytes();

        [l0, l1, l2, l3, l4, l5, l6, l7, h0, h1, h2, h3, h4, h5, h6, h7]
    }
}

impl<T> DescriptorTableRegister<T>
where
    T: DescriptorTable,
{
    /// Constructs a register from a canonical base and architectural one-biased limit.
    #[inline]
    #[must_use]
    pub const fn from_limit(base: La, limit: u16) -> Self {
        Self {
            base,
            limit,
            table: PhantomData,
        }
    }

    /// Create a descriptor-table register from a base and table length.
    ///
    /// Returns `None` when the architectural limit derived from
    /// `byte_length` does not fit in the register's 16-bit limit field.
    #[inline]
    #[must_use]
    pub fn from_byte_len(base: La, byte_length: NonZeroU32) -> Option<Self> {
        let limit = byte_length.get() - 1;
        let limit = u16::try_from(limit);

        match limit {
            Ok(limit) => {
                let table = PhantomData;

                Some(Self { base, limit, table })
            },
            Err(_error) => None,
        }
    }

    /// Returns the canonical table base.
    #[inline]
    #[must_use]
    pub const fn base(&self) -> La {
        let &Self { base, .. } = self;

        base
    }

    /// Returns the architectural one-biased byte limit.
    #[inline]
    #[must_use]
    pub const fn limit(&self) -> u16 {
        let &Self { limit, .. } = self;

        limit
    }

    /// Returns the complete little-endian long-mode descriptor-table register image.
    ///
    /// `LGDT` and `LIDT` consume a ten-byte memory operand containing the 16-bit limit followed
    /// immediately by the 64-bit base. This method deliberately serializes that hardware image
    /// instead of exposing the Rust representation of [`DescriptorTableRegister`].
    #[inline]
    #[must_use]
    pub const fn to_le_bytes(&self) -> [u8; 10] {
        let &Self { base, limit, .. } = self;

        let [l0, l1] = limit.to_le_bytes();

        let [b0, b1, b2, b3, b4, b5, b6, b7] = base.bits().to_le_bytes();

        [l0, l1, b0, b1, b2, b3, b4, b5, b6, b7]
    }
}

#[cfg(test)]
mod tests {
    use core::num::NonZeroU32;

    use super::{GdtRegister, LongModeTssDescriptor};
    use crate::{
        x86::privilege::PrivilegeLevel,
        x86_64::paging::{La, La48},
    };

    #[test]
    fn descriptor_table_register_tracks_architectural_limit() {
        let base = La::new::<La48>(0x6000).expect("test table base must be canonical");
        let one = NonZeroU32::new(1).expect("one is nonzero");
        let maximum = NonZeroU32::new(0x0001_0000).expect("maximum table length is nonzero");
        let oversized = NonZeroU32::new(0x0001_0001).expect("oversized table length is nonzero");
        let smallest = GdtRegister::from_byte_len(base, one).expect("one byte table must fit");
        let largest = GdtRegister::from_byte_len(base, maximum).expect("maximum table must fit");

        assert_eq!(smallest.base(), base);
        assert_eq!(smallest.limit(), 0);
        assert_eq!(smallest.to_le_bytes(), [0, 0, 0, 0x60, 0, 0, 0, 0, 0, 0]);
        assert_eq!(largest.limit(), u16::MAX);
        assert!(GdtRegister::from_byte_len(base, oversized).is_none());
    }

    #[test]
    fn long_mode_tss_descriptor_matches_the_architectural_layout() {
        let base = La::new::<La48>(0x1234_5678).expect("test TSS base must be canonical");
        let descriptor = LongModeTssDescriptor::new(base, PrivilegeLevel::Ring0);

        assert_eq!(
            descriptor.to_le_bytes(),
            [0x67, 0x00, 0x78, 0x56, 0x34, 0x89, 0x00, 0x12, 0, 0, 0, 0, 0, 0, 0, 0]
        );
    }
}

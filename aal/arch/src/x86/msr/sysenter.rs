//! Legacy SYSENTER model-specific register representations.
//!
//! Raw types preserve architectural register snapshots. Checked values
//! carry selector adjacency and canonical linear-address properties.

use nekor_bitwise::prelude::{Counterpart, Field};

use crate::x86::{
    msr::{Cpl0Access, detail::snapshotted},
    privilege::PrivilegeLevel,
    segmentation::{CodeSegment, DataSegment, TableIndicator},
};
#[cfg(target_arch = "x86_64")]
use crate::x86_64::paging::{La, LaMode};

/// Code-selector field in RawSysEnterCs.
pub type SysEnterCsSelector<'value> = Field<'value, 0, 15, u64>;

/// Mutable counterpart to SysEnterCsSelector.
pub type SysEnterCsSelectorMut<'value> = <SysEnterCsSelector<'value> as Counterpart>::Mut;

/// Reserved high field in RawSysEnterCs.
pub type SysEnterCsReserved<'value> = Field<'value, 16, 63, u64>;

/// Mutable counterpart to SysEnterCsReserved.
pub type SysEnterCsReservedMut<'value> = <SysEnterCsReserved<'value> as Counterpart>::Mut;

snapshotted::snapshot!(
    /// `IA32_SYSENTER_CS` register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct RawSysEnterCs for (0x0000_0174) use Cpl0Access
);

impl RawSysEnterCs {
    /// Borrows the selector field.
    #[inline]
    #[must_use]
    pub const fn selector(&self) -> SysEnterCsSelector<'_> {
        let &Self(ref target_value) = self;

        SysEnterCsSelector::wrap(target_value)
    }

    /// Mutably borrows the selector field.
    #[inline]
    pub const fn selector_mut(&mut self) -> SysEnterCsSelectorMut<'_> {
        let &mut Self(ref mut target_value) = self;

        SysEnterCsSelectorMut::wrap(target_value)
    }

    /// Borrows the reserved high field.
    #[inline]
    #[must_use]
    pub const fn reserved(&self) -> SysEnterCsReserved<'_> {
        let &Self(ref target_value) = self;

        SysEnterCsReserved::wrap(target_value)
    }

    /// Mutably borrows the reserved high field.
    #[inline]
    pub const fn reserved_mut(&mut self) -> SysEnterCsReservedMut<'_> {
        let &mut Self(ref mut target_value) = self;

        SysEnterCsReservedMut::wrap(target_value)
    }
}

/// Checked SYSENTER code and stack selector family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The stored raw snapshot selects the supplied ring-zero GDT code segment and the
// supplied ring-zero data segment has the immediately following descriptor index. This does not
// prove that either descriptor is installed in the live GDT.
pub struct SysEnterCs(RawSysEnterCs);

impl SysEnterCs {
    /// Proves the selector relationship consumed by SYSENTER.
    #[inline]
    #[must_use]
    pub const fn new(code: CodeSegment, data: DataSegment) -> Option<Self> {
        match (
            code.table(),
            data.table(),
            code.privilege(),
            data.privilege(),
            data.index().raw().checked_sub(code.index().raw()),
        ) {
            (TableIndicator::Gdt, TableIndicator::Gdt, PrivilegeLevel::Ring0, PrivilegeLevel::Ring0, Some(1)) => {
                let mut target_value = RawSysEnterCs::new(u64::MIN);

                target_value.selector_mut().const_merge(code.raw().raw());

                Some(Self(target_value))
            },
            _ => None,
        }
    }

    /// Borrows the proven code selector field.
    #[inline]
    #[must_use]
    pub const fn selector(&self) -> SysEnterCsSelector<'_> {
        let &Self(ref target_value) = self;

        target_value.selector()
    }

    /// Lowers this checked selector family into the register snapshot.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> RawSysEnterCs {
        let Self(target_value) = self;

        target_value
    }
}

snapshotted::snapshot!(
    /// `IA32_SYSENTER_ESP` register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct RawSysEnterSp for (0x0000_0175) use Cpl0Access
);

/// Canonical SYSENTER stack pointer.
#[cfg(target_arch = "x86_64")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The stored value is a canonical linear address established by La construction
// or checked lifting from a raw `SYSENTER_ESP` snapshot.
pub struct SysEnterSp(La);

#[cfg(target_arch = "x86_64")]
impl SysEnterSp {
    /// Constructs a stack pointer from a linear address.
    #[inline]
    #[must_use]
    pub const fn new(address: La) -> Self {
        Self(address)
    }

    /// Lifts a raw stack-pointer snapshot when it is canonical under M.
    #[inline]
    #[must_use]
    pub const fn lift<M>(target_value: RawSysEnterSp) -> Option<Self>
    where
        M: LaMode,
    {
        let RawSysEnterSp(target_value) = target_value;

        match La::new::<M>(target_value) {
            Some(address) => Some(Self(address)),
            None => None,
        }
    }

    /// Returns the canonical linear address.
    #[inline]
    #[must_use]
    pub const fn la(self) -> La {
        let Self(address) = self;

        address
    }

    /// Lowers this checked stack pointer into the register snapshot.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> RawSysEnterSp {
        let Self(address) = self;

        RawSysEnterSp::new(address.bits())
    }
}

snapshotted::snapshot!(
    /// `IA32_SYSENTER_EIP` register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct RawSysEnterIp for (0x0000_0176) use Cpl0Access
);

/// Canonical SYSENTER instruction pointer.
#[cfg(target_arch = "x86_64")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The stored value is a canonical linear address established by La construction
// or checked lifting from a raw `SYSENTER_EIP` snapshot.
pub struct SysEnterIp(La);

#[cfg(target_arch = "x86_64")]
impl SysEnterIp {
    /// Constructs an instruction pointer from a linear address.
    #[inline]
    #[must_use]
    pub const fn new(address: La) -> Self {
        Self(address)
    }

    /// Lifts a raw instruction-pointer snapshot when it is canonical under M.
    #[inline]
    #[must_use]
    pub const fn lift<M>(target_value: RawSysEnterIp) -> Option<Self>
    where
        M: LaMode,
    {
        let RawSysEnterIp(target_value) = target_value;

        match La::new::<M>(target_value) {
            Some(address) => Some(Self(address)),
            None => None,
        }
    }

    /// Returns the canonical linear address.
    #[inline]
    #[must_use]
    pub const fn la(self) -> La {
        let Self(address) = self;

        address
    }

    /// Lowers this checked instruction pointer into the register snapshot.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> RawSysEnterIp {
        let Self(address) = self;

        RawSysEnterIp::new(address.bits())
    }
}

#[cfg(test)]
mod tests {
    use super::{RawSysEnterCs, RawSysEnterIp, RawSysEnterSp, SysEnterCs};
    use crate::x86::{
        descriptor::{DescriptorIndex, RawSegmentDescriptor},
        msr::Msr,
        privilege::PrivilegeLevel,
        segmentation::{CodeSegment, DataSegment, SegmentSelector, TableIndicator},
    };

    fn code(index: u16) -> CodeSegment {
        let index = DescriptorIndex::lift(index).expect("fixture index must be nonzero");
        let selector = SegmentSelector::new(index, TableIndicator::Gdt, PrivilegeLevel::Ring0);
        let descriptor = RawSegmentDescriptor::long_mode_code(PrivilegeLevel::Ring0);

        CodeSegment::new(selector, descriptor).expect("fixture descriptor must be code")
    }

    fn data(index: u16) -> DataSegment {
        let index = DescriptorIndex::lift(index).expect("fixture index must be nonzero");
        let selector = SegmentSelector::new(index, TableIndicator::Gdt, PrivilegeLevel::Ring0);
        let descriptor = RawSegmentDescriptor::flat_data(PrivilegeLevel::Ring0);

        DataSegment::new(selector, descriptor).expect("fixture descriptor must be data")
    }

    #[test]
    fn sysenter_cs_requires_adjacent_ring_zero_gdt_segments() {
        let valid = SysEnterCs::new(code(1), data(2));
        let invalid = SysEnterCs::new(code(1), data(3));

        assert_eq!(valid.map(|value| value.selector().const_value()), Some(8));
        assert_eq!(invalid, None);
    }

    #[test]
    fn register_indices_match_architecture() {
        assert_eq!(RawSysEnterCs::REGISTER.address(), 0x174);
        assert_eq!(RawSysEnterSp::REGISTER.address(), 0x175);
        assert_eq!(RawSysEnterIp::REGISTER.address(), 0x176);
    }
}

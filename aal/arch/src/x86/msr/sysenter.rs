//! Register images for legacy SYSENTER entry.
//!
//! `MsrSysEnterCs::segments` checks the supplied code and data descriptor relationship.
//! Stack and instruction-pointer images can be prepared from canonical addresses.
//! Installing these values requires the entry owner to establish the live descriptors,
//! mappings and stack backing on which the entry path depends.

use nekor_bitwise::prelude::{Counterpart, Field};

use crate::x86::{
    msr::detail::snapshotted,
    privilege::PrivilegeLevel,
    segmentation::{CodeSegment, DataSegment, TableIndicator},
};
#[cfg(target_arch = "x86_64")]
use crate::x86_64::paging::{La, LaMode};

/// The code-selector field in `MsrSysEnterCs`.
pub type SysEnterCsSelector<'a> = Field<'a, 0, 15, u64>;

/// The mutable counterpart to `SysEnterCsSelector`.
pub type SysEnterCsSelectorMut<'a> = <SysEnterCsSelector<'a> as Counterpart>::Mut;

/// The reserved high field in `MsrSysEnterCs`.
pub type SysEnterCsReserved<'a> = Field<'a, 16, 63, u64>;

/// The mutable counterpart to `SysEnterCsReserved`.
pub type SysEnterCsReservedMut<'a> = <SysEnterCsReserved<'a> as Counterpart>::Mut;

snapshotted::snapshot!(
    /// The `IA32_SYSENTER_CS` SYSENTER code-selector model-specific register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MsrSysEnterCs for (0x0000_0174)
    for<'a> &'a crate::x86::privilege::Cpl<0, T>
);

impl MsrSysEnterCs {
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

impl MsrSysEnterCs {
    /// Encode adjacent ring-zero GDT code and data selectors.
    ///
    /// This checks the supplied descriptor values. The caller installing the register
    /// must also install the corresponding descriptors in the live GDT.
    #[inline]
    #[must_use]
    pub const fn segments(code: CodeSegment, data: DataSegment) -> Option<Self> {
        match (
            code.table(),
            data.table(),
            code.privilege(),
            data.privilege(),
            data.index().raw().checked_sub(code.index().raw()),
        ) {
            (TableIndicator::Gdt, TableIndicator::Gdt, PrivilegeLevel::Ring0, PrivilegeLevel::Ring0, Some(1)) => {
                let mut target_value = Self::new(u64::MIN);

                target_value.selector_mut().const_merge(code.raw().raw());

                Some(target_value)
            },
            _ => None,
        }
    }
}

snapshotted::snapshot!(
    /// The `IA32_SYSENTER_ESP` SYSENTER stack-pointer model-specific register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MsrSysEnterSp for (0x0000_0175)
    for<'a> &'a crate::x86::privilege::Cpl<0, T>
);

#[cfg(target_arch = "x86_64")]
impl MsrSysEnterSp {
    /// Construct an image from a canonical linear address.
    ///
    /// The address's construction mode must agree with the processor mode at installation.
    #[inline]
    #[must_use]
    pub const fn at(address: La) -> Self {
        Self::new(address.bits())
    }

    /// Interpret the image as a canonical address under the selected paging mode.
    #[inline]
    #[must_use]
    pub const fn la<M>(&self) -> Option<La>
    where
        M: LaMode,
    {
        La::new::<M>(self.raw())
    }
}

snapshotted::snapshot!(
    /// The `IA32_SYSENTER_EIP` SYSENTER instruction-pointer model-specific register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MsrSysEnterIp for (0x0000_0176)
    for<'a> &'a crate::x86::privilege::Cpl<0, T>
);

#[cfg(target_arch = "x86_64")]
impl MsrSysEnterIp {
    /// Construct an image from a canonical linear address.
    ///
    /// The address's construction mode must agree with the processor mode at installation.
    #[inline]
    #[must_use]
    pub const fn at(address: La) -> Self {
        Self::new(address.bits())
    }

    /// Interpret the image as a canonical address under the selected paging mode.
    #[inline]
    #[must_use]
    pub const fn la<M>(&self) -> Option<La>
    where
        M: LaMode,
    {
        La::new::<M>(self.raw())
    }
}

#[cfg(test)]
mod tests {
    use crate::x86::{
        descriptor::{DescriptorIndex, RawSegmentDescriptor},
        msr::{
            Msr,
            sysenter::{MsrSysEnterCs, MsrSysEnterIp, MsrSysEnterSp},
        },
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
        let valid = MsrSysEnterCs::segments(code(1), data(2));
        let invalid = MsrSysEnterCs::segments(code(1), data(3));

        assert_eq!(valid.map(|value| value.selector().const_value()), Some(8));
        assert_eq!(invalid, None);
    }

    #[test]
    fn register_indices_match_architecture() {
        assert_eq!(MsrSysEnterCs::REGISTER.address(), 0x174);
        assert_eq!(MsrSysEnterSp::REGISTER.address(), 0x175);
        assert_eq!(MsrSysEnterIp::REGISTER.address(), 0x176);
    }
}

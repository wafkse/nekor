//! Register images for fast system-call entry and return.
//!
//! `MsrStar` encodes selector families checked by `SyscallSegments` and `SysretSegments`.
//! Those checks describe the supplied descriptors, so the entry owner must also install
//! them in the live GDT. Address helpers on `MsrLStar` and `MsrCStar` check canonicality
//! against a selected paging mode without establishing handler lifetime or mappings.

use nekor_bitwise::prelude::{Counterpart, Field};

use crate::x86::{
    msr::detail::snapshotted,
    privilege::PrivilegeLevel,
    segmentation::{CodeSegment, DataSegment, RawSegmentSelector, TableIndicator},
};
#[cfg(target_arch = "x86_64")]
use crate::x86_64::paging::{La, LaMode};

snapshotted::snapshot!(
    /// The `IA32_LSTAR` long-mode system-call target model-specific register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MsrLStar for (0xC000_0082)
    for<'a> &'a crate::x86::privilege::Cpl<0, T>
);

#[cfg(target_arch = "x86_64")]
impl MsrLStar {
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
    /// The `IA32_CSTAR` compatibility-mode system-call target model-specific register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MsrCStar for (0xC000_0083)
    for<'a> &'a crate::x86::privilege::Cpl<0, T>
);

#[cfg(target_arch = "x86_64")]
impl MsrCStar {
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

/// The kernel code-selector field in `MsrStar`.
pub type StarKernelSelector<'a> = Field<'a, 32, 47, u64>;

/// The mutable counterpart to `StarKernelSelector`.
pub type StarKernelSelectorMut<'a> = <StarKernelSelector<'a> as Counterpart>::Mut;

/// The user selector-base field in `MsrStar`.
pub type StarUserSelector<'a> = Field<'a, 48, 63, u64>;

/// The mutable counterpart to `StarUserSelector`.
pub type StarUserSelectorMut<'a> = <StarUserSelector<'a> as Counterpart>::Mut;

snapshotted::snapshot!(
    /// The `IA32_STAR` system-call selector model-specific register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MsrStar for (0xC000_0081)
    for<'a> &'a crate::x86::privilege::Cpl<0, T>
);

impl MsrStar {
    /// Borrows the kernel code-selector field.
    #[inline]
    #[must_use]
    pub const fn kernel(&self) -> StarKernelSelector<'_> {
        let &Self(ref target_value) = self;

        StarKernelSelector::wrap(target_value)
    }

    /// Mutably borrows the kernel code-selector field.
    #[inline]
    pub const fn kernel_mut(&mut self) -> StarKernelSelectorMut<'_> {
        let &mut Self(ref mut target_value) = self;

        StarKernelSelectorMut::wrap(target_value)
    }

    /// Borrows the user selector-base field.
    #[inline]
    #[must_use]
    pub const fn user(&self) -> StarUserSelector<'_> {
        let &Self(ref target_value) = self;

        StarUserSelector::wrap(target_value)
    }

    /// Mutably borrows the user selector-base field.
    #[inline]
    pub const fn user_mut(&mut self) -> StarUserSelectorMut<'_> {
        let &mut Self(ref mut target_value) = self;

        StarUserSelectorMut::wrap(target_value)
    }
}

/// A pair of adjacent ring-zero GDT segments supplied for long-mode SYSCALL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The stored selector names the supplied ring-zero GDT code segment and the
// supplied ring-zero data segment has the immediately following descriptor index. This does not
// prove that either descriptor is installed in the live GDT.
pub struct SyscallSegments(u16);

impl SyscallSegments {
    /// Proves the selector relationship required by long-mode SYSCALL.
    ///
    /// This validates the supplied descriptor values only. It does not prove
    /// installation in the live GDT.
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
                Some(Self(code.raw().raw()))
            },
            _ => None,
        }
    }

    /// Returns the encoded kernel code selector.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> RawSegmentSelector {
        let Self(target_value) = self;

        RawSegmentSelector::new(target_value)
    }
}

/// A pair of adjacent ring-three GDT segments supplied for long-mode SYSRET.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The stored selector is one GDT entry before the supplied ring-three data
// segment and the supplied ring-three code segment immediately follows that data segment. The RPL
// is zero because SYSRET supplies ring three while deriving SS and CS from this base. This does not
// prove that either descriptor is installed in the live GDT.
pub struct SysretSegments(u16);

impl SysretSegments {
    /// Proves the selector relationship required by long-mode SYSRET.
    ///
    /// This validates the supplied descriptor values only. It does not prove
    /// installation in the live GDT.
    #[inline]
    #[must_use]
    pub const fn new(data: DataSegment, code: CodeSegment) -> Option<Self> {
        match (
            data.table(),
            code.table(),
            data.privilege(),
            code.privilege(),
            code.index().raw().checked_sub(data.index().raw()),
            data.index().raw().checked_sub(1),
        ) {
            (
                TableIndicator::Gdt,
                TableIndicator::Gdt,
                PrivilegeLevel::Ring3,
                PrivilegeLevel::Ring3,
                Some(1),
                Some(base),
            ) => {
                let mut selector = RawSegmentSelector::zeroed();

                selector.index_mut().const_merge(base);

                Some(Self(selector.raw()))
            },
            _ => None,
        }
    }

    /// Returns the encoded architectural SYSRET selector base.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> RawSegmentSelector {
        let Self(target_value) = self;

        RawSegmentSelector::new(target_value)
    }
}

impl MsrStar {
    /// Encode the supplied SYSCALL and SYSRET selector families.
    ///
    /// Their constructors check descriptor relationships. The installation owner must
    /// arrange the corresponding live descriptor table before enabling the entry path.
    #[inline]
    #[must_use]
    pub const fn segments(kernel: SyscallSegments, user: SysretSegments) -> Self {
        let mut target_value = Self::new(u64::MIN);

        target_value.kernel_mut().const_merge(kernel.raw().raw());
        target_value.user_mut().const_merge(user.raw().raw());

        target_value
    }
}

snapshotted::snapshot!(
    /// The `IA32_FMASK` system-call flag mask model-specific register snapshot.
    ///
    /// The bit positions correspond to RFLAGS positions, but a mask is not an
    /// RFLAGS machine state and therefore does not reuse the semantic Rflags type.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MsrFMask for (0xC000_0084)
    for<'a> &'a crate::x86::privilege::Cpl<0, T>
);

#[cfg(test)]
mod tests {
    use crate::x86::{
        descriptor::{DescriptorIndex, RawSegmentDescriptor},
        msr::{
            Msr,
            syscall::{MsrCStar, MsrFMask, MsrLStar, MsrStar, SyscallSegments, SysretSegments},
        },
        privilege::PrivilegeLevel,
        segmentation::{CodeSegment, DataSegment, SegmentSelector, TableIndicator},
    };

    fn code(index: u16, privilege: PrivilegeLevel) -> CodeSegment {
        let index = DescriptorIndex::lift(index).expect("fixture index must be nonzero");
        let selector = SegmentSelector::new(index, TableIndicator::Gdt, privilege);
        let descriptor = RawSegmentDescriptor::long_mode_code(privilege);

        CodeSegment::new(selector, descriptor).expect("fixture descriptor must be long-mode code")
    }

    fn data(index: u16, privilege: PrivilegeLevel) -> DataSegment {
        let index = DescriptorIndex::lift(index).expect("fixture index must be nonzero");
        let selector = SegmentSelector::new(index, TableIndicator::Gdt, privilege);
        let descriptor = RawSegmentDescriptor::flat_data(privilege);

        DataSegment::new(selector, descriptor).expect("fixture descriptor must be writable data")
    }

    #[test]
    fn star_encodes_proven_selector_families() {
        let kernel = SyscallSegments::new(code(1, PrivilegeLevel::Ring0), data(2, PrivilegeLevel::Ring0))
            .expect("is_adjacent kernel segments must validate");
        let user = SysretSegments::new(data(3, PrivilegeLevel::Ring3), code(4, PrivilegeLevel::Ring3))
            .expect("is_adjacent user segments must validate");
        let star = MsrStar::segments(kernel, user);

        assert_eq!(star.kernel().const_value(), 8);
        assert_eq!(star.user().const_value(), 16);
    }

    #[test]
    fn raw_star_preserves_arbitrary_snapshots() {
        let raw = MsrStar::new(u64::MAX);

        assert_eq!(raw.raw(), u64::MAX);
    }

    #[test]
    fn segment_families_reject_nonadjacent_descriptors() {
        let kernel = SyscallSegments::new(code(1, PrivilegeLevel::Ring0), data(3, PrivilegeLevel::Ring0));
        let user = SysretSegments::new(data(3, PrivilegeLevel::Ring3), code(5, PrivilegeLevel::Ring3));

        assert_eq!(kernel, None);
        assert_eq!(user, None);
    }

    #[test]
    fn register_indices_match_architecture() {
        assert_eq!(MsrStar::REGISTER.address(), 0xC000_0081);
        assert_eq!(MsrLStar::REGISTER.address(), 0xC000_0082);
        assert_eq!(MsrCStar::REGISTER.address(), 0xC000_0083);
        assert_eq!(MsrFMask::REGISTER.address(), 0xC000_0084);
    }
}

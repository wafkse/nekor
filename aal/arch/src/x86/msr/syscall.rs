//! Fast system-call and system-return model-specific register representations.
//!
//! Raw types preserve architectural register snapshots. Checked types
//! encode the selector and linear-address relationships used by long-mode
//! SYSCALL and SYSRET.

use nekor_bitwise::prelude::{Counterpart, Field};

use crate::x86::{
    msr::{Cpl0Access, detail::snapshotted},
    privilege::PrivilegeLevel,
    segmentation::{CodeSegment, DataSegment, RawSegmentSelector, TableIndicator},
};
#[cfg(target_arch = "x86_64")]
use crate::x86_64::paging::{La, LaMode};

snapshotted::snapshot!(
    /// `IA32_LSTAR` register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct RawLStar for (0xC000_0082) use Cpl0Access
);

/// Canonical long-mode system-call target.
#[cfg(target_arch = "x86_64")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The stored value is a canonical linear address established by La construction
// or checked lifting from a raw `LSTAR` snapshot.
pub struct LStar(La);

#[cfg(target_arch = "x86_64")]
impl LStar {
    /// Constructs a system-call target from a linear address.
    #[inline]
    #[must_use]
    pub const fn new(address: La) -> Self {
        Self(address)
    }

    /// Lifts a raw `LSTAR` snapshot when it is canonical under M.
    #[inline]
    #[must_use]
    pub const fn lift<M>(target_value: RawLStar) -> Option<Self>
    where
        M: LaMode,
    {
        let RawLStar(target_value) = target_value;

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

    /// Lowers this checked target into its `LSTAR` snapshot.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> RawLStar {
        let Self(address) = self;

        RawLStar::new(address.bits())
    }
}

snapshotted::snapshot!(
    /// `IA32_CSTAR` register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct RawCStar for (0xC000_0083) use Cpl0Access
);

/// Canonical compatibility-mode system-call target.
#[cfg(target_arch = "x86_64")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The stored value is a canonical linear address established by La construction
// or checked lifting from a raw `CSTAR` snapshot.
pub struct CStar(La);

#[cfg(target_arch = "x86_64")]
impl CStar {
    /// Constructs a compatibility-mode target from a linear address.
    #[inline]
    #[must_use]
    pub const fn new(address: La) -> Self {
        Self(address)
    }

    /// Lifts a raw `CSTAR` snapshot when it is canonical under M.
    #[inline]
    #[must_use]
    pub const fn lift<M>(target_value: RawCStar) -> Option<Self>
    where
        M: LaMode,
    {
        let RawCStar(target_value) = target_value;

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

    /// Lowers this checked target into its `CSTAR` snapshot.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> RawCStar {
        let Self(address) = self;

        RawCStar::new(address.bits())
    }
}

/// Kernel code-selector field in RawStar.
pub type StarKernelSelector<'value> = Field<'value, 32, 47, u64>;

/// Mutable counterpart to StarKernelSelector.
pub type StarKernelSelectorMut<'value> = <StarKernelSelector<'value> as Counterpart>::Mut;

/// User selector-base field in RawStar.
pub type StarUserSelector<'value> = Field<'value, 48, 63, u64>;

/// Mutable counterpart to StarUserSelector.
pub type StarUserSelectorMut<'value> = <StarUserSelector<'value> as Counterpart>::Mut;

snapshotted::snapshot!(
    /// `IA32_STAR` register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct RawStar for (0xC000_0081) use Cpl0Access
);

impl RawStar {
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

/// Adjacent ring-zero GDT segments supplied for long-mode SYSCALL.
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

/// Adjacent ring-three GDT segments supplied for long-mode SYSRET.
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

/// Checked long-mode `IA32_STAR` selector configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The wrapped raw snapshot is constructed only from independently proven SYSCALL
// and SYSRET selector families and cannot be mutated through this semantic type.
pub struct Star(RawStar);

impl Star {
    /// Constructs `STAR` from proven SYSCALL and SYSRET selector families.
    #[inline]
    #[must_use]
    pub const fn new(kernel: SyscallSegments, user: SysretSegments) -> Self {
        let kernel = kernel.raw().raw();
        let user = user.raw().raw();
        let mut target_value = RawStar::new(u64::MIN);

        target_value.kernel_mut().const_merge(kernel);
        target_value.user_mut().const_merge(user);

        Self(target_value)
    }

    /// Borrows the proven kernel selector field.
    #[inline]
    #[must_use]
    pub const fn kernel(&self) -> StarKernelSelector<'_> {
        let &Self(ref target_value) = self;

        target_value.kernel()
    }

    /// Borrows the proven user selector-base field.
    #[inline]
    #[must_use]
    pub const fn user(&self) -> StarUserSelector<'_> {
        let &Self(ref target_value) = self;

        target_value.user()
    }

    /// Lowers this checked selector configuration into the `STAR` snapshot.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> RawStar {
        let Self(target_value) = self;

        target_value
    }
}

snapshotted::snapshot!(
    /// `IA32_FMASK` register snapshot.
    ///
    /// The bit positions correspond to RFLAGS positions, but a mask is not an
    /// RFLAGS machine state and therefore does not reuse the semantic Rflags type.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct RawFMask for (0xC000_0084) use Cpl0Access
);

#[cfg(test)]
mod tests {
    use super::{RawCStar, RawFMask, RawLStar, RawStar, Star, SyscallSegments, SysretSegments};
    use crate::x86::{
        descriptor::{DescriptorIndex, RawSegmentDescriptor},
        msr::Msr,
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
        let star = Star::new(kernel, user);

        assert_eq!(star.kernel().const_value(), 8);
        assert_eq!(star.user().const_value(), 16);

        let raw = star.raw();

        assert_eq!(raw.kernel().const_value(), 8);
        assert_eq!(raw.user().const_value(), 16);
    }

    #[test]
    fn raw_star_preserves_arbitrary_snapshots() {
        let raw = RawStar::new(u64::MAX);

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
        assert_eq!(RawStar::REGISTER.address(), 0xC000_0081);
        assert_eq!(RawLStar::REGISTER.address(), 0xC000_0082);
        assert_eq!(RawCStar::REGISTER.address(), 0xC000_0083);
        assert_eq!(RawFMask::REGISTER.address(), 0xC000_0084);
    }
}

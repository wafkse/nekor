//! Fast system-call and system-return MSR representations.
//!
//! Raw types preserve architectural register images. Checked types
//! encode the selector and linear-address relationships used by long-mode
//! SYSCALL and SYSRET.

use nekor_bitwise::prelude::{Counterpart, Field};

use super::{Cpl0Access, Msr, ReadWrite};
use crate::x86::{
    privilege::PrivilegeLevel,
    segmentation::{CodeSegment, DataSegment, RawSegmentSelector, TableIndicator},
};
#[cfg(target_arch = "x86_64")]
use crate::x86_64::paging::{La, LaMode};

/// Exact IA32_LSTAR register image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): Every u64 value is a representable raw LSTAR image.
pub struct RawLStar(u64);

impl RawLStar {
    /// Constructs a raw LSTAR image.
    #[inline]
    #[must_use]
    pub const fn new(target_value: u64) -> Self {
        Self(target_value)
    }

    /// Returns the raw LSTAR image.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u64 {
        let Self(target_value) = self;

        target_value
    }
}

// SAFETY: RawLStar is transparent over u64 and every bit pattern is valid.
unsafe impl Msr for RawLStar {
    type Access = ReadWrite;
    type Authority = Cpl0Access;

    const ADDRESS: u32 = 0xC000_0082;
}

impl super::private::Sealed for RawLStar {}

/// Canonical long-mode system-call target.
#[cfg(target_arch = "x86_64")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The stored value is a canonical linear address established by La construction
// or checked lifting from a raw LSTAR image.
pub struct LStar(La);

#[cfg(target_arch = "x86_64")]
impl LStar {
    /// Constructs a system-call target from a linear address.
    #[inline]
    #[must_use]
    pub const fn new(address: La) -> Self {
        Self(address)
    }

    /// Lifts a raw LSTAR image when it is canonical under M.
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

    /// Lowers this checked target into its exact LSTAR image.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> RawLStar {
        let Self(address) = self;

        RawLStar::new(address.bits())
    }
}

/// Exact IA32_CSTAR register image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): Every u64 value is a representable raw CSTAR image.
pub struct RawCStar(u64);

impl RawCStar {
    /// Constructs a raw CSTAR image.
    #[inline]
    #[must_use]
    pub const fn new(target_value: u64) -> Self {
        Self(target_value)
    }

    /// Returns the raw CSTAR image.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u64 {
        let Self(target_value) = self;

        target_value
    }
}

// SAFETY: RawCStar is transparent over u64 and every bit pattern is valid.
unsafe impl Msr for RawCStar {
    type Access = ReadWrite;
    type Authority = Cpl0Access;

    const ADDRESS: u32 = 0xC000_0083;
}

impl super::private::Sealed for RawCStar {}

/// Canonical compatibility-mode system-call target.
#[cfg(target_arch = "x86_64")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The stored value is a canonical linear address established by La construction
// or checked lifting from a raw CSTAR image.
pub struct CStar(La);

#[cfg(target_arch = "x86_64")]
impl CStar {
    /// Constructs a compatibility-mode target from a linear address.
    #[inline]
    #[must_use]
    pub const fn new(address: La) -> Self {
        Self(address)
    }

    /// Lifts a raw CSTAR image when it is canonical under M.
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

    /// Lowers this checked target into its exact CSTAR image.
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

/// Exact IA32_STAR register image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): Every u64 value is a representable raw STAR image.
pub struct RawStar(u64);

impl RawStar {
    /// Constructs a raw STAR image.
    #[inline]
    #[must_use]
    pub const fn new(target_value: u64) -> Self {
        Self(target_value)
    }

    /// Returns the raw STAR image.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u64 {
        let Self(target_value) = self;

        target_value
    }

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

// SAFETY: RawStar is transparent over u64 and every bit pattern is valid.
unsafe impl Msr for RawStar {
    type Access = ReadWrite;
    type Authority = Cpl0Access;

    const ADDRESS: u32 = 0xC000_0081;
}

impl super::private::Sealed for RawStar {}

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
        let tables_valid = matches!(code.table(), TableIndicator::Gdt) && matches!(data.table(), TableIndicator::Gdt);
        let privileges_valid =
            matches!(code.privilege(), PrivilegeLevel::Ring0) && matches!(data.privilege(), PrivilegeLevel::Ring0);
        let adjacent = matches!(data.index().raw().checked_sub(code.index().raw()), Some(1));

        match (tables_valid, privileges_valid, adjacent) {
            (true, true, true) => Some(Self(code.raw().raw())),
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
        let tables_valid = matches!(data.table(), TableIndicator::Gdt) && matches!(code.table(), TableIndicator::Gdt);
        let privileges_valid =
            matches!(data.privilege(), PrivilegeLevel::Ring3) && matches!(code.privilege(), PrivilegeLevel::Ring3);
        let adjacent = matches!(code.index().raw().checked_sub(data.index().raw()), Some(1));
        let base = data.index().raw().checked_sub(1);

        match (tables_valid, privileges_valid, adjacent, base) {
            (true, true, true, Some(base)) => {
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

/// Checked long-mode IA32_STAR selector configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The wrapped raw image is constructed only from independently proven SYSCALL
// and SYSRET selector families and cannot be mutated through this semantic type.
pub struct Star(RawStar);

impl Star {
    /// Constructs STAR from proven SYSCALL and SYSRET selector families.
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

    /// Lowers this checked selector configuration into the exact STAR image.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> RawStar {
        let Self(target_value) = self;

        target_value
    }
}

/// Exact IA32_FMASK register image.
///
/// The bit positions correspond to RFLAGS positions, but a mask is not an
/// RFLAGS machine state and therefore does not reuse the semantic Rflags type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): Every u64 value is a representable raw FMASK image.
pub struct RawFMask(u64);

impl RawFMask {
    /// Constructs a raw FMASK image.
    #[inline]
    #[must_use]
    pub const fn new(target_value: u64) -> Self {
        Self(target_value)
    }

    /// Returns the raw FMASK image.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u64 {
        let Self(target_value) = self;

        target_value
    }
}

// SAFETY: RawFMask is transparent over u64 and every bit pattern is valid.
unsafe impl Msr for RawFMask {
    type Access = ReadWrite;
    type Authority = Cpl0Access;

    const ADDRESS: u32 = 0xC000_0084;
}

impl super::private::Sealed for RawFMask {}

#[cfg(test)]
mod tests {
    use super::{Msr, RawCStar, RawFMask, RawLStar, RawStar, Star, SyscallSegments, SysretSegments};
    use crate::x86::{
        descriptor::{DescriptorIndex, RawSegmentDescriptor},
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
            .expect("adjacent kernel segments must validate");
        let user = SysretSegments::new(data(3, PrivilegeLevel::Ring3), code(4, PrivilegeLevel::Ring3))
            .expect("adjacent user segments must validate");
        let star = Star::new(kernel, user);

        assert_eq!(star.kernel().const_value(), 8);
        assert_eq!(star.user().const_value(), 16);

        let raw = star.raw();

        assert_eq!(raw.kernel().const_value(), 8);
        assert_eq!(raw.user().const_value(), 16);
    }

    #[test]
    fn raw_star_preserves_arbitrary_images() {
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

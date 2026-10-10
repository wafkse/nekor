//! FS and GS segment-base model-specific register representations.
//!
//! Raw types preserve register snapshots. Checked values carry canonical
//! linear addresses for the selected x86-64 address mode.

use crate::x86::msr::{Cpl0Access, detail::snapshotted};
#[cfg(target_arch = "x86_64")]
use crate::x86_64::paging::{La, LaMode};

snapshotted::snapshot!(
    /// `IA32_FS_BASE` register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct RawFsBase for (0xC000_0100) use Cpl0Access
);

/// Canonical FS base.
#[cfg(target_arch = "x86_64")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The stored value is a canonical linear address established by La construction
// or checked lifting from a raw FS-base snapshot.
pub struct FsBase(La);

#[cfg(target_arch = "x86_64")]
impl FsBase {
    /// Constructs an FS base from a linear address.
    #[inline]
    #[must_use]
    pub const fn new(address: La) -> Self {
        Self(address)
    }

    /// Lifts a raw FS-base snapshot when it is canonical under M.
    #[inline]
    #[must_use]
    pub const fn lift<M>(target_value: RawFsBase) -> Option<Self>
    where
        M: LaMode,
    {
        let RawFsBase(target_value) = target_value;

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

    /// Lowers this checked base into the register snapshot.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> RawFsBase {
        let Self(address) = self;

        RawFsBase::new(address.bits())
    }
}

snapshotted::snapshot!(
    /// `IA32_GS_BASE` register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct RawGsBase for (0xC000_0101) use Cpl0Access
);

/// Canonical GS base.
#[cfg(target_arch = "x86_64")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The stored value is a canonical linear address established by La construction
// or checked lifting from a raw GS-base snapshot.
pub struct GsBase(La);

#[cfg(target_arch = "x86_64")]
impl GsBase {
    /// Constructs a GS base from a linear address.
    #[inline]
    #[must_use]
    pub const fn new(address: La) -> Self {
        Self(address)
    }

    /// Lifts a raw GS-base snapshot when it is canonical under M.
    #[inline]
    #[must_use]
    pub const fn lift<M>(target_value: RawGsBase) -> Option<Self>
    where
        M: LaMode,
    {
        let RawGsBase(target_value) = target_value;

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

    /// Lowers this checked base into the register snapshot.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> RawGsBase {
        let Self(address) = self;

        RawGsBase::new(address.bits())
    }
}

snapshotted::snapshot!(
    /// `IA32_KERNEL_GS_BASE` register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct RawKernelGsBase for (0xC000_0102) use Cpl0Access
);

/// Canonical kernel GS base exchanged by SWAPGS.
#[cfg(target_arch = "x86_64")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The stored value is a canonical linear address established by La construction
// or checked lifting from a raw kernel-GS-base snapshot.
pub struct KernelGsBase(La);

#[cfg(target_arch = "x86_64")]
impl KernelGsBase {
    /// Constructs a kernel GS base from a linear address.
    #[inline]
    #[must_use]
    pub const fn new(address: La) -> Self {
        Self(address)
    }

    /// Lifts a raw kernel-GS-base snapshot when it is canonical under M.
    #[inline]
    #[must_use]
    pub const fn lift<M>(target_value: RawKernelGsBase) -> Option<Self>
    where
        M: LaMode,
    {
        let RawKernelGsBase(target_value) = target_value;

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

    /// Lowers this checked base into the register snapshot.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> RawKernelGsBase {
        let Self(address) = self;

        RawKernelGsBase::new(address.bits())
    }
}

#[cfg(test)]
mod tests {
    use super::{RawFsBase, RawGsBase, RawKernelGsBase};
    use crate::x86::msr::Msr;

    #[test]
    fn register_indices_match_architecture() {
        assert_eq!(RawFsBase::REGISTER.address(), 0xC000_0100);
        assert_eq!(RawGsBase::REGISTER.address(), 0xC000_0101);
        assert_eq!(RawKernelGsBase::REGISTER.address(), 0xC000_0102);
    }
}

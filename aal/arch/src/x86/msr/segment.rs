//! FS and GS segment-base MSR representations.
//!
//! Raw types preserve register images. Checked values carry canonical
//! linear addresses for the selected x86-64 address mode.

use super::{Msr, ReadWrite};
#[cfg(target_arch = "x86_64")]
use crate::x86_64::paging::{La, LaMode};

/// Exact IA32_FS_BASE register image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): Every u64 value is a representable raw FS-base image.
pub struct RawFsBase(u64);

impl RawFsBase {
    /// Constructs a raw FS-base image.
    #[inline]
    #[must_use]
    pub const fn new(target_value: u64) -> Self {
        Self(target_value)
    }

    /// Returns the raw FS-base image.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u64 {
        let Self(target_value) = self;

        target_value
    }
}

// SAFETY: RawFsBase is transparent over u64 and every bit pattern is valid.
unsafe impl Msr for RawFsBase {
    type Access = ReadWrite;

    const ADDRESS: u32 = 0xC000_0100;
}

impl super::private::Sealed for RawFsBase {}

/// Canonical FS base.
#[cfg(target_arch = "x86_64")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The stored value is a canonical linear address established by La construction
// or checked lifting from a raw FS-base image.
pub struct FsBase(La);

#[cfg(target_arch = "x86_64")]
impl FsBase {
    /// Constructs an FS base from a linear address.
    #[inline]
    #[must_use]
    pub const fn new(address: La) -> Self {
        Self(address)
    }

    /// Lifts a raw FS-base image when it is canonical under M.
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

    /// Lowers this checked base into the exact register image.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> RawFsBase {
        let Self(address) = self;

        RawFsBase::new(address.bits())
    }
}

/// Exact IA32_GS_BASE register image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): Every u64 value is a representable raw GS-base image.
pub struct RawGsBase(u64);

impl RawGsBase {
    /// Constructs a raw GS-base image.
    #[inline]
    #[must_use]
    pub const fn new(target_value: u64) -> Self {
        Self(target_value)
    }

    /// Returns the raw GS-base image.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u64 {
        let Self(target_value) = self;

        target_value
    }
}

// SAFETY: RawGsBase is transparent over u64 and every bit pattern is valid.
unsafe impl Msr for RawGsBase {
    type Access = ReadWrite;

    const ADDRESS: u32 = 0xC000_0101;
}

impl super::private::Sealed for RawGsBase {}

/// Canonical GS base.
#[cfg(target_arch = "x86_64")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The stored value is a canonical linear address established by La construction
// or checked lifting from a raw GS-base image.
pub struct GsBase(La);

#[cfg(target_arch = "x86_64")]
impl GsBase {
    /// Constructs a GS base from a linear address.
    #[inline]
    #[must_use]
    pub const fn new(address: La) -> Self {
        Self(address)
    }

    /// Lifts a raw GS-base image when it is canonical under M.
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

    /// Lowers this checked base into the exact register image.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> RawGsBase {
        let Self(address) = self;

        RawGsBase::new(address.bits())
    }
}

/// Exact IA32_KERNEL_GS_BASE register image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): Every u64 value is a representable raw kernel-GS-base image.
pub struct RawKernelGsBase(u64);

impl RawKernelGsBase {
    /// Constructs a raw kernel-GS-base image.
    #[inline]
    #[must_use]
    pub const fn new(target_value: u64) -> Self {
        Self(target_value)
    }

    /// Returns the raw kernel-GS-base image.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u64 {
        let Self(target_value) = self;

        target_value
    }
}

// SAFETY: RawKernelGsBase is transparent over u64 and every bit pattern is valid.
unsafe impl Msr for RawKernelGsBase {
    type Access = ReadWrite;

    const ADDRESS: u32 = 0xC000_0102;
}

impl super::private::Sealed for RawKernelGsBase {}

/// Canonical kernel GS base exchanged by SWAPGS.
#[cfg(target_arch = "x86_64")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): The stored value is a canonical linear address established by La construction
// or checked lifting from a raw kernel-GS-base image.
pub struct KernelGsBase(La);

#[cfg(target_arch = "x86_64")]
impl KernelGsBase {
    /// Constructs a kernel GS base from a linear address.
    #[inline]
    #[must_use]
    pub const fn new(address: La) -> Self {
        Self(address)
    }

    /// Lifts a raw kernel-GS-base image when it is canonical under M.
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

    /// Lowers this checked base into the exact register image.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> RawKernelGsBase {
        let Self(address) = self;

        RawKernelGsBase::new(address.bits())
    }
}

#[cfg(test)]
mod tests {
    use super::{Msr, RawFsBase, RawGsBase, RawKernelGsBase};

    #[test]
    fn register_indices_match_architecture() {
        assert_eq!(RawFsBase::REGISTER.address(), 0xC000_0100);
        assert_eq!(RawGsBase::REGISTER.address(), 0xC000_0101);
        assert_eq!(RawKernelGsBase::REGISTER.address(), 0xC000_0102);
    }
}

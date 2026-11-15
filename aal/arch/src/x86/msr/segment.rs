//! FS and GS segment-base register images.
//!
//! Each image preserves arbitrary bits. Address helpers prepare a value from a linear
//! address or check an observed value against the selected paging mode. Installation
//! still requires compatible machine controls and live segment-base ownership.

use crate::x86::{msr::detail::snapshotted, privilege::Cpl};
#[cfg(target_arch = "x86_64")]
use crate::x86_64::paging::{La, LaMode};

snapshotted::snapshot!(
    /// The `IA32_FS_BASE` FS base address model-specific register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MsrFsBase for (0xC000_0100) for<'a> &'a crate::x86::privilege::Cpl<0, T>
);

#[cfg(target_arch = "x86_64")]
impl MsrFsBase {
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
    /// The `IA32_GS_BASE` GS base address model-specific register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MsrGsBase for (0xC000_0101) for<'a> &'a crate::x86::privilege::Cpl<0, T>
);

#[cfg(target_arch = "x86_64")]
impl MsrGsBase {
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
    /// The `IA32_KERNEL_GS_BASE` kernel GS base address model-specific register snapshot.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MsrKernelGsBase for (0xC000_0102) for<'a> &'a Cpl<0, T>
);

#[cfg(target_arch = "x86_64")]
impl MsrKernelGsBase {
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
    use crate::x86::msr::{
        Msr,
        segment::{MsrFsBase, MsrGsBase, MsrKernelGsBase},
    };

    #[test]
    fn register_indices_match_architecture() {
        assert_eq!(MsrFsBase::REGISTER.address(), 0xC000_0100);
        assert_eq!(MsrGsBase::REGISTER.address(), 0xC000_0101);
        assert_eq!(MsrKernelGsBase::REGISTER.address(), 0xC000_0102);
    }
}

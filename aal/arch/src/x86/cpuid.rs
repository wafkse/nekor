//! Statically selected architectural CPUID records.

#[cfg(target_arch = "x86")]
use core::arch::x86::{__cpuid_count, CpuidResult};
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{__cpuid_count, CpuidResult};

/// output from the selected CPUID leaf and subleaf.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
// NOTE(invariant): LEAF and SUBLEAF are part of the Rust type. The wrapped core result is the
// complete architectural output from that exact selector pair.
pub struct Cpuid<const LEAF: u32, const SUBLEAF: u32 = 0>(CpuidResult);

impl<const LEAF: u32, const SUBLEAF: u32> Cpuid<LEAF, SUBLEAF> {
    /// Execute the selector pair encoded by this type.
    #[inline]
    #[must_use]
    pub fn read() -> Self {
        Self(__cpuid_count(LEAF, SUBLEAF))
    }

    /// Borrow the core CPUID result.
    #[inline]
    #[must_use]
    pub const fn result(&self) -> &CpuidResult {
        let &Self(ref result) = self;

        result
    }
}

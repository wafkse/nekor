//! Statically selected architectural CPUID records.

#[cfg(target_arch = "x86")]
use core::arch::x86::{__cpuid_count, CpuidResult};
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{__cpuid_count, CpuidResult};

/// Complete output from one compile-time-selected CPUID leaf and subleaf.
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

    /// Borrow the complete core CPUID result.
    #[inline]
    #[must_use]
    pub const fn result(&self) -> &CpuidResult {
        let &Self(ref result) = self;

        result
    }

    /// Construct one synthetic record inside the x86 architecture implementation.
    // NOTE(rationale): Sibling architecture tests need controlled records without exposing record
    // forgery through the supported public API.
    #[cfg(any(test, miri))]
    pub(in crate::x86) const fn synthetic(result: CpuidResult) -> Self {
        Self(result)
    }
}

#[cfg(test)]
mod tests {
    use core::arch::x86_64::CpuidResult;

    use super::Cpuid;

    #[test]
    fn static_selector_identity_preserves_complete_register_domains() {
        let registers = Cpuid::<{ u32::MAX }, { u32::MAX }>::synthetic(CpuidResult {
            eax: 1,
            ebx: 2,
            ecx: 3,
            edx: 4,
        });

        let result = registers.result();

        assert_eq!((result.eax, result.ebx), (1, 2));
        assert_eq!((result.ecx, result.edx), (3, 4));
    }
}

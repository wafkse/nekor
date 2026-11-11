//! x86 CPUID selectors and returned register values.

#[macro_use]
mod detail {
    //! Implementation details for x86 CPUID values.

    /// Generate raw constructors and accessors for scalar x86 values.
    macro_rules! scalar {
        ($target_name:ident($target_repr:ty)) => {
            tokel::stream! {
                impl $target_name {
                    /// Preserve the target raw scalar value.
                    #[inline]
                    #[must_use]
                    pub const fn new(value: $target_repr) -> Self {
                        Self(value)
                    }

                    /// Return the raw scalar value.
                    #[inline]
                    #[must_use]
                    pub const fn value(self) -> $target_repr {
                        let Self(value) = self;

                        value
                    }
                }
            }
        };
    }
}

/// x86 CPUID function number, distinct from its subleaf index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): None.
pub struct CpuidFunction(u32);

/// x86 CPUID subleaf index, distinct from its function number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): None.
pub struct CpuidSubleaf(u32);

scalar!(CpuidFunction(u32));
scalar!(CpuidSubleaf(u32));

/// Four register values returned by an x86 CPUID query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CpuidRegisters {
    /// Value returned in EAX.
    pub eax: u32,

    /// Value returned in EBX.
    pub ebx: u32,

    /// Value returned in ECX.
    pub ecx: u32,

    /// Value returned in EDX.
    pub edx: u32,
}

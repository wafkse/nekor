//! CPUID queries and register values used to configure KVM virtual processors.
//!
//! Query attributes preserve unknown bits when converting between entry representations.

use nekor_bitwise::prelude::{Bit, BitMut, Counterpart};
use eustace_arch::x86::cpuid::{CpuidFunction, CpuidRegisters, CpuidSubleaf};

use crate::sys::ffi::{binding::kvm, lower::arch::x86};

mod detail {
    //! Implementation details for KVM CPUID values.

    /// Return the bit index encoded by a one-bit flag mask.
    pub const fn bit_index_from_mask(mask: u32) -> usize {
        assert!(mask.is_power_of_two(), "the flag mask must contain exactly one bit");

        mask.trailing_zeros() as usize
    }
}

/// Significant-index flag in a KVM CPUID query.
pub type CpuidSignificantIndex<'value> =
    Bit<'value, u32, { detail::bit_index_from_mask(kvm::preferred::KVM_CPUID_FLAG_SIGNIFCANT_INDEX) }>;

/// Mutable significant-index flag in a KVM CPUID query.
pub type CpuidSignificantIndexMut<'value> = <CpuidSignificantIndex<'value> as Counterpart>::Mut;

/// Stateful-function flag in a KVM CPUID query.
pub type CpuidStatefulFunction<'value> =
    Bit<'value, u32, { detail::bit_index_from_mask(kvm::preferred::KVM_CPUID_FLAG_STATEFUL_FUNC) }>;

/// Mutable stateful-function flag in a KVM CPUID query.
pub type CpuidStatefulFunctionMut<'value> = <CpuidStatefulFunction<'value> as Counterpart>::Mut;

/// State-read-next flag in a KVM CPUID query.
pub type CpuidStateReadNext<'value> =
    Bit<'value, u32, { detail::bit_index_from_mask(kvm::preferred::KVM_CPUID_FLAG_STATE_READ_NEXT) }>;

/// Mutable state-read-next flag in a KVM CPUID query.
pub type CpuidStateReadNextMut<'value> = <CpuidStateReadNext<'value> as Counterpart>::Mut;

/// KVM-specific attributes attached to a CPUID query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
// NOTE(invariant): None.
pub struct CpuidFlags(u32);

impl CpuidFlags {
    /// Preserve a raw KVM CPUID attribute word.
    #[inline]
    #[must_use]
    pub const fn new(target_value: u32) -> Self {
        Self(target_value)
    }

    /// Return the raw attribute word, including bits unknown to this release.
    #[inline]
    #[must_use]
    pub const fn bits(&self) -> u32 {
        let &Self(target_value) = self;

        target_value
    }

    /// Borrow the significant-index attribute.
    #[inline]
    #[must_use]
    pub const fn significant_index(&self) -> CpuidSignificantIndex<'_> {
        let Self(target_value) = self;

        Bit::wrap(target_value)
    }

    /// Mutably borrow the significant-index attribute.
    #[inline]
    #[must_use]
    pub const fn significant_index_mut(&mut self) -> CpuidSignificantIndexMut<'_> {
        let Self(target_value) = self;

        BitMut::wrap(target_value)
    }

    /// Borrow the stateful-function attribute.
    #[inline]
    #[must_use]
    pub const fn stateful_function(&self) -> CpuidStatefulFunction<'_> {
        let Self(target_value) = self;

        Bit::wrap(target_value)
    }

    /// Mutably borrow the stateful-function attribute.
    #[inline]
    #[must_use]
    pub const fn stateful_function_mut(&mut self) -> CpuidStatefulFunctionMut<'_> {
        let Self(target_value) = self;

        BitMut::wrap(target_value)
    }

    /// Borrow the state-read-next attribute.
    #[inline]
    #[must_use]
    pub const fn state_read_next(&self) -> CpuidStateReadNext<'_> {
        let Self(target_value) = self;

        Bit::wrap(target_value)
    }

    /// Mutably borrow the state-read-next attribute.
    #[inline]
    #[must_use]
    pub const fn state_read_next_mut(&mut self) -> CpuidStateReadNextMut<'_> {
        let Self(target_value) = self;

        BitMut::wrap(target_value)
    }
}

/// Leaf selection and KVM attributes for a CPUID query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CpuidQuery {
    /// CPUID function selected by the query.
    pub function: CpuidFunction,

    /// CPUID subleaf selected by the query.
    pub subleaf: CpuidSubleaf,

    /// KVM-specific query attributes, including unknown bits.
    pub flags: CpuidFlags,
}

/// A CPUID query and its returned register values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CpuidEntry {
    /// Function, subleaf, and KVM query attributes.
    pub query: CpuidQuery,

    /// Registers returned for the associated CPUID query.
    pub registers: CpuidRegisters,
}

impl CpuidEntry {
    /// Construct an entry from its ABI representation, preserving unknown query attributes.
    #[inline]
    #[must_use]
    pub const fn lift(entry: x86::cpuid::Entry) -> Self {
        let x86::cpuid::Entry { query, registers } = entry;

        let x86::cpuid::Query { leaf, subleaf, flags } = query;

        let x86::cpuid::Registers { eax, ebx, ecx, edx } = registers;

        let function = CpuidFunction::new(leaf);
        let subleaf = CpuidSubleaf::new(subleaf);
        let flags = CpuidFlags::new(flags);

        let query = CpuidQuery {
            function,
            subleaf,
            flags,
        };
        let registers = CpuidRegisters { eax, ebx, ecx, edx };

        Self { query, registers }
    }

    /// Return the ABI representation of the query and register values.
    #[inline]
    #[must_use]
    pub const fn raw(&self) -> x86::cpuid::Entry {
        let &Self {
            query: CpuidQuery {
                function,
                subleaf,
                flags,
            },
            registers: CpuidRegisters { eax, ebx, ecx, edx },
        } = self;

        let leaf = function.value();
        let subleaf = subleaf.value();
        let flags = flags.bits();

        let query = x86::cpuid::Query { leaf, subleaf, flags };
        let registers = x86::cpuid::Registers { eax, ebx, ecx, edx };

        x86::cpuid::Entry { query, registers }
    }
}

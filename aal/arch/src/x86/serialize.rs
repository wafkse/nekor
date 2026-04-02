//! Architectural and Processor State serialization.
//!
//! # Methodology
//!
//! We use the `serialize` instruction to ensure that all previous
//! instructions have completed before proceeding. This is particularly
//! important for code patching, where we need to ensure that the new code is
//! executed after the old code has completed.
//!
//! In the case that the instruction is not available, we fall back to a
//! self-iret, which is a self-interrupt return instruction that performs a full
//! architectural pipeline flush and consequent serialize.

use core::arch;

#[cfg(target_arch = "x86")]
use arch::x86::__cpuid;

#[cfg(target_arch = "x86_64")]
use arch::x86_64::__cpuid;

/// An uninhabited type to act as a cohesive entity for all processor
/// synchronization that depends on a self-*Interrupt Return* methodology.
///
/// This encompasses distinct methods of pipeline and microarchitectural
/// serialization that are generally differing in terms of availability and
/// performance.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub enum IRet {}

impl IRet {
    /// Perform a self-*Interrupt Return* in a core-local manner.
    ///
    /// This is meant to perform a full architectural pipeline flush and
    /// consequent serialize.
    ///
    /// This method is preferred over other methods, as it is available at any
    /// privilege level, making it available to both usermode-based testing and
    /// paravirtualization.
    #[cfg(target_arch = "x86")]
    #[inline(always)]
    pub fn x86() {
        // SAFETY: This is both memory- and architecturally-safe.
        unsafe {
            arch::asm!(
                "pushfl",
                "popl {0:e}",
                "pushl %cs",
                "call 2f",
                "2:",
                "addl $(2f - 2b), (%esp)",
                "pushl {0:e}",
                "iretl",
                "2:",
                lateout(reg) _,
                options(nomem, preserves_flags, att_syntax)
            );
        }
    }

    /// Perform a self-*Interrupt Return* in a core-local manner.
    ///
    /// This is meant to perform a full architectural pipeline flush and
    /// consequent serialize.
    ///
    /// This method is preferred over other methods, as it is available at any
    /// privilege level, without requiring extra architectural features, making
    /// it available to both usermode-based testing and paravirtualization.
    #[cfg(target_arch = "x86_64")]
    #[inline(always)]
    pub fn x86_64() {
        // SAFETY: This is both memory- and architecturally-safe.
        unsafe {
            arch::asm!(
                "movw %ss, {0:x}",
                "pushq {0:r}",
                "xorq {0:r}, {0:r}",
                "pushq %rsp",
                "pushfq",
                "popq {0:r}",
                // NOTE: We adjust the in-stack `sp` here to match the current one.
                "addq $8, (%rsp)",
                "pushq {0:r}",
                "xorq {0:r}, {0:r}",
                "movw %cs, {0:x}",
                "pushq {0:r}",
                "leaq 2f(%rip), {0:r}",
                "pushq {0:r}",
                "iretq",
                "2:",
                lateout(reg) _,
                options(nomem, preserves_flags, att_syntax)
            );
        }
    }
}

/// An uninhabited type to act as a cohesive entity for all processor
/// synchronization that depends on a *cpuid* execution methodology.
///
/// This instruction is ubiquitous (*>= i686*) and can be used to serialize the
/// pipeline and microarchitectural state of the processor.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub enum CpuId {}

impl CpuId {
    /// A no-op function whose instruction stream is guaranteed to be executed
    /// in a serialized manner.
    ///
    /// This function is useful for ensuring that all preceding instructions
    /// have completed before any subsequent instructions are executed.
    ///
    /// This is universally available on all modern *x86* processors (>= i686),
    /// therefore, no `EFLAGS.ID` check requirement is imposed.
    #[inline]
    pub fn ubiquitous() {
        let _ = __cpuid(0x0000_0000);
    }
}

/// An uninhabited type to act as a cohesive entity for all processor
/// synchronization that depends on the execution of an architecturally-defined
/// serializing instruction.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub enum Serialize {}

impl Serialize {
    /// A no-op function whose instruction stream is guaranteed to be executed
    /// in a serialized manner.
    ///
    /// This function is useful for ensuring that all preceding instructions
    /// have completed before any subsequent instructions are executed.
    ///
    /// # Safety
    ///
    /// This depends on the `serialize` instruction to be available.
    ///
    /// Use the built-in feature `Serialize` (in the feature AAL crate) type to
    /// determine if this instruction is available.
    #[inline]
    pub unsafe fn now() {
        // SAFETY: The required feature has been asserted by the caller, and the
        // `serialize` instruction does not raise any CPU-level exceptions.
        let _ = unsafe { crate::x86::instruction::serialize() };
    }
}

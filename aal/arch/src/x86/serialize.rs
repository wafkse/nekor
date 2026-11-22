//! Local instruction serialization.
//!
//! These operations serialize the calling processor's instruction stream. They do not
//! notify other processors, wait for remote execution to stop or establish a complete
//! cross-modifying-code publication protocol.

use core::{arch, hint::black_box};

#[cfg(target_arch = "x86")]
use arch::x86::__cpuid;
#[cfg(target_arch = "x86_64")]
use arch::x86_64::__cpuid;

use crate::x86::instruction;

/// Serialization by returning to the current code segment through `iret`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub enum IRet {}

impl IRet {
    /// Perform a self-*Interrupt Return* in a core-local manner.
    ///
    /// The assembly constructs a return frame for the current execution context.
    /// It does not transfer to another privilege level.
    #[cfg(target_arch = "x86")]
    #[inline]
    pub fn x86() {
        // SAFETY: The return frame is built on the current stack from the active
        // segment selectors and flags, and its destination is the local continuation.
        //
        // The respective stack adjustments restore the caller's stack at that continuation.
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
    /// The assembly constructs a return frame for the current execution context.
    /// It does not transfer to another privilege level.
    #[cfg(target_arch = "x86_64")]
    #[inline]
    pub fn x86_64() {
        // SAFETY: The return frame is built on the current stack from the active
        // segment selectors and flags, and its destination is the local continuation.
        //
        // The respective stack adjustments restore the caller's stack at that continuation.
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
    /// Serialize local instruction execution.
    ///
    /// This function is useful for ensuring that all preceding instructions
    /// have completed before any subsequent instructions are executed.
    ///
    /// This is universally available on all modern *x86* processors (>= i686),
    /// therefore, no `EFLAGS.ID` check requirement is imposed.
    #[inline]
    pub fn ubiquitous() {
        black_box(__cpuid(0x0000_0000));
    }
}

/// An uninhabited type to act as a cohesive entity for all processor
/// synchronization that depends on the execution of an architecturally-defined
/// serializing instruction.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub enum Serialize {}

impl Serialize {
    /// Serialize local instruction execution.
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
        unsafe { instruction::serialize() };
    }
}

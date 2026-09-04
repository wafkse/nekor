//! Raw access to `x86` system-level and other miscenalleous instructions.

use core::arch;

pub mod gdtr {
    //! Instructions related to the *Global Descriptor Table Register*.

    use core::arch;

    use crate::x86::{
        descriptor::{DescriptorTablePointer, Gdt},
        mode::Bits32,
    };

    /// Load the *Protected Mode* *Global Descriptor Table*.
    ///
    /// The use of this system instruction must be left as a last resort.
    ///
    /// # Safety
    ///
    /// - Interrupts must be masked for the current core while the *Global Descriptor Table* is
    ///   being updated.
    /// - The *Global Descriptor Table Register* must describe a valid and accessible *Global
    ///   Descriptor Table* immediately after execution of this instruction.
    /// - All segment selectors currently loaded into segment registers (including `CS`, `DS`, `SS`,
    ///   `ES`, `FS`, `GS`) must remain valid with respect to the newly loaded *Global Descriptor
    ///   Table*.
    /// - In particular, the active *Code Segment* (`CS`) selector must point to a present,
    ///   executable code segment descriptor in the new *Global Descriptor Table*.
    /// - The current *Current Privilege Level* must be zero.
    ///
    /// ## Correctness
    ///
    /// - If any segment register (e.g., `DS`, `SS`, `ES`, `FS`, `GS`) contains a selector that does
    ///   not reference a valid descriptor in the newly loaded *Global Descriptor Table*, subsequent
    ///   memory accesses through that segment may trigger a *General Protection Fault* (`#GP`) or
    ///   *Segment Not Present Fault* (`#NP`).
    /// - If the `CS` selector does not reference a valid code segment in the new *Global Descriptor
    ///   Table*, execution cannot safely continue and will typically result in an unrecoverable
    ///   fault.
    /// - Any *Gate Descriptor* in the *Interrupt Descriptor Table* (e.g., interrupt gates, trap
    ///   gates, task gates) that contains a stale segment selector will also fail when invoked,
    ///   typically causing a *General Protection Fault* or a double fault if the condition arises
    ///   during exception handling.
    /// - To avoid stale selectors, it is common practice to reload all segment registers (except
    ///   `CS`, which requires a far jump, far return, or, in cases involving privilege level
    ///   transitions, a call gate) immediately after loading a new *Global Descriptor Table*.
    ///
    /// # Last Resort Use Only
    ///
    /// - Prefer using a high-level *GDT management library* where possible. These abstractions
    ///   guarantee that selectors and descriptors remain consistent, and they handle segment
    ///   register reloads safely.
    /// - For advanced cases, consider using a *Descriptor Table Swapchain* to stage and atomically
    ///   install new descriptor tables without exposing the system to a window of invalid
    ///   selectors.
    /// - Direct use of `lgdtd` is only advisable in the lowest-level runtime code (bootloaders,
    ///   critical subsystems) where no safer abstraction is available.
    #[inline]
    pub unsafe fn lgdtd(target_register: &'static DescriptorTablePointer<Gdt, Bits32>) {
        // SAFETY: Caller guarantees safety for current architectual state.
        unsafe {
            arch::asm!(
                "lgdtd ({})",
                in(reg) target_register,
                options(nostack, readonly, preserves_flags, att_syntax)
            );
        }
    }
    /// Store the *Protected Mode* *Global Descriptor Table* into a *Global
    /// Descriptor Table Register*.
    ///
    /// # Safety
    ///
    /// This operation is safe, and available at any privilege level, however,
    /// if *User-Mode Instruction Prevention* is enabled, the *Current
    /// Privilege Level* must be zero.
    #[inline]
    pub unsafe fn sgdtd(target_register: &mut DescriptorTablePointer<Gdt, Bits32>) {
        // SAFETY: The operation is inherently safe, presence of UMIP is bypassed with
        // *Current Privilege Level* of zero.
        unsafe {
            arch::asm!(
                "sgdtd ({})",
                in(reg) target_register,
                options(nostack, preserves_flags, att_syntax)
            );
        }
    }
}

pub mod idtr {
    //! Instructions related to the *Interrupt Descriptor Table Register*.

    use core::arch;

    use crate::x86::{
        descriptor::{DescriptorTablePointer, Idt},
        mode::Bits32,
    };

    /// Load the *Protected Mode* *Interrupt Descriptor Table*.
    ///
    /// # Safety
    ///
    /// - Interrupts must be masked for the current core.
    /// - The *Non-maskable-interrupt* (NMI) *Gate Descriptor* in the specified *Interrupt
    ///   Descriptor Table* described by the *Interrupt Descriptor Table Register* must be present
    ///   and immediately capable of handling a *NMI*.
    /// - The current *Current Privilege Level* must be zero.
    #[inline]
    pub unsafe fn lidtd(target_register: &'static DescriptorTablePointer<Idt, Bits32>) {
        // SAFETY: Caller guarantees safety for current architectural state.
        unsafe {
            arch::asm!(
                "lidtd ({})",
                in(reg) target_register,
                options(nostack, readonly, preserves_flags, att_syntax)
            );
        }
    }

    /// Store the *Protected Mode* *Global Descriptor Table* into a *Global
    /// Descriptor Table Register*.
    ///
    /// # Safety
    ///
    /// This operation is safe, and available at any privilege level, however,
    /// if *User-Mode Instruction Prevention* is enabled, the *Current
    /// Privilege Level* must be zero.
    #[inline]
    pub unsafe fn sidtd(target_register: &mut DescriptorTablePointer<Idt, Bits32>) {
        // SAFETY: The operation is inherently safe, presence of UMIP is bypassed with
        // *Current Privilege Level* of zero.
        unsafe {
            arch::asm!(
                "sidtd ({})",
                in(reg) target_register,
                options(nostack, preserves_flags, att_syntax)
            );
        }
    }
}

/// Serialize *Instruction Execution*, *Architectural State*, and *Memory
/// Accesses* for this processor.
///
/// # Safety
///
/// The `CPUID.07H.0H:EDX.SERIALIZE[bit 14]` feature bit must be set.
#[inline]
pub unsafe fn serialize() {
    // SAFETY: The instruction is supported by the processor as guaranteed by
    // the caller.
    unsafe {
        arch::asm!("serialize", options(att_syntax, nomem, nostack, preserves_flags));
    }
}

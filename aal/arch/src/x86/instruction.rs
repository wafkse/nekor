//! Raw access to `x86` system-level and other miscenalleous instructions.

use core::arch;

pub mod relax {
    //! Instructions that reduce processo activity while waiting.

    use core::arch;

    use nekor_bitwise::prelude::{Bit, Counterpart, Field, State};

    use super::info::{TscDeadlineEax, TscDeadlineEdx};

    /// Indicates to the processor that the code sequence is a spin-wait loop.
    ///
    /// This instruction is useful for reducing power consumption and improving
    /// performance in certain scenarios.
    #[inline]
    pub fn pause() {
        // SAFETY: PAUSE only changes the processor's spin-wait behavior.
        unsafe { arch::asm!("pause", options(nomem, nostack, preserves_flags, att_syntax)) };
    }

    /// Execute consecutive `pause` instructions grouped into chains.
    ///
    /// A chain contains `PAUSE_COUNT` consecutive `pause` instructions. Callers may
    /// treat one chain as a wait quantum and use `CHAIN_COUNT` to determine how many
    /// quanta execute between timestamp checks.
    ///
    /// The grouping introduces no branches or timestamp reads between chains.
    /// Both counts are const parameters so callers can select their grouping
    /// without generic const arithmetic. If either count is zero, no instructions
    /// are emitted.
    #[inline]
    pub fn chain<const PAUSE_COUNT: usize, const CHAIN_COUNT: usize>() {
        // SAFETY: Each PAUSE only changes the processor's spin-wait behavior.
        unsafe {
            arch::asm!(
                ".rept {chain_count}",
                ".rept {pause_count}",
                "pause",
                ".endr",
                ".endr",
                chain_count = const CHAIN_COUNT,
                pause_count = const PAUSE_COUNT,
                options(nomem, nostack, preserves_flags, att_syntax)
            );
        }
    }

    /// A C-State for the processor to enter during a [`tpause`] instruction.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Cstate {
        /// A sub-C-State of `C0` that is optimized for faster wakeup latency, but
        /// incurs a higher power consumption.
        C0_1,

        /// A sub-C-State of `C0` that is optimized for low power consumption, but
        /// incurs additional wakeup latency.
        ///
        /// This guarantees an increase in performance of other logical threads in
        /// the same processor core.
        C0_2,
    }

    /// Direct the processor to enter an implementation-defined low-power state
    /// until:
    ///
    /// - The timestamp counter reaches the specified deadline.
    /// - The processor thread is interrupted.
    ///
    /// This instruction is useful for reducing power consumption and improving
    /// performance in certain scenarios.
    ///
    /// # Safety
    ///
    /// This requires the `Waitpkg` extension to be present.
    ///
    /// The feature is represented by `nekor_aal_feature::arch::x86::qualified::Waitpkg`.
    #[inline]
    pub unsafe fn tpause(wait_state: Cstate, target_deadline: u64) {
        type TpauseCState<'a> = Bit<'a, u32, 0>;
        type TpauseCStateMut<'a> = <TpauseCState<'a> as Counterpart>::Mut;

        type TpauseCStateReserved<'a> = Field<'a, 1, 31, u32>;
        type TpauseCStateReservedMut<'a> = <TpauseCStateReserved<'a> as Counterpart>::Mut;

        let edx = TscDeadlineEdx::wrap(&target_deadline).value();
        let eax = TscDeadlineEax::wrap(&target_deadline).value();

        let mut ecx = u32::MIN;

        TpauseCStateMut::wrap(&mut ecx).set(if wait_state == Cstate::C0_1 {
            State::Set
        } else {
            State::Cleared
        });

        // NOTE: Keep all reserved bits clear.
        TpauseCStateReservedMut::wrap(&mut ecx).merge(u32::MIN);

        // SAFETY: The caller establishes WAITPKG support.
        unsafe {
            arch::asm!(
                "tpause %ecx",
                in("edx") edx,
                in("eax") eax,
                in("ecx") ecx,
                options(nomem, nostack, att_syntax)
            );
        }
    }
}

pub mod info {
    //! Instructions that read processor information and counters.

    use core::arch;

    use nekor_bitwise::prelude::{Counterpart, Field};

    /// The lower-half of the timestamp counter deadline.
    ///
    /// This is to be located in the `eax` register.
    pub type TscDeadlineEax<'a> = Field<'a, 0, 31, u64>;

    /// The higher-half of the timestamp counter deadline.
    ///
    /// This is to be located in the `edx` register.
    pub type TscDeadlineEdx<'a> = Field<'a, 32, 63, u64>;

    /// The mutable alternative to [`TscDeadlineEax`].
    pub type TscDeadlineEaxMut<'a> = <TscDeadlineEax<'a> as Counterpart>::Mut;

    /// The mutable alternative to [`TscDeadlineEdx`].
    pub type TscDeadlineEdxMut<'a> = <TscDeadlineEdx<'a> as Counterpart>::Mut;

    /// Reads the current value of the time-stamp counter.
    ///
    /// # Safety
    ///
    /// This is a partially-privileged instruction, therefore, the caller must
    /// satisfy either of:
    ///
    /// - The *Time Stamp Disable* (TSD) bit in the `cr4` control register is cleared.
    /// - ..or the current *Current Privilege Level* is `Ring0`.
    #[inline]
    #[must_use]
    pub unsafe fn rdtsc() -> u64 {
        let (edx, eax): (u32, u32);

        // SAFETY: The caller permits RDTSC in this processor context.
        unsafe {
            arch::asm!(
                "rdtsc",
                lateout("eax") eax,
                lateout("edx") edx,
                options(nomem, nostack, preserves_flags, att_syntax)
            );
        }

        let mut target_value = u64::MIN;

        TscDeadlineEaxMut::wrap(&mut target_value).merge(eax);
        TscDeadlineEdxMut::wrap(&mut target_value).merge(edx);

        target_value
    }
}

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

/// Fence preceding loads against subsequent loads.
///
/// This operation also acts as a compiler barrier for memory accesses.
#[inline]
pub fn lfence() {
    // SAFETY: LFENCE is available on the supported processor baseline. Omitting `nomem`
    // prevents compiler memory motion across the fence.
    unsafe {
        arch::asm!("lfence", options(nostack, preserves_flags, att_syntax));
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

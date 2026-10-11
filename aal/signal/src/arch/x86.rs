//! Support for the `x86` architecture.

// TODO: Implement base data structures in bare arch crate. This needs APIC and
// x2APIC support for IPIs.

use core::{arch, ptr::NonNull};

pub use nekor_aal_arch::x86::instruction::{
    info::{TscDeadlineEax, TscDeadlineEaxMut, TscDeadlineEdx, TscDeadlineEdxMut, rdtsc},
    relax::{Cstate, pause, tpause},
};
use nekor_bitwise::prelude::{Bit, Counterpart, State};

#[cfg(not(miri))]
pub mod monitor;

/// Monitor a select memory address through the `monitor` instruction.
///
/// # Safety
///
/// - The caller must have a `Current Privilege Level` of `0`, however, this is not required if the
///   (in the AMD namespace-based naming convention) `Core::X86::Msr::HWCR[MonMwaitUserEn]`
///   model-specific register bit is set.
///
/// - The [`Monitor`] feature must be present.
///
///
/// - The specified address must be readable in the active processor context, however, note that
///   this is not the same as Rust-specific guarantees, the processor simply performs access
///   control, but does not fetch or write to the memory in any way.
///
/// [`Monitor`]: nekor_aal_feature::arch::x86::qualified::Monitor
#[inline]
pub unsafe fn monitor<T>(target_address: NonNull<T>) {
    // SAFETY: Pointer is valid as guaranteed by caller and the required
    // `Current Privilege Level` is satisfied.
    #[cfg(target_pointer_width = "32")]
    unsafe {
        arch::asm!(
            // NOTE(ordering): Commit any pending writes that might trample with the monitor hardware.
            "sfence",
            "monitor",
            in("eax") target_address.as_ptr(),
            in("ecx") usize::MIN,
            in("edx") usize::MIN,
            // Keep the condition load after monitor engagement.
            options(nostack, preserves_flags, att_syntax)
        )
    }

    // SAFETY: [see above]
    #[cfg(target_pointer_width = "64")]
    unsafe {
        arch::asm!(
            // NOTE(ordering): [see other block]
            "sfence",
            "monitor",
            in("rax") target_address.as_ptr(),
            in("rcx") usize::MIN,
            in("rdx") usize::MIN,
            // Keep the condition load after monitor engagement.
            options(nostack, preserves_flags, att_syntax)
        );
    }
}

/// Wait until either:
///
/// - An interrupt is served to this core.
/// - The monitor hardware is trampled.
///
/// # Safety
///
/// - The caller must have a `Current Privilege Level` of `0`.
/// - The [`Monitor`] feature must be present.
///
/// [`Monitor`]: nekor_aal_feature::arch::x86::qualified::Monitor
#[inline]
pub unsafe fn mwait() {
    // SAFETY: Privilege level is guaranteed by caller.
    unsafe {
        arch::asm!(
            "mwait",
            in("eax") u32::MIN,
            in("ecx") u32::MIN,
            options(nostack, att_syntax)
        );
    }
}

/// An AMD-specific `hardware monitor` instruction comparable to the [`monitor()`]
/// instruction.
///
/// However, in contrast to the regular `monitor` instruction, this is
/// unaffected by the (in the AMD namespace-based naming convention)
/// `Core::X86::Msr::HWCR[MonMwaitUserEn|MonMwaitDis]` model-specific register
/// bits.
/// In other words, the `monitor|mwaitx` family of instructions can be used at
/// any *Current Privilege Level*.
///
/// # Safety
///
/// To guarantee the safety of this operation, the caller must ensure that all
/// of the following are satisfied:
///
/// - The condition (in AMD CPUID leaf-as-function notation) `CPUID Fn8000_0001_ECX[MONITORX](bit
///   29) = 1` to be true.
/// - The specified address must be readable in the active processor context. Note that this is not
///   the same as Rust-specific guarantees, the processor simply performs access control, but does
///   not fetch or write to the memory in any way.
#[inline]
pub unsafe fn monitorx<T>(target_address: NonNull<T>) {
    /// The hint flags provided to the `monitorx` instruction.
    ///
    /// There are no existing hints for this instruction.
    const MONITORX_HINT: u32 = u32::MIN;

    /// The extensions provided to the `monitorx` instruction.
    ///
    /// There are no existing extensions for this instruction.
    const MONITORX_EXTENDED: u32 = u32::MIN;

    // SAFETY: The caller guarantees MONITORX support and an accessible address.
    #[cfg(target_pointer_width = "64")]
    unsafe {
        arch::asm!(
            // NOTE(asm): Seems that no assembler in our toolchain supports the AMD-specific `monitorx` extension.
            "monitorx",
            // NOTE: AMD indicates that there are no existing extensions or hints for this instruction.
            //
            // For further information, see the documentation for the `monitorx` instruction on the AMD Developer Manual.
            in("edx") MONITORX_HINT,
            in("ecx") MONITORX_EXTENDED,
            in("rax") target_address.as_ptr(),
            options(nostack, att_syntax)
        );
    }

    // SAFETY: The same requirement applies in 32-bit mode.
    #[cfg(target_pointer_width = "32")]
    unsafe {
        arch::asm!(
            "monitorx",
            in("edx") MONITORX_HINT,
            in("ecx") MONITORX_EXTENDED,
            in("eax") target_address.as_ptr(),
            options(nostack, att_syntax)
        );
    }
}

/// Wait on a range previously armed by [`monitorx`], with a bounded timeout.
///
/// `timeout` is a relative count in software P0 clocks.
///
/// # Safety
///
/// The current CPU must support MONITORX/MWAITX, and the caller must have
/// executed `monitorx` on this CPU before waiting.
#[inline]
pub unsafe fn mwaitx(timeout: u32) {
    type MwaitxTimer<'a> = Bit<'a, u32, 1>;
    type MwaitxTimerMut<'a> = <MwaitxTimer<'a> as Counterpart>::Mut;

    let mut ecx = 0_u32;

    MwaitxTimerMut::wrap(&mut ecx).set(State::Set);

    // SAFETY: The caller establishes support and the matching monitor state.
    unsafe {
        #[cfg(target_pointer_width = "64")]
        arch::asm!(
            "pushq %rbx",
            "movl {timeout:e}, %ebx",
            "mwaitx",
            "popq %rbx",
            timeout = in(reg) timeout,
            in("eax") 0_u32,
            in("ecx") ecx,
            options(att_syntax)
        );

        #[cfg(target_pointer_width = "32")]
        arch::asm!(
            "pushl %ebx",
            "movl {timeout:e}, %ebx",
            "mwaitx",
            "popl %ebx",
            timeout = in(reg) timeout,
            in("eax") 0_u32,
            in("ecx") ecx,
            options(att_syntax)
        );
    }
}

/// Arm a user-level monitor on the supplied readable address.
///
/// # Safety
///
/// The current CPU must support WAITPKG, and `target_address` must be readable
/// in the current address space.
#[inline]
pub unsafe fn umonitor<T>(target_address: NonNull<T>) {
    // SAFETY: The caller establishes WAITPKG support and address accessibility.
    unsafe {
        #[cfg(target_pointer_width = "64")]
        arch::asm!("umonitor %rax", in("rax") target_address.as_ptr(), options(nostack, att_syntax));

        #[cfg(target_pointer_width = "32")]
        arch::asm!("umonitor %eax", in("eax") target_address.as_ptr(), options(nostack, att_syntax));
    }
}

/// Wait on the address previously armed by [`umonitor`] until a deadline or wake.
///
/// # Safety
///
/// The current CPU must support WAITPKG, TSC access must be enabled, and the
/// matching user monitor must have been armed on this CPU.
#[inline]
pub unsafe fn umwait(wait_state: Cstate, target_deadline: u64) {
    let eax = TscDeadlineEax::wrap(&target_deadline).value();
    let edx = TscDeadlineEdx::wrap(&target_deadline).value();
    let ecx = u32::from(wait_state == Cstate::C0_1);

    // SAFETY: The caller establishes WAITPKG and TSC access.
    unsafe {
        arch::asm!(
            "umwait %ecx",
            in("eax") eax,
            in("edx") edx,
            in("ecx") ecx,
            options(nostack, att_syntax)
        );
    }
}

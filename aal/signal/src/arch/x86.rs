//! Support for the `x86` architecture.

// TODO: Implement base data structures in bare arch crate. This needs APIC and
// x2APIC support for IPIs.

use nekor_bitwise::prelude::{BitMut, Counterpart, Field, FieldMut, State};

use core::{arch, ptr::NonNull};

/// Monitor a select memory address through the `monitor` instruction.
///
/// # Safety
///
/// - The caller must have a `Current Privilege Level` of `0`, however, this is
///   not required if the (in the AMD namespace-based naming convention)
///   `Core::X86::Msr::HWCR[MonMwaitUserEn]` model-specific register bit is set.
///
/// - The [`Monitor`] feature must be present.
///
///
/// - The specified address must be readable in the active processor context,
///   however, note that this is not the same as Rust-specific guarantees, the
///   processor simply performs access control, but does not fetch or write to
///   the memory in any way.
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
            options(nostack, readonly, preserves_flags)
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
            options(nostack, readonly, preserves_flags)
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
    unsafe { arch::asm!("mwait", options(nostack, readonly, att_syntax)) }
}

/// Indicates to the processor that the code sequence is a spin-wait loop.
///
/// This instruction is useful for reducing power consumption and improving
/// performance in certain scenarios.
#[inline]
pub fn pause() {
    // SAFETY: The `pause` instruction introduces non-safety-altering behavior.
    unsafe {
        arch::asm!(
            "pause",
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

/// The higher-half of the timestamp counter deadline.
///
/// This is to be located in the `eax` register.
pub type TscDeadlineEax<'a> = Field<'a, 0, 31, u64>;

/// The lower-half of the timestamp counter deadline.
///
/// This is to be located in the `edx` register.
pub type TscDeadlineEdx<'a> = Field<'a, 32, 63, u64>;

/// The mutable alternative to [`TscDeadlineEax`].
pub type TscDeadlineEaxMut<'a> = <TscDeadlineEax<'a> as Counterpart>::Mut;

/// The mutable alternative to [`TscDeadlineEdx`].
pub type TscDeadlineEdxMut<'a> = <TscDeadlineEdx<'a> as Counterpart>::Mut;

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
/// This requires the [`Waitpkg`] extension to be present.
///
/// [`Waitpkg`]: nekor_aal_feature::arch::x86::qualified::Waitpkg
#[inline]
pub unsafe fn tpause(wait_state: Cstate, ref target_deadline: u64) {
    type TpauseCState<'a> = BitMut<'a, u32, 0>;

    type TpauseCStateReserved<'a> = FieldMut<'a, 1, 31, u32>;

    let edx = TscDeadlineEdx::wrap(target_deadline).value();
    let eax = TscDeadlineEax::wrap(target_deadline).value();

    let mut ecx = u32::MIN;

    TpauseCState::wrap(&mut ecx).set(if let Cstate::C0_1 = wait_state {
        State::Set
    } else {
        State::Cleared
    });

    // NOTE: We always zero out any bits marked as reserved.
    TpauseCStateReserved::wrap(&mut ecx).merge(u32::MIN);

    // SAFETY: The `tpause` instruction is available and introduces
    // non-safety-altering behavior.
    unsafe {
        arch::asm!(
            "tpause",
            in("edx") edx,
            in("eax") eax,
            in("ecx") ecx,
            options(nomem, nostack, att_syntax));
    }
}

/// An AMD-specific `hardware monitor` instruction comparable to the [`monitor`]
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
/// - The condition (in AMD CPUID leaf-as-function notation) `CPUID
///   Fn8000_0001_ECX[MONITORX](bit
///   29) = 1` to be true.
/// - The specified address must be readable in the active processor context.
///   Note that this is not the same as Rust-specific guarantees, the processor
///   simply performs access control, but does not fetch or write to the memory
///   in any way.
#[inline]
pub unsafe fn monitorx<T>(target_address: NonNull<T>) {
    /// The hint flags provided to the `monitorx` instruction.
    ///
    /// There are no existing hints for this instruction.
    const NO_MONITORX_HINTS: u32 = u32::MIN;

    /// The extensions provided to the `monitorx` instruction.
    ///
    /// There are no existing extensions for this instruction.
    const NO_MONITORX_EXTENSIONS: u32 = u32::MIN;

    // SAFETY: The monitor
    unsafe {
        arch::asm!(
            // NOTE(asm): Seems that no assembler in our toolchain supports the AMD-specific `monitorx` extension.
            "monitorx",
            // NOTE: AMD indicates that there are no existing extensions or hints for this instruction.
            //
            // For further information, see the documentation for the `monitorx` instruction on the AMD Developer Manual.
            in("edx") NO_MONITORX_HINTS,
            in("ecx") NO_MONITORX_EXTENSIONS,
            in("rax") target_address.as_ptr(),
            options(nostack)
        );
    }
}

/// Reads the current value of the time-stamp counter.
///
/// # Safety
///
/// This is a partially-privileged instruction, therefore, the caller must
/// satisfy either of:
///
/// - The *Time Stamp Disable* (TSD) bit in the `cr4` control register is
///   cleared.
/// - ..or the current *Current Privilege Level* is `Ring0`.
#[inline]
#[must_use]
pub unsafe fn rdtsc() -> u64 {
    let (edx, eax): (u32, u32);

    // SAFETY:
    //
    // The `rdtsc` instruction introduces non-safety-altering behavior, and the
    // caller guarantees that the instruction will not cause a fault due to
    // missing privileges or a disabled TSC.
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

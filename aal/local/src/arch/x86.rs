//! Per-CPU infrastructure for the `x86` architecture.
//!
//! There is major diverging behavior in this module.
//!
//! Particularly, this is due to the required support of running the kernel at a
//! higher *Current Privilege Level* (*CPL*). than `0`.
//!
//! On regular `x86{,_64}` kernel-space builds, either the `gs` or `fs` segment
//! registers are used. In 32-bit mode, `fs` is set through the *General
//! Descriptor Table* (*GDT*). In 64-bit mode, `gs` is populated through the
//! `IA32_GS_BASE` *Model Specific Register*.
//!
//! In usermode builds, the native thread local mechanism is used instead.

use crate::area::Area;

#[cfg(not(usermode))]
use core::arch;

#[cfg(usermode)]
use core::cell::Cell;

#[cfg(usermode)]
thread_local!(static CPU_AREA: Cell<Area> = Cell::new(Area::local()));

// TODO: Move to rdgsbase

/// The entry point to accessing per-CPU data.
///
/// This is simply a getter for [`Area`], the mandatory per-CPU structure.
#[inline]
#[must_use]
pub fn read_area() -> Area {
    #[cfg(usermode)]
    return CPU_AREA.get();

    /// Read from the specific segment register who has been designated for
    /// per-CPU area.
    ///
    /// # Safety
    ///
    /// The per-CPU area for this processor core must have been initialized.
    #[cfg(not(usermode))]
    #[inline(always)]
    unsafe fn read_segment_area() -> usize {
        let s: usize;

        // SAFETY: Reads from designated CPU area.
        #[cfg(target_arch = "x86_64")]
        unsafe {
            arch::asm!(
                "movq %gs:0, {0:r}",
                out(reg) s,
                options(nostack, nomem, preserves_flags, att_syntax)
            );
        }

        // SAFETY: Reads from designated CPU area.
        #[cfg(target_arch = "x86")]
        unsafe {
            arch::asm!(
                "movl %fs:0, {}",
                out(reg) s,
                options(nostack, nomem, preserves_flags, att_syntax)
            );
        }

        s
    }

    #[cfg(not(usermode))]
    return Area::raw(
        // SAFETY: Uninitialized cores never reach the Rust boundary.
        // Therefore, this is safe.
        unsafe { read_segment_area() },
    );
}

/// The entry point to accessing per-CPU data.
///
/// This is simply a setter for [`Area`], the mandatory per-CPU structure.
#[inline]
pub fn write_area(area: Area) {
    #[cfg(usermode)]
    return CPU_AREA.set(area);

    /// Writes to the specific segment register who has been designated for
    /// per-CPU area.
    ///
    /// # Safety
    ///
    /// The per-CPU area for this processor core must have been initialized.
    #[cfg(not(usermode))]
    #[inline(always)]
    unsafe fn write_segment_area(v: usize) {
        // SAFETY: Writes to designated CPU area.
        #[cfg(target_arch = "x86_64")]
        unsafe {
            arch::asm!(
                "movq {0:r}, %gs:0",
                in(reg) v,
                options(nostack, nomem, preserves_flags, att_syntax)
            );
        }

        // SAFETY: Reads from designated CPU area.
        #[cfg(target_arch = "x86")]
        unsafe {
            arch::asm!(
                "movl {}, %fs:0",
                in(reg) v,
                options(nostack, nomem, preserves_flags, att_syntax)
            );
        }
    }

    #[cfg(not(usermode))]
    // SAFETY: Uninitialized cores never reach the Rust boundary. Therefore,
    // this is safe.
    return unsafe { write_segment_area(Area::value(area)) };
}

//! Segment-relative access to the local area, or host TLS under `cfg(usermode)`.
//!
//! The kernel assembly accesses a word at segment offset zero, using `GS` in long mode and
//! `FS` in the 32-bit instruction path. It does not read or write the segment base register.
//! The platform must install a base addressing a live local word before ordinary Rust entry.

#[cfg(not(usermode))]
use core::arch;
#[cfg(usermode)]
use core::cell::Cell;

use crate::area::Area;

#[cfg(usermode)]
thread_local!(static CPU_AREA: Cell<Area> = Cell::new(Area::local()));

// TODO: Move to rdgsbase

/// Read the local word through the configured segment or host thread-local cell.
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
    #[inline]
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

/// Write the local word through the configured segment or host thread-local cell.
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
    #[inline]
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

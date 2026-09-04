//! The `x86` architecture.

use core::{
    arch,
    sync::atomic::{Ordering, compiler_fence},
};

/// Disable all maskable interrupts through the `cli` instruction.
///
/// # Safety
///
/// The caller must have *I/O Privileges* as per their current `CPL`.
#[inline]
pub unsafe fn cli() {
    // NOTE(ordering): Do not allow reordering across a critical section
    // boundary.
    compiler_fence(Ordering::SeqCst);

    // SAFETY: IO privileges guaranteed by caller.
    unsafe {
        arch::asm!(
            "cli",
            // NOTE(ordering): Not `nomem` to guarantee that the surrounding
            // fences are honored.
            options(nostack, nomem, att_syntax)
        );
    }

    compiler_fence(Ordering::SeqCst);
}

/// Enable maskable interrupts through the `sti` instruction.
///
/// # Safety
///
/// The caller must have *I/O Privileges* as per their current `CPL`.
#[inline]
pub unsafe fn sti() {
    // NOTE(ordering): Do not allow reordering across a critical section
    // boundary.
    compiler_fence(Ordering::SeqCst);

    // SAFETY: IO privileges guaranteed by caller.
    unsafe {
        arch::asm!(
            "sti",
            // NOTE(ordering): Not `nomem` to guarantee that the surrounding
            // fences are honored.
            options(nostack, nomem, att_syntax)
        );
    }

    compiler_fence(Ordering::SeqCst);
}

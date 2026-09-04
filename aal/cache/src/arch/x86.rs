//! Cache-related intrinsics for the `x86` architecture.

use core::{arch, ptr::NonNull};

use crate::line::Cacheline;

/// Evict the target [`Cacheline`] through the `clflush` instruction.
///
/// # Safety
///
/// - The provided pointer must be canonical as per the current `N`-level paging infrastructure.
/// - The provided pointer must be well-aligned.
/// - The feature bit `CPUID.01H:EDX.CLFSH[bit 19]` must be set.
#[inline]
pub unsafe fn clflush(target_cacheline: NonNull<Cacheline>) {
    // SAFETY: The caller guarantees the pointer and instruction feature are valid.
    unsafe {
        arch::asm!(
            "clflush [{0:r}]",
            in(reg) target_cacheline.as_ptr(),
            options(nostack, preserves_flags, att_syntax)
        );
    }
}

/// Evict the target [`Cacheline`] through the `clflushopt` instruction.
///
/// This is a non-serializing alternative to the [`clflush`] instruction.
///
/// # Safety
///
/// - The provided pointer must be canonical as per the current `N`-level paging infrastructure.
/// - The provided pointer must be well-aligned.
/// - The feature bit `CPUID.(EAX=07H,ECX=0H):EBX[bit 23]` must be set.
#[inline]
pub unsafe fn clflushopt(target_cacheline: NonNull<Cacheline>) {
    // SAFETY: The caller guarantees the pointer and instruction feature are valid.
    unsafe {
        arch::asm!(
            "clflushopt [{0:r}]",
            in(reg) target_cacheline.as_ptr(),
            options(nostack, preserves_flags, att_syntax)
        );
    }
}

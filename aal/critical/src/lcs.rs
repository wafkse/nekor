//! Maskable-interrupt exclusion on the current processor.
//!
//! Very little delay, used for processor initialization and for contexts that
//! are non preemptable.
//!
//! Note that [`Lcs`] cannot inhibit *Non Maskable Interrupts* (*NMI*s) on
//! architectures such as `x86`.
//!
//! In usermode emulation mode, no interrupt masking is done, as kernel
//! preemption is opaque and guarantees are core-local.

use core::marker;

#[cfg(not(usermode))]
use crate::arch;

/// A core-*Local Critical Section*.
///
/// # Notes
///
/// This masks ordinary x86 interrupts outside usermode builds. It does not save the prior
/// interrupt state, so nesting is forbidden and release always enables interrupts.
pub enum Lcs {}

// FIXME: Lcs needs to determine whether interrupts were already disabled. Do
// something alike to preempt_count in Linux.

impl Lcs {
    /// Acquire a core-*Local Critical Section*.
    ///
    /// # Safety
    ///
    /// - A call to this function must be matched with a posterior [`Lcs::release`] call.
    /// - The output [`LcsToken`] must not outlive the actual *LCS*.
    /// - The calling core must not be in the following Critical Section types:
    ///     - [`Lcs`]: *Local Critical Section*, i.e., it is non-reentrant.
    ///     - [`crate::gcs::Gcs`]: *Global Critical Section*
    ///     - [`crate::mscs::MsCs`]: *Machine-Stop Critical Section*
    ///
    /// - The code region the [`LcsToken`] is linked with must be surrounded by a [`SeqCst`]
    ///   [`compiler fence`].
    ///
    /// - In `x86`, the calling core must have *I/O Privileges* as per its current `CPL`, unless the
    ///   `usermode` configuration is enabled.
    ///
    /// [`SeqCst`]: core::sync::atomic::Ordering::SeqCst
    /// [`compiler fence`]: core::sync::atomic::compiler_fence
    #[inline]
    #[must_use]
    pub unsafe fn acquire() -> LcsToken {
        #[cfg(not(usermode))]
        // SAFETY: Caller asserts required IO privileges.
        unsafe {
            #[cfg(target_arch = "x86")]
            arch::x86::cli();

            #[cfg(target_arch = "x86_64")]
            arch::x86_64::cli();
        }

        // NOTE: On usermode `x86`, we simply make this a no-op, as we will be
        // preempted by the kernel transparently.

        // SAFETY: The core has just acquired a critical section.
        unsafe { LcsToken::raw() }
    }

    /// Release a core-*Local Critical Section*.
    ///
    /// # Safety
    ///
    /// Consume the token from the matching earlier [`Lcs::acquire`] call on the same
    /// processor. No nested local, global or machine-stop critical section may depend
    /// on interrupts remaining disabled, because release enables them unconditionally.
    ///
    /// Outside usermode builds, the current x86 privilege level must permit `sti`.
    /// The compiler fences required by acquisition must also surround the protected code.
    pub unsafe fn release(_: LcsToken) {
        #[cfg(not(usermode))]
        // SAFETY: Caller asserts required IO privileges.
        unsafe {
            #[cfg(target_arch = "x86")]
            arch::x86::sti();

            #[cfg(target_arch = "x86_64")]
            arch::x86_64::sti();
        }
    }

    /// Enter a core-*Local Critical Section* that is only active during the
    /// call frame of a target closure.
    ///
    /// # Panics
    ///
    /// If the closure unwinds, release is not called because this function has no drop guard.
    /// Maskable interrupts can therefore remain disabled. Use a result for expected failure
    /// and arrange any required unwind cleanup outside this helper.
    ///
    /// # Safety
    ///
    /// - The calling core must not be in the following Critical Section types:
    ///     - [`Lcs`]: *Local Critical Section*, i.e., it is non-reentrant.
    ///     - [`crate::gcs::Gcs`]: *Global Critical Section*
    ///     - [`crate::mscs::MsCs`]: *Machine-Stop Critical Section*
    ///
    /// - In `x86`, the calling core must have *I/O Privileges* as per its current `CPL`, unless the
    ///   `usermode` configuration is enabled.
    #[inline]
    pub unsafe fn closure<F, O>(target_closure: F) -> O
    where
        F: FnOnce(&LcsToken) -> O,
    {
        // SAFETY: Constraints satified by caller.
        let lcs_token = unsafe { Self::acquire() };

        let result = target_closure(&lcs_token);

        // SAFETY: Constraints satified by caller.
        unsafe {
            Self::release(lcs_token);
        }

        result
    }
}

/// Authority to access operations restricted to a local critical section.
///
/// The owned token is consumed by [`Lcs::release`]. A borrowed token lets an operation use
/// that authority while leaving release to its owner. Construction is unsafe, and callers
/// must keep the token within the critical section because it carries no section lifetime.
#[derive(Debug)]
// NOTE(invariant): Only a caller in the local critical section may construct this authority token.
pub struct LcsToken(
    // The raw-pointer marker keeps critical-section authority local to its execution context.
    marker::PhantomData<*mut Self>,
);

impl LcsToken {
    /// Construct a [`LcsToken`] out of thin air.
    ///
    /// # Safety
    ///
    /// The caller must be held in a [`Lcs`], and the lifetime of the
    /// [`LcsToken`] must not outlive such critical section.
    #[inline]
    #[must_use]
    pub const unsafe fn raw() -> Self {
        Self(marker::PhantomData)
    }
}

//! - *Local Critical Sections* (*LCS*) are those that are native to the
//!   currently-active core, and thus correspond to *disabling interrupts* for
//!   the current core.
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
/// This will inhibit all preemption for the currently-executing core.
pub enum Lcs {}

// FIXME: Lcs needs to determine whether interrupts were already disabled. Do
// something alike to preempt_count in Linux.

impl Lcs {
    /// Acquire a core-*Local Critical Section*.
    ///
    /// # Safety
    ///
    /// - A call to this function must be matched with a posterior
    ///   [`Lcs::release`] call.
    /// - The output [`LcsToken`] must not outlive the actual *LCS*.
    /// - The calling core must not be in the following Critical Section types:
    ///     - [`Lcs`]: *Local Critical Section*, i.e., it is non-reentrant.
    ///     - [`Gcs`]: *Global Critical Section*
    ///     - [`MsCs`]: *Machine-Stop Critical Section*
    ///
    /// - The code region the [`LcsToken`] is linked with must be surrounded by
    ///   a [`SeqCst`] [`compiler fence`].
    ///
    /// - In `x86`, the calling core must^[1] have *I/O Privileges* as per their
    ///   current `CPL`.
    ///
    /// [1]: This is not required when the `usermode` crate feature is enabled.
    ///
    /// [`MsCs`]: crate::mscs::MsCs
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
    /// - A call to this function must be matched with an ulterior
    ///   [`Lcs::acquire`] call.
    /// - The output [`LcsToken`] must not outlive the actual *LCS*.
    /// - The calling core must not be in the following Critical Section types:
    ///     - [`Lcs`]: *Local Critical Section*, i.e., it is non-reentrant.
    ///     - [`Gcs`]: *Global Critical Section*
    ///     - [`MsCs`]: *Machine-Stop Critical Section*
    ///
    /// - In `x86`, the calling core must<sup>[1]</sup> have *I/O Privileges* as
    ///   per their current `CPL`.
    ///
    /// [1]: This is not required when the `usermode` crate feature is enabled.
    ///
    /// [`MsCs`]: crate::mscs::MsCs
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
    /// The provided closure should attempt to not panic, and should instead
    /// rely on a [`Result`].
    ///
    /// # Safety
    ///
    /// - The calling core must not be in the following Critical Section types:
    ///     - [`Lcs`]: *Local Critical Section*, i.e., it is non-reentrant.
    ///     - [`Gcs`]: *Global Critical Section*
    ///     - [`MsCs`]: *Machine-Stop Critical Section*
    ///
    /// - In `x86`, the calling core must<sup>1</sup> have *I/O Privileges* as
    ///   per their current `CPL`.
    ///
    /// `1`: This is not required when the `usermode` crate feature is enabled.
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

/// A private-constructible token that can *prove* whether a caller is currently
/// inside a [`Lcs`].
///
/// This token serves two purposes based on ownership:
///
/// ## Owned Token
///
/// When you own an `LcsToken`, you control the critical section's lifetime:
///
/// ```ignore
/// let token = unsafe { Lcs::acquire() };  // CS begins
/// // ... do work ...
/// unsafe { Lcs::release(token) };         // CS ends, token consumed
/// ```
///
/// ## Borrowed Token
///
/// When you receive `&LcsToken`, you have proof that a CS exists but cannot
/// terminate it:
///
/// ```ignore
/// fn critical_operation(cs: &LcsToken) {
///     // Can prove we're in a CS
///     // Cannot terminate it (no owned token)
///     // Cannot store it (borrow checker prevents escaping)
/// }
/// ```
#[derive(Debug)]
pub struct LcsToken(
    // NOTE(invariant): Do not implement `Send` or `Sync` for this type.
    marker::PhantomData<fn() -> *mut Self>,
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

//! Stable-point critical sections stop all processor cores at safe locations.
//!
//! - *Stable-point Critical Sections* (*`SpCS`*):
//!     - Puts the currently-active core in a waiting state and publishes stablepoint deferral to
//!       all other cores through an *IPI*.
//!     - Once all cores are in a well-defined stablepoint, they are all put to sleep.
//!
//! This mechanism has a very large delay. It is used for runtime code patching.
//!
//! Well, but what is a *stablepoint*?
//!
//! A stablepoint is a small region in kernel code where a core can be put
//! offline without hazard regarding unfulfilled work. In other words, it's a
//! point in the kernel loop lifecycle where a core is guaranteed to be in a
//! safe quiescent state.
//!
//! ## Yes, but how does it work?
//!
//! At each stablepoint region, [`Stablepoint::opportunity`] is called to allow
//! machine-wide synchronization to happen.
//!
//! When *`SpCS`* is inactive, this call is a no-op. A single `nop` instruction
//! is executed and the core continues normally.
//!
//! When *`SpCS`* is active, the stablepoint entry is patched to raise a CPU
//! exception (e.g., `int3` on `x86`). The exception handler checks the
//! instruction pointer, identifies it as a stablepoint, and suspends the core.
//!
//! Once all cores reach their stablepoints and suspend, the machine is fully
//! stopped. Machine-wide mutation can proceed safely.
//!
//! ## Where should stablepoints be placed?
//!
//! Stablepoints should be frequent enough to avoid long delays, but only at
//! safe locations:
//!
//! - Scheduler loop start.
//! - Idle loop iterations
//! - Between independent kernel operations
//!
//! Stablepoints must never be placed where:
//!
//! - Locks are held.
//! - Data structures are in an inconsistent state.
//! - Critical operations are incomplete.

/// An uninhabited type representing a stablepoint.
pub enum Stablepoint {}

impl Stablepoint {
    /// An opportunity to engage in system-wide stablepoint synchronization.
    ///
    /// # Behavior
    ///
    /// When *`SpCS`* is inactive, this is a single `nop` instruction.
    ///
    /// When *`SpCS`* is active, this traps via CPU exception and suspends the
    /// core until the machine is released.
    ///
    /// # Safety
    ///
    /// The caller must ensure this is called only at a valid stablepoint:
    ///
    /// - No locks may be held
    /// - No partial operations may be in progress
    /// - The core must be in a consistent, quiescent state
    pub const unsafe fn opportunity() {}
}

//! Raw interpreter threads for controlled tests.

use crate::ffi;

/// An interpreter thread ID awaiting an explicit join.
#[must_use = "join the interpreter thread before test exit"]
// NOTE(invariant): The stored ID comes from `miri_thread_spawn`, not arbitrary caller input.
pub struct Thread(usize);

impl Thread {
    /// Spawn a thread with raw callback data.
    ///
    /// # Safety
    ///
    /// `data` must remain valid until the callback returns, may cross threads, and all
    /// access to it must be correctly synchronized. The callback must not unwind.
    #[inline]
    pub unsafe fn spawn(callback: extern "Rust" fn(*mut ()), data: *mut ()) -> Self {
        // SAFETY: The caller owns the data lifetime and synchronization contract.
        Self(unsafe { ffi::miri_thread_spawn(callback, data) })
    }

    /// Wait for this thread and report whether Miri accepted its ID.
    #[inline]
    pub fn join(self) -> bool {
        let Self(id) = self;

        // SAFETY: The ID came from the interpreter spawn shim.
        unsafe { ffi::miri_thread_join(id) }
    }
}

/// Yield to another interpreter thread during a busy wait.
#[inline]
pub fn spin() {
    // SAFETY: The shim has no caller-side preconditions.
    unsafe { ffi::miri_spin_loop() };
}

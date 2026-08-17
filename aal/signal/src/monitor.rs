//! # Value Monitoring
//!
//! The [`Monitor`] abstraction provides a lightweight mechanism for
//! observing mutable accesses to a shared memory location. There are **two
//! primary monitoring strategies**:
//!
//! ## 1. Side-Channel Based Monitoring
//!
//! Side-channel monitors rely on **hardware-assisted memory observation**.
//! A thread can efficiently suspend execution until a write is detected at
//! a monitored memory location. Wakeups may be **spurious**, so the
//! monitored value must always be rechecked.
//!
//! ## 2. Wake-Up Based Event Monitoring
//!
//! Wake-up monitors use **explicit signaling** to notify waiting threads.
//! Threads suspend until another thread or core sends a wakeup event. This
//! approach guarantees that waiting threads are notified, and is typically
//! used for inter-core coordination.
//!
//! ## Platform Examples
//!
//! | Monitoring Type            | Example Instructions | Notes                              |
//! |----------------------------|-------------------|------------------------------------|
//! | Side-Channel Based          | `monitor` / `mwait` | Efficient, best-effort wakeups     |
//! | Wake-Up Based Event         | `wfe` / `sev`       | Explicit wakeup between cores      |
//!
//! ## General Notes
//!
//! * [`Monitor::engage`] starts monitoring the memory location.
//! * [`Monitored::wait`] suspends the thread until a possible wake event
//!   occurs.
//! * All monitor operations are **best-effort**; the monitored value must be
//!   rechecked after wakeup.
//! * Side-channel monitoring is highly efficient, while wake-up event
//!   monitoring provides explicit guarantees across threads or cores.
//! * Access guards must be used for *Wake-Up Based Event Monitoring* to
//!   function.
//!
//! [`Monitor`]: crate::monitor::Monitor
//! [`Monitored::wait`]: crate::monitor::Monitored::wait

use core::{
    marker, mem,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

#[cfg(not(any(usermode, test, miri)))]
use crate::arch;

/// A new-type that monitors memory accesses to the underlying value.
///
/// # Functionality
///
/// This allows an accessor to *selectively wake up* a waiting partner for the
/// same value.
///
/// By "selective", this implies that the underlying `T` can be freely mutated
/// without a corresponding wake-up event being emitted in an implicit manner.
///
/// For additional information, see the [`module-level documentation`].
///
/// [`module-level documentation`]: self
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
#[repr(transparent)]
pub struct Monitor<T>(
    // NOTE: Architectures relying on side-channel monitoring benefit from the
    // underlying value being cacheline-isolated.
    T,
);

impl<T> Monitor<T> {
    /// Construct a new [`Monitor`] new-type.
    #[inline]
    pub const fn new(target_value: T) -> Self {
        Self(target_value)
    }

    /// Access the underlying value through a dedicated RAII guard.
    #[inline]
    pub const fn access(&self) -> MonitorGuard<'_, T> {
        let Self(target_value) = self;

        MonitorGuard(target_value)
    }

    /// Mutably access the underlying value through a dedicated RAII guard.
    #[inline]
    pub const fn access_mut(&mut self) -> MonitorGuardMut<'_, T> {
        let &mut Self(ref mut target_value) = self;

        MonitorGuardMut(target_value)
    }

    /// Engage the monitor on the value.
    #[inline]
    pub fn engage(&self) -> Monitored<'_, T> {
        let target_pointer = NonNull::from_ref(self);

        #[cfg(not(any(usermode, test, miri)))]
        #[cfg(target_arch = "x86")]
        unsafe {
            arch::x86::monitor(target_pointer);
        };

        #[cfg(not(any(usermode, test, miri)))]
        #[cfg(target_arch = "x86_64")]
        unsafe {
            arch::x86_64::monitor(target_pointer);
        };

        Monitored(target_pointer, marker::PhantomData)
    }
}

impl<T> Deref for Monitor<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        let Self(target_value) = self;

        target_value
    }
}

impl<T> DerefMut for Monitor<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        let &mut Self(ref mut target_value) = self;

        target_value
    }
}

/// A currently-monitored memory address through architecture-specific means.
///
/// # Remarks
///
/// Note that the monitor behavior is best-effort, and could spuriously trigger.
///
/// Always check the underlying condition after a [`Monitored::wait`].
#[repr(transparent)]
pub struct Monitored<'a, T>(NonNull<Monitor<T>>, marker::PhantomData<&'a T>);

impl<T> Monitored<'_, T> {
    /// Wait for a write to happen at the monitored location.
    ///
    /// # Remark
    ///
    /// This is done on a best-effort basis, if no monitor hardware capability
    /// is present, this is effectively an entry to an
    /// architecture-dependent optimized state.
    #[inline]
    pub fn wait(self) {
        #[cfg(not(any(usermode, test, miri)))]
        #[cfg(target_arch = "x86")]
        unsafe {
            arch::x86::mwait();
        };

        #[cfg(not(any(usermode, test, miri)))]
        #[cfg(target_arch = "x86_64")]
        unsafe {
            arch::x86_64::mwait();
        };

        // TODO: Add umonitor and umwait support, but that requires runtime
        // instruction specialization.
        //
        // Also add transient Relax::now, direct copy from Linux's cpu_relax
    }
}

/// The load side of a monitored value.
///
/// In architectures with well-defined interprocessor signaling, this will wake
/// any waiting side.
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct MonitorGuard<'a, T>(&'a T);

impl<T> MonitorGuard<'_, T> {
    /// Disallow use of `Monitor` on trivially-copyable types.
    #[doc(hidden)]
    #[allow(unused)]
    const ASSERT_NEEDS_DROP: () = assert!(mem::needs_drop::<Self>());

    /// Access a reference to the monitor-guarded type.
    #[inline]
    pub const fn as_ref(&self) -> &'_ T {
        let Self(target_value) = self;

        target_value
    }
}

impl<T> Deref for MonitorGuard<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        let &Self(target_value) = self;

        target_value
    }
}

impl<T> Drop for MonitorGuard<'_, T> {
    #[inline]
    fn drop(&mut self) {
        // TODO: This `drop` is for signal-based monitoring only.
    }
}

/// The store side of a monitored value.
#[repr(transparent)]
pub struct MonitorGuardMut<'a, T>(&'a mut T);

impl<T> Deref for MonitorGuardMut<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        let Self(target_value) = self;

        target_value
    }
}

impl<T> DerefMut for MonitorGuardMut<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        let &mut Self(ref mut target_value) = self;

        target_value
    }
}

impl<T> Drop for MonitorGuardMut<'_, T> {
    #[inline]
    fn drop(&mut self) {
        // TODO: This `drop` is for signal-based monitoring only.
    }
}

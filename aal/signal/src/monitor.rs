//! Value monitoring.
//!
//! [`Monitor::engage`] arms observation of a shared value. [`Monitored::wait`] waits for a
//! possible wake event or performs a backoff cycle when no hardware monitor is selected.
//!
//! *Side-channel monitoring*, such as `monitor`/`mwait`, observes memory writes.
//! *Event-based monitoring*, such as `wfe`/`sev`, requires explicit signals through access guards.
//!
//! # Remarks
//!
//! Monitoring is best-effort. Recheck the condition after engagement and after waking.
//! [`Polling`] combines bounded optimistic retries with monitored waiting.

use core::{
    hint, marker,
    ops::{Deref, DerefMut},
};

use nekor_backoff::backoff::{Backoff, BackoffState};

/// A value with selective monitor engagement and guarded access.
///
/// # Remarks
///
/// Direct mutation does not implicitly emit an explicit wakeup signal.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
#[repr(transparent)]
// NOTE(invariant): None.
pub struct Monitor<T>(
    // NOTE: Cache-line isolation benefits side-channel monitoring.
    T,
);

impl<T> Monitor<T> {
    /// Wrap a value in a [`Monitor`].
    #[inline]
    pub const fn new(target_value: T) -> Self {
        Self(target_value)
    }

    /// Borrow the value through a [`MonitorGuard`].
    #[inline]
    pub const fn access(&self) -> MonitorGuard<'_, T> {
        let Self(ref target_value) = *self;

        MonitorGuard(target_value)
    }

    /// Borrow the value exclusively through a [`MonitorGuardMut`].
    #[inline]
    pub const fn access_mut(&mut self) -> MonitorGuardMut<'_, T> {
        let &mut Self(ref mut target_value) = self;

        MonitorGuardMut(target_value)
    }

    /// Arm monitoring and return a borrowed [`Monitored`] token.
    #[inline]
    pub fn engage(&self) -> Monitored<'_, T> {
        cfg_select! {
            all(any(target_arch = "x86", target_arch = "x86_64"), not(miri)) => {
                use core::ptr::NonNull;

                use nekor_aal_hotpatch::prelude::Patch;

                use crate::arch::x86::monitor::{Dispatch, Operation};

                Patch::<Dispatch, Dispatch>::run(Operation::Arm(NonNull::from_ref(self).cast()));

                Monitored(marker::PhantomData)
            },
            _ => Monitored(marker::PhantomData),
        }
    }
}

impl<T> Deref for Monitor<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        let Self(ref target_value) = *self;

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

/// A monitor engagement that keeps the value borrowed until waiting.
///
/// # Remarks
///
/// Wakeups may be spurious. Recheck the condition after [`Monitored::wait`].
// NOTE(invariant): None.
pub struct Monitored<'a, T>(marker::PhantomData<&'a Monitor<T>>);

impl<T> Monitored<'_, T> {
    /// Wait for a possible monitored write or wake event.
    ///
    /// # Remarks
    ///
    /// [`BackoffState`] advances once, even if the selected monitor ignores the delay.
    /// Retain it across waits to increase timed delays. Without hardware monitoring,
    /// backoff returns after the requested delay for a condition recheck.
    #[inline]
    pub fn wait(self, backoff_state: &mut BackoffState) {
        cfg_select! {
            all(any(target_arch = "x86", target_arch = "x86_64"), not(miri)) => {
                use nekor_aal_hotpatch::prelude::Patch;
                use nekor_backoff::arch::x86::Delay;

                use crate::arch::x86::monitor::{Dispatch, Operation};

                let target_delay = backoff_state.advance().map_or(Delay::Hint, Delay::Timed);

                Patch::<Dispatch, Dispatch>::run(Operation::Wait(target_delay));
            },
            _ => Backoff::cycle(backoff_state),
        };
    }
}

/// The outcome of an operation on a monitored value.
///
/// [`Attempt::Pending`] must permit a retry without invalid partial mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attempt<R> {
    /// The operation completed.
    Ready(R),

    /// The operation needs a retry.
    Pending,
}

/// An optimistic retry budget before monitored waiting.
///
/// The immediate attempt and post-engagement recheck do not consume the budget.
/// Workloads may define their own [`Tier`].
pub trait Tier {
    /// Optimistic retries after the immediate attempt.
    const RETRIES: usize;
}

/// Engage the monitor after the immediate attempt fails.
pub struct Immediate;

impl Tier for Immediate {
    const RETRIES: usize = 0;
}

/// Poll briefly before engaging the monitor.
pub struct Cruise;

impl Tier for Cruise {
    const RETRIES: usize = 4;
}

/// Poll longer for operations expected to complete quickly.
pub struct Fastlane;

impl Tier for Fastlane {
    const RETRIES: usize = 8;
}

/// Bounded polling followed by monitored waiting.
///
/// Wakeups do not restore the retry budget. Backoff progresses across waits.
// NOTE(invariant): `remaining` only decreases and never exceeds `P::RETRIES`.
pub struct Polling<'a, T, P: Tier> {
    /// The borrowed monitor.
    monitor: &'a Monitor<T>,

    /// Optimistic retries remaining.
    remaining: usize,

    /// Backoff progression across waits.
    backoff_state: BackoffState,

    /// The compile-time retry tier.
    tier: marker::PhantomData<P>,
}

impl<'a, T, P: Tier> Polling<'a, T, P> {
    /// Borrow a monitor with [`Backoff::minimal`] for timed waits.
    #[inline]
    pub const fn of(monitor: &'a Monitor<T>) -> Self {
        Self::with_backoff(monitor, Backoff::minimal())
    }

    /// Borrow a monitor with the selected [`Backoff`] strategy.
    #[inline]
    pub const fn with_backoff(monitor: &'a Monitor<T>, backoff: Backoff) -> Self {
        Self {
            monitor,
            remaining: P::RETRIES,
            backoff_state: backoff.state(),
            tier: marker::PhantomData,
        }
    }

    /// Retry an operation until it completes.
    ///
    /// The attempt may inspect or atomically modify the value. [`Attempt::Pending`]
    /// must permit a retry. Each passive wait follows engagement and a condition recheck.
    #[inline]
    pub fn until<R>(&mut self, mut target_closure: impl FnMut(&T) -> Attempt<R>) -> R {
        let &mut Self {
            monitor,
            ref mut remaining,
            ref mut backoff_state,
            ..
        } = self;

        loop {
            match target_closure(monitor) {
                Attempt::Ready(target_outcome) => break target_outcome,
                Attempt::Pending => (),
            }

            if *remaining != 0 {
                *remaining -= 1;
                hint::spin_loop();
                continue;
            }

            let engaged = monitor.engage();

            match target_closure(monitor) {
                Attempt::Ready(result) => break result,
                Attempt::Pending => engaged.wait(backoff_state),
            }
        }
    }
}

/// Shared guarded access to a monitored value.
///
/// Explicit wakeup strategies require guarded access.
#[derive(Debug, Clone)]
#[repr(transparent)]
// NOTE(invariant): None.
pub struct MonitorGuard<'a, T>(&'a T);

impl<T> MonitorGuard<'_, T> {
    /// Borrow the guarded value.
    #[inline]
    #[must_use]
    pub const fn as_ref(&self) -> &'_ T {
        let Self(target_value) = *self;

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

/// Exclusive guarded access to a monitored value.
#[repr(transparent)]
// NOTE(invariant): None.
pub struct MonitorGuardMut<'a, T>(&'a mut T);

impl<T> Deref for MonitorGuardMut<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        let Self(ref target_value) = *self;

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

#[cfg(test)]
mod tests {
    extern crate alloc;

    use alloc::sync::Arc;
    use core::{
        num::NonZero,
        sync::atomic::{AtomicUsize, Ordering},
    };
    use std::{sync::Barrier, thread};

    use nekor_backoff::backoff::Backoff;

    use super::{Attempt, Cruise, Immediate, Monitor, Polling, Tier};

    #[test]
    fn recheck_after_engagement_completes_without_waiting() {
        let target = Monitor::new(AtomicUsize::new(0));
        let mut polling = Polling::<_, Immediate>::of(&target);
        let mut calls = 0;

        let result = polling.until(|_| {
            calls += 1;

            match calls {
                1 => Attempt::Pending,
                _ => Attempt::Ready(calls),
            }
        });

        assert_eq!(result, 2);
    }

    #[test]
    fn early_budget_is_consumed_once() {
        struct Burst;

        impl Tier for Burst {
            const RETRIES: usize = 3;
        }

        let target = Monitor::new(AtomicUsize::new(0));
        let mut polling = Polling::<_, Burst>::of(&target);
        let mut calls = 0;

        let result = polling.until(|_| {
            calls += 1;

            match calls {
                1..5 => Attempt::Pending,
                _ => Attempt::Ready(calls),
            }
        });

        assert_eq!(result, 5);
        assert_eq!(polling.remaining, 0);
    }

    #[test]
    fn write_between_engagement_and_recheck_is_observed() {
        let target = Arc::new(Monitor::new(AtomicUsize::new(0)));
        let ready = Arc::new(Barrier::new(2));
        let written = Arc::new(Barrier::new(2));
        let writer_target = Arc::clone(&target);
        let writer_ready = Arc::clone(&ready);
        let writer_written = Arc::clone(&written);

        let writer = thread::spawn(move || {
            writer_ready.wait();
            writer_target.store(1, Ordering::Release);
            writer_written.wait();
        });

        let mut polling = Polling::<_, Immediate>::of(&target);
        let mut calls = 0;
        let result = polling.until(|value| {
            calls += 1;

            if calls == 2 {
                ready.wait();
                written.wait();
            }

            match value.load(Ordering::Acquire) {
                0 => Attempt::Pending,
                observed => Attempt::Ready(observed),
            }
        });

        writer.join().expect("writer thread panicked");
        assert_eq!(result, 1);
        assert_eq!(calls, 2);
    }

    #[test]
    fn wakeups_do_not_restore_early_budget() {
        let target = Monitor::new(AtomicUsize::new(0));
        let mut polling = Polling::<_, Cruise>::of(&target);
        let mut calls = 0;

        let result = polling.until(|_| {
            calls += 1;

            match calls {
                1..8 => Attempt::Pending,
                _ => Attempt::Ready(calls),
            }
        });

        assert_eq!(result, 8);
        assert_eq!(polling.remaining, 0);
    }

    #[test]
    fn pending_compare_exchange_can_be_retried() {
        let target = Monitor::new(AtomicUsize::new(0));
        let mut polling = Polling::<_, Cruise>::of(&target);
        let attempts = AtomicUsize::new(0);

        let result = polling.until(|value| {
            let attempt = attempts.fetch_add(1, Ordering::Relaxed);

            if attempt == 1 {
                value.store(1, Ordering::Release);
            }

            match value.compare_exchange(1, 2, Ordering::AcqRel, Ordering::Acquire) {
                Ok(previous) => Attempt::Ready(previous),
                Err(..) => Attempt::Pending,
            }
        });

        assert_eq!(result, 1);
        assert_eq!(target.load(Ordering::Acquire), 2);
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn default_monitor_path_returns_for_recheck() {
        let target = Monitor::new(AtomicUsize::new(0));
        let mut backoff_state = Backoff::minimal().state();

        target.engage().wait(&mut backoff_state);
        target.engage().wait(&mut backoff_state);

        assert_eq!(target.load(Ordering::Relaxed), 0);
        assert_eq!(backoff_state.cycle(), NonZero::new(2));
    }
}

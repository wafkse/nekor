//! Sequence tracking through atomic integer primitives.

use core::{
    ptr,
    sync::atomic::{AtomicUsize, Ordering::SeqCst},
};

use nekor_aal_signal::{monitor::MonitorGuard, prelude::Monitor};

/// An atomic sequence-counter.
///
/// This is used for out-of-band "something happened while you were not looking"
/// notifications.
///
/// Particularly, [`sequence snapshot`] is acquired to determine whether the
/// sequence count has changed.
///
/// [`sequence snapshot`]: Sequence
#[derive(Debug)]
#[repr(transparent)]
pub struct AtomicSequence(Monitor<AtomicUsize>);

impl AtomicSequence {
    /// Constructs a brand-new atomic sequence-counter, initialized at
    /// [`usize::MIN`].
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self(Monitor::new(AtomicUsize::new(usize::MIN)))
    }

    /// Creates a snapshot of this [`AtomicSequence`].
    #[inline]
    pub fn snapshot(&self) -> Sequence<'_> {
        let &Self(ref target_state) = self;

        let target_guard = target_state.access();

        let target_snapshot = target_guard.load(SeqCst);

        Sequence(target_guard, target_snapshot)
    }
}

impl Default for AtomicSequence {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// A sequence-counter snapshot.
///
/// This is used to represent the past state where the associated
/// sequence-counter has been advanced.
///
/// # ABA and Overflow
///
/// Users of a [`Sequence`] must guarantee that they won't hold it for an
/// unbounded amount of time. Because the underlying sequence counter is bounded
/// by `usize`, an ABA problem can theoretically occur if the counter wraps
/// around entirely (i.e., exactly `usize::MAX + 1` state changes happen between
/// a thread's reads).
///
/// It is deferred to the caller to evaluate the potential for such a delay
/// given the sequence counter's ABA issue. Particularly, the ABA issue might
/// cause logical bugs (for example, losing a wakeup event), but it will never
/// cause memory unsafety.
#[derive(Debug)]
pub struct Sequence<'a>(MonitorGuard<'a, AtomicUsize>, usize);

impl Sequence<'_> {
    /// Determines whether the atomic sequence has changed in respect to the
    /// sequence.
    ///
    /// If the sequence snapshot does not belong to the provided
    /// [`AtomicSequence`] provided, this will yield `None`.
    #[inline]
    pub fn changed(self, target_sequence: &AtomicSequence) -> Option<bool> {
        let &AtomicSequence(ref left_sequence) = target_sequence;

        let Self(right_sequence, right_snapshot) = self;

        ptr::addr_eq(target_sequence, right_sequence.as_ref()).then(|| {
            let left_snapshot = left_sequence.load(SeqCst);

            left_snapshot != right_snapshot
        })
    }
}

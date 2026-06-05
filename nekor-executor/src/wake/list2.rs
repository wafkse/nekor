//! Intrusive wait queue for event-driven task coordination.
//!
//! This module implements a lock-free circular singly-linked list of
//! [`AtomicWaker`]s waiting on external events. Each waiting task embeds a
//! [`Node`] containing its atomic waker, which is linked into the list without
//! allocation.
//!
//! The queue supports constant-time (`O(1)`) lock-free insertion and
//! linear-time (`O(n)`) wake-all through an atomic detach operation, allowing
//! wake sources to atomically claim the entire wait list and traverse it
//! independently. Tasks may cancel waiting by atomically clearing their waker
//! (`O(1)`), or be physically removed during task deallocation through a
//! detach-filter-reinsert operation that rebuilds the list without the target
//! node (`O(n)`).
//!
//! [`WaitNode`]s are pinned and statically allocated as part of task storage,
//! with each task potentially embedding multiple nodes for simultaneous waits
//! on distinct event sources.

use core::{
    marker,
    mem::{ManuallyDrop, MaybeUninit},
    pin::Pin,
    ptr::{self, NonNull},
    sync::atomic::{AtomicPtr, AtomicUsize, Ordering},
    task::Waker,
};

use nekor_aal::signal::{monitor::MonitorGuard, prelude::Monitor};

use nekor_sync::atomic::sequence::AtomicSequence;

/// An union of the underlying types that a linked list node pointer can point
/// to.
///
/// This is considered an "element", for easier description of the functionality
/// of the list.
pub union Element {
    /// A wake node.
    ///
    /// This represents a non-root element.
    wake_node: ManuallyDrop<WakeNode>,

    /// A wake list.
    ///
    /// This represents a root element.
    wake_root: ManuallyDrop<WakeList>,
}

/// * Wakey, wakey! It's time!
///
/// This is a singly-linked circular list where elements are of type
/// [`AtomicWaker`].
pub struct WakeList {
    /// An atomic pointer to the head element of the linked list.
    // NOTE(invariant): `list-head` corresponds to some lifetime `'1` that outlives the wake-list
    // or is `'static`.
    list_head: Monitor<AtomicPtr<Element>>,

    /// The count of concurrent walkers through the list.
    ///
    /// This is to be used to safely sever the
    walk_count: Monitor<AtomicUsize>,

    /// The sequence-counter of this wake list.
    ///
    /// This is used for detach-filter-reinsert missed-wakeup signaling.
    ///
    /// This may coalesce multiple one or more wakeup events into a singular
    /// one.
    sequence: AtomicSequence,

    /// A marker field indicating that this type cannot be [`Unpin`]ned.
    marker: marker::PhantomPinned,
}

impl WakeList {
    /// Constructs a [`WakeList`] in-place, returning a pinned mutable reference
    /// to it.
    #[inline]
    pub fn inplace(target_storage: Pin<&mut MaybeUninit<Self>>) -> Pin<&mut Self> {
        // SAFETY: The value here is uninitialized, so no pinned guarantees are made
        // yet.
        let uninit_value = unsafe { target_storage.get_unchecked_mut() };

        // NOTE: This is used as a sentinel value, and never as an extraneous access
        // point to the list.
        let head_atomic = AtomicPtr::new(uninit_value.as_mut_ptr().cast::<Element>());

        let list_head = Monitor::new(head_atomic);

        let walk_count = Monitor::new(AtomicUsize::new(usize::MIN));

        let sequence = AtomicSequence::new();

        let marker = marker::PhantomPinned;

        let target_value = uninit_value.write(Self {
            list_head,
            walk_count,
            sequence,
            marker,
        });

        // SAFETY: The `WakeList` is provided as a valid pinned type, and will not be
        // able to be moved.
        unsafe { Pin::new_unchecked(target_value) }
    }
}

// FIXME: Require 'static lifetime for WakeList and WakeNode to prevent ABA.
// mark that in the receivers as Pin<&'static T>
//
// FIXME: Sometimes a wakeup event can be lost (example: being simultaneous with
// a detach-filter-reinsert operation, which can remove waiting AtomicWakers).
//
// We cannot EVER miss a wakeup event, so a proper way to indicate "I'm walking
// to wake up futures, so please don't hide them from me."

/// A single node in a [`WakeList`], encompassing a sole [`Waker`].
pub struct WakeNode {
    /// An atomic pointer to next element of the list.
    // NOTE(invariant): `list-next` cannot ever be self-referential in a `WakeNode` context.
    //                  This implies that a `WakeNode` with a `null` `list-next` is not inside
    //                  any list.
    // NOTE(invariant): the `list-next` pointer corresponds to a `'static` lifetime when it is
    //                  non-`null`
    list_next: Monitor<AtomicPtr<Element>>,

    /// The [`Waker`] this whole schenanigan is for.
    waker: Waker,

    /// A marker field indicating that this type cannot be [`Unpin`].
    _pinned: marker::PhantomPinned,
}

impl WakeNode {
    /// Constructs a [`WakeNode`] for a target [`AtomicWaker`].
    #[inline]
    pub const fn new(waker: Waker) -> Self {
        let list_next = Monitor::new(AtomicPtr::new(ptr::null_mut()));

        let _pinned = marker::PhantomPinned;

        Self {
            list_next,
            waker,
            _pinned,
        }
    }
}

/// An active walker in a [`WakeList`].
#[repr(transparent)]
pub struct Walker<'a>(
    // NOTE: This is the walk count.
    MonitorGuard<'a, AtomicUsize>,
    marker::PhantomData<NonNull<Self>>,
);

impl<'a> Walker<'a> {
    /// Register a linked-list walker for the target [`WakeList`].
    pub fn at(target_list: &'a WakeList) -> Self {
        let WakeList {
            walk_count: walker_count,
            ..
        } = target_list;

        // NOTE: We do not re-use the monitor guard later here to issue wakeups on
        // addition as well.
        Monitor::access(walker_count).fetch_add(1, Ordering::SeqCst);

        Self(Monitor::access(walker_count), marker::PhantomData)
    }
}

impl<'a> Drop for Walker<'a> {
    fn drop(&mut self) {
        let Self(walker_count, ..) = self;

        walker_count.fetch_sub(1, Ordering::SeqCst);
    }
}

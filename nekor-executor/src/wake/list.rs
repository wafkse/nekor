//! Interrupt-safe intrusive wait registration.
//!
//! This module implements an allocation-free, append-only list of static
//! [`WakeNode`] values. Every node is linked exactly once through exclusive
//! pinned ownership and remains in one [`WakeList`] permanently. An immutable
//! topology removes physical deletion, memory reclamation, and list ABA.
//!
//! Each node contains one [`AtomicWaker`]. A [`LinkedNode`] carries the
//! validated [`WakeTarget`] used to arm that slot whenever its event remains
//! pending. Registration arms before checking readiness. A racing wake
//! therefore either claims the target or releases its event publication to the
//! arming operation before the readiness predicate executes.
//!
//! # Interrupt safety
//!
//! Linking uses a nonblocking head compare-exchange loop. Wake traversal reads
//! only immutable links and performs one atomic exchange per node. The list can
//! be traversed recursively and can interrupt linking, registration, or
//! cancellation without waiting on list state owned by the interrupted
//! context. Hardware atomics and wake callbacks may still have target-dependent
//! latency.
//!
//! Wake callbacks inherit the interrupt-safety contract of [`WakeTarget`]. A
//! wake pass is linear in the number of permanently linked nodes, including
//! inactive nodes. Interrupt users must therefore impose a suitable bound on
//! list length.

use core::{
    cell::UnsafeCell,
    fmt, marker,
    pin::Pin,
    ptr::{self, NonNull},
    sync::atomic::{AtomicPtr, Ordering},
};

use nekor_aal::cache::padded::CachePadded;

use crate::wake::{AtomicWaker, WakeTarget};

/// An append-only intrusive list of static wake nodes.
///
/// The head remains atomic so new nodes can be published while interrupts and
/// concurrent execution are enabled.
// NOTE(invariant): Every non-null head and next pointer addresses an
// initialized static `WakeNode`. A node's next pointer becomes immutable before
// the release operation that publishes that node through `head`. Published
// nodes are never removed, moved, overwritten, or reinitialized.
pub struct WakeList(CachePadded<AtomicPtr<WakeNode>>);

impl WakeList {
    /// Constructs an empty wake list.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self(CachePadded::new(AtomicPtr::new(ptr::null_mut())))
    }

    /// Permanently links an exclusively owned static node.
    ///
    /// Consuming the pinned mutable reference proves that safe code cannot link
    /// the same node twice or issue two registration capabilities for it. The
    /// returned [`LinkedNode`] is the sole authority for registration and
    /// cancellation.
    ///
    /// This method may be called with interrupts enabled. If an interrupt or
    /// another thread publishes a different node first, this operation updates
    /// the unpublished next pointer and retries against the new head.
    ///
    /// Linking must complete before the associated wake source becomes active
    /// when registration relies on the node-slot ordering handshake alone. A
    /// node linked concurrently with an earlier wake pass is outside that pass
    /// and requires independent event-state synchronization.
    #[inline]
    pub fn link(&'static self, target_node: Pin<&'static mut WakeNode>, target_waker: WakeTarget) -> LinkedNode {
        let head = &self.0;

        // SAFETY: The node has static storage and remains pinned. Converting the
        // exclusive reference to raw pointers lets its unique borrow end before
        // the address is published to concurrent readers.
        let target_node = unsafe { Pin::into_inner_unchecked(target_node) };
        let next_address = target_node.next.get();
        let node_address = ptr::from_mut(target_node);
        let mut current_head = head.load(Ordering::Acquire);

        loop {
            // SAFETY: This node is still unpublished when the write occurs. The
            // consumed unique capability proves that no other writer can access
            // its next field.
            unsafe {
                ptr::write(next_address, current_head);
            }

            match head.compare_exchange(current_head, node_address, Ordering::AcqRel, Ordering::Acquire) {
                Ok(_) => break,
                Err(observed_head) => current_head = observed_head,
            }
        }

        // SAFETY: The release publication completed and the node has static
        // storage. Its next pointer will never be written again.
        let target_node = unsafe { node_address.as_ref().unwrap_unchecked() };

        LinkedNode::new(self, target_node, target_waker)
    }

    /// Runs one wake pass over the currently published list.
    ///
    /// Each active target encountered by the pass is atomically claimed before
    /// its callback runs. Concurrent or recursive wake passes may coalesce into
    /// one callback for a given registration.
    ///
    /// Event state must be published before this method is called. For every
    /// node already reachable at the initial head load, a racing registration
    /// either gets claimed or acquires that publication before its readiness
    /// predicate runs.
    ///
    /// Nodes linked after the initial head load are not part of this pass.
    ///
    /// Returns the number of active targets claimed and invoked.
    #[inline]
    pub fn wake_all(&'static self) -> usize {
        let head = &self.0;

        let mut current_node = head.load(Ordering::Acquire);
        let mut wake_count = usize::MIN;

        while let Some(node_address) = NonNull::new(current_node) {
            // SAFETY: The list invariant guarantees that every reachable
            // address points to an initialized static node whose fields remain
            // valid permanently.
            let target_node = unsafe { node_address.as_ref() };
            let next = &target_node.next;
            let waker = &target_node.waker;

            // SAFETY: Publication made this pointer immutable before any reader
            // could reach the node. The acquire head load observes that release
            // and every earlier publication in the chain.
            current_node = unsafe { ptr::read(next.get()) };
            wake_count += usize::from(AtomicWaker::wake(waker));
        }

        wake_count
    }
}

impl Default for WakeList {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// A permanently linked wake-registration node.
///
/// Construction produces an unlinked movable value. [`WakeList::link`] requires
/// exclusive pinned static ownership before publishing its address.
// NOTE(invariant): `next` is null while newly constructed, may be rewritten
// only by the exclusive linking operation before publication, and is immutable
// for the remainder of the program after publication. `waker` independently
// transitions between inactive and one valid tagged wake target.
pub struct WakeNode {
    /// The next older node in the append-only list.
    next: UnsafeCell<*mut Self>,

    /// The node's active tagged wake slot.
    waker: AtomicWaker,

    /// The marker that prevents movement after pinning.
    _pinned: marker::PhantomPinned,
}

impl WakeNode {
    /// Constructs an unlinked inactive node.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        let next = UnsafeCell::new(ptr::null_mut());
        let waker = AtomicWaker::null();
        let pinned = marker::PhantomPinned;

        Self {
            next,
            waker,
            _pinned: pinned,
        }
    }
}

impl Default for WakeNode {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for WakeNode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("WakeNode").finish_non_exhaustive()
    }
}

// SAFETY: Before publication the node has one exclusive owner. After
// publication its plain next pointer is immutable and its only mutable runtime
// state is an `AtomicWaker`.
unsafe impl Send for WakeNode {}

// SAFETY: Every shared access after publication observes immutable topology.
// Wake-slot mutation is synchronized by the contained atomic pointer.
unsafe impl Sync for WakeNode {}

/// Unique authority to register one node in one exact wake list.
///
/// This capability is neither cloneable nor copyable. Registration and
/// cancellation require mutable access, which prevents safe overlapping wait
/// epochs for one node. Dropping the capability cancels an active target and
/// permanently retires the node from future registration.
///
/// A callback claimed before cancellation or drop may still run afterward.
pub struct LinkedNode {
    /// The list that permanently contains the node.
    list: &'static WakeList,

    /// The static node controlled by this capability.
    node: &'static WakeNode,

    /// The fixed target installed by each registration.
    target: WakeTarget,
}

impl LinkedNode {
    /// Constructs the unique capability for a newly published node.
    #[inline]
    const fn new(list: &'static WakeList, node: &'static WakeNode, target: WakeTarget) -> Self {
        Self { list, node, target }
    }

    /// Returns the list that permanently contains this node.
    #[inline]
    #[must_use]
    pub const fn list(&self) -> &'static WakeList {
        self.list
    }

    /// Arms this node and checks whether its event is already ready.
    ///
    /// The arm is an acquiring read-modify-write and occurs before `is_ready`.
    /// A wake source must publish level-triggered or generation-backed event
    /// state before invoking [`WakeList::wake_all`]. A racing wake therefore
    /// either claims the target or releases its publication to the arm before
    /// the predicate executes.
    ///
    /// The predicate must read the event state associated with this node's list
    /// and must not block indefinitely when bounded interrupt latency is
    /// required.
    ///
    /// Callers should return their ready value after [`WaitState::Ready`]. They
    /// should return [`core::task::Poll::Pending`] after [`WaitState::Armed`]
    /// or [`WaitState::Notified`]. The notified state means a concurrent
    /// wake has already requested another poll.
    #[inline]
    pub fn register<F>(&mut self, is_ready: F) -> WaitState
    where
        F: FnOnce() -> bool,
    {
        let node = self.node;
        let target = self.target;
        let waker = &node.waker;

        let _replaced_target = AtomicWaker::arm(waker, target);

        if is_ready() {
            let _was_armed = AtomicWaker::cancel(waker);

            WaitState::Ready
        } else if AtomicWaker::is_armed(waker) {
            WaitState::Armed
        } else {
            WaitState::Notified
        }
    }

    /// Cancels an active registration.
    ///
    /// A wake that claimed the target first may still invoke its callback after
    /// this method returns.
    ///
    /// Returns whether this call cancelled an active target.
    #[inline]
    #[must_use]
    pub fn cancel(&mut self) -> bool {
        let node = self.node;
        let waker = &node.waker;

        AtomicWaker::cancel(waker)
    }

    /// Determines whether this node currently has an active registration.
    #[inline]
    #[must_use]
    pub fn is_armed(&self) -> bool {
        let node = self.node;
        let waker = &node.waker;

        AtomicWaker::is_armed(waker)
    }
}

impl Drop for LinkedNode {
    #[inline]
    fn drop(&mut self) {
        let node = self.node;
        let waker = &node.waker;

        let _was_armed = AtomicWaker::cancel(waker);
    }
}

/// The outcome of checking an event and updating its registration.
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum WaitState {
    /// The checked event was ready and no target remains armed.
    Ready,

    /// The event was pending and the target was armed when observed.
    ///
    /// A concurrent wake may claim it immediately after that observation.
    Armed,

    /// A wake claimed the target while the event was being checked.
    ///
    /// The callback has requested another poll.
    Notified,
}

#[cfg(test)]
mod tests {
    use alloc::{boxed::Box, sync::Arc};
    use core::{
        pin::Pin,
        ptr,
        sync::atomic::{AtomicUsize, Ordering},
        task::{RawWaker, RawWakerVTable, Waker},
    };
    use std::{sync::Barrier, thread};

    use super::{LinkedNode, WaitState, WakeList, WakeNode};
    use crate::wake::{TEST_VTABLE, WakeTarget};

    #[repr(C, align(8))]
    struct CountingWake(AtomicUsize);

    impl CountingWake {
        const fn new() -> Self {
            Self(AtomicUsize::new(usize::MIN))
        }

        fn count(&self) -> usize {
            self.0.load(Ordering::SeqCst)
        }
    }

    static FOREIGN_VTABLE: RawWakerVTable = const {
        unsafe fn clone(data: *const ()) -> RawWaker {
            RawWaker::new(data, &FOREIGN_VTABLE)
        }

        unsafe fn wake(_: *const ()) {}

        const fn drop(_: *const ()) {}

        RawWakerVTable::new(clone, wake, wake, drop)
    };

    fn list() -> &'static WakeList {
        Box::leak(Box::new(WakeList::new()))
    }

    fn node() -> Pin<&'static mut WakeNode> {
        let target_node = Box::leak(Box::new(WakeNode::new()));

        // SAFETY: The leaked node has static storage and cannot be moved through
        // any other safe reference after this unique reference is consumed.
        unsafe { Pin::new_unchecked(target_node) }
    }

    fn counter() -> &'static CountingWake {
        Box::leak(Box::new(CountingWake::new()))
    }

    fn target(target_counter: &'static CountingWake) -> WakeTarget {
        let counter_address = ptr::from_ref(&target_counter.0).cast::<()>();

        // SAFETY: `counter_address` points to a static aligned `AtomicUsize`, as
        // required by the test vtable.
        let target_waker = unsafe { Waker::new(counter_address, &TEST_VTABLE) };
        let target = WakeTarget::new(&target_waker);

        assert!(target.is_some());

        let Some(target) = target else {
            unreachable!();
        };

        target
    }

    fn linked(target_list: &'static WakeList) -> (LinkedNode, &'static CountingWake) {
        let target_counter = counter();
        let target_link = target_list.link(node(), target(target_counter));

        (target_link, target_counter)
    }

    #[test]
    fn foreign_waker_is_rejected() {
        let target_counter = counter();
        let counter_address = ptr::from_ref(&target_counter.0).cast::<()>();

        // SAFETY: The foreign table does not dereference its data pointer.
        let target_waker = unsafe { Waker::new(counter_address, &FOREIGN_VTABLE) };

        assert!(WakeTarget::new(&target_waker).is_none());
    }

    #[test]
    fn empty_list_wakes_nothing() {
        assert_eq!(list().wake_all(), 0);
    }

    #[test]
    fn linked_node_is_woken_once_per_registration() {
        let target_list = list();
        let (mut target_link, target_counter) = linked(target_list);

        assert_eq!(target_link.register(|| false), WaitState::Armed);
        assert!(target_link.is_armed());
        assert_eq!(target_list.wake_all(), 1);
        assert_eq!(target_counter.count(), 1);
        assert!(!target_link.is_armed());
        assert_eq!(target_list.wake_all(), 0);
        assert_eq!(target_counter.count(), 1);
    }

    #[test]
    fn ready_condition_does_not_leave_target_armed() {
        let target_list = list();
        let (mut target_link, target_counter) = linked(target_list);

        assert_eq!(target_link.register(|| true), WaitState::Ready);
        assert!(!target_link.is_armed());
        assert_eq!(target_list.wake_all(), 0);
        assert_eq!(target_counter.count(), 0);
    }

    #[test]
    fn cancellation_consumes_registration() {
        let target_list = list();
        let (mut target_link, target_counter) = linked(target_list);

        assert_eq!(target_link.register(|| false), WaitState::Armed);
        assert!(target_link.cancel());
        assert!(!target_link.cancel());
        assert_eq!(target_list.wake_all(), 0);
        assert_eq!(target_counter.count(), 0);
    }

    #[test]
    fn dropping_capability_cancels_registration() {
        let target_list = list();
        let (mut target_link, target_counter) = linked(target_list);

        assert_eq!(target_link.register(|| false), WaitState::Armed);

        drop(target_link);

        assert_eq!(target_list.wake_all(), 0);
        assert_eq!(target_counter.count(), 0);
    }

    #[test]
    fn node_can_be_rearmed_after_wake() {
        let target_list = list();
        let (mut target_link, target_counter) = linked(target_list);

        assert_eq!(target_link.register(|| false), WaitState::Armed);
        assert_eq!(target_list.wake_all(), 1);
        assert_eq!(target_link.register(|| false), WaitState::Armed);
        assert_eq!(target_list.wake_all(), 1);
        assert_eq!(target_counter.count(), 2);
    }

    #[test]
    fn wake_during_predicate_reports_notification() {
        let target_list = list();
        let (mut target_link, target_counter) = linked(target_list);

        let wait_state = target_link.register(|| {
            assert_eq!(target_list.wake_all(), 1);

            false
        });

        assert_eq!(wait_state, WaitState::Notified);
        assert_eq!(target_counter.count(), 1);
        assert!(!target_link.is_armed());
    }

    #[test]
    fn wake_all_visits_every_published_node() {
        let target_list = list();
        let (mut first_link, first_counter) = linked(target_list);
        let (mut second_link, second_counter) = linked(target_list);
        let (mut third_link, third_counter) = linked(target_list);

        assert_eq!(first_link.register(|| false), WaitState::Armed);
        assert_eq!(second_link.register(|| false), WaitState::Armed);
        assert_eq!(third_link.register(|| false), WaitState::Armed);
        assert_eq!(target_list.wake_all(), 3);
        assert_eq!(first_counter.count(), 1);
        assert_eq!(second_counter.count(), 1);
        assert_eq!(third_counter.count(), 1);
    }

    #[test]
    fn nested_wake_passes_coalesce() {
        let target_list = list();
        let (mut target_link, target_counter) = linked(target_list);

        assert_eq!(target_link.register(|| false), WaitState::Armed);
        assert_eq!(target_list.wake_all(), 1);
        assert_eq!(target_list.wake_all(), 0);
        assert_eq!(target_counter.count(), 1);
    }

    #[test]
    fn concurrent_registration_and_wake_do_not_lose_event() {
        let target_list = list();
        let (mut target_link, target_counter) = linked(target_list);
        let rendezvous = Arc::new(Barrier::new(2));
        let register_rendezvous = Arc::clone(&rendezvous);

        let register_thread = thread::spawn(move || {
            let wait_state = target_link.register(|| {
                let _rendezvous_state = register_rendezvous.wait();

                false
            });

            (wait_state, target_link)
        });

        let _rendezvous_state = rendezvous.wait();
        let wake_count = target_list.wake_all();
        let register_result = register_thread.join().ok();

        assert!(register_result.is_some());

        let Some((register_state, target_link)) = register_result else {
            return;
        };

        assert!(matches!(register_state, WaitState::Armed | WaitState::Notified));
        assert!(wake_count <= 1);
        assert_eq!(target_counter.count(), 1);
        assert!(!target_link.is_armed());
    }

    #[test]
    fn concurrent_wake_passes_claim_target_once() {
        let target_list = list();
        let (mut target_link, target_counter) = linked(target_list);
        let rendezvous = Arc::new(Barrier::new(3));
        let first_rendezvous = Arc::clone(&rendezvous);
        let second_rendezvous = Arc::clone(&rendezvous);

        assert_eq!(target_link.register(|| false), WaitState::Armed);

        let first_thread = thread::spawn(move || {
            let _rendezvous_state = first_rendezvous.wait();

            target_list.wake_all()
        });
        let second_thread = thread::spawn(move || {
            let _rendezvous_state = second_rendezvous.wait();

            target_list.wake_all()
        });

        let _rendezvous_state = rendezvous.wait();
        let first_count = first_thread.join().ok();
        let second_count = second_thread.join().ok();

        assert!(matches!(
            (first_count, second_count),
            (Some(1), Some(0)) | (Some(0), Some(1))
        ));
        assert_eq!(target_counter.count(), 1);
        assert!(!target_link.is_armed());
    }

    #[test]
    fn concurrent_distinct_nodes_remain_reachable() {
        let target_list = list();
        let first_counter = counter();
        let second_counter = counter();
        let first_node = node();
        let second_node = node();
        let first_target = target(first_counter);
        let second_target = target(second_counter);
        let rendezvous = Arc::new(Barrier::new(3));
        let first_rendezvous = Arc::clone(&rendezvous);
        let second_rendezvous = Arc::clone(&rendezvous);

        let first_thread = thread::spawn(move || {
            let _rendezvous_state = first_rendezvous.wait();

            target_list.link(first_node, first_target)
        });
        let second_thread = thread::spawn(move || {
            let _rendezvous_state = second_rendezvous.wait();

            target_list.link(second_node, second_target)
        });

        let _rendezvous_state = rendezvous.wait();
        let first_link = first_thread.join().ok();
        let second_link = second_thread.join().ok();

        assert!(first_link.is_some());
        assert!(second_link.is_some());

        let Some(mut first_link) = first_link else {
            return;
        };
        let Some(mut second_link) = second_link else {
            return;
        };

        assert_eq!(first_link.register(|| false), WaitState::Armed);
        assert_eq!(second_link.register(|| false), WaitState::Armed);
        assert_eq!(target_list.wake_all(), 2);
        assert_eq!(first_counter.count(), 1);
        assert_eq!(second_counter.count(), 1);
    }
}

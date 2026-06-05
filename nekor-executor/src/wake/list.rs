//! Intrusive wait queue for event-driven task coordination.
//!
//! This module implements a lock-free circular singly-linked list of [`Waker`]s
//! waiting on external events. Each waiting task embeds a [`Node`] containing
//! its waker, which is linked into the list without allocation.
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
    fmt, marker,
    mem::MaybeUninit,
    pin::Pin,
    ptr::{self},
    sync::atomic::{AtomicPtr, AtomicUsize, Ordering},
};

use nekor_aal::signal::monitor::{Monitor, Monitored};

use crate::wake::AtomicWaker;

/// A dummy type to serve as a node in a wait queue or a root of a wait queue.
///
/// This is not used directly, but rather as marker for the node pointer type.
enum RootOrNode {}

/// An atomic [`Waker`]-holding node.
///
/// This is to be preserved in the [`Future`] for wake-up requests.
pub struct WakeNode {
    /// The atomic pointer to the next
    // NOTE(invariant): If `next node` is null, the node is not present in any
    // wait queue.
    next_node: AtomicPtr<RootOrNode>,

    /// The Nekor-specific atomic [`Waker`], used to issue task wake-up.
    ///
    /// This can be uninitialized, in which case it is not associated with any
    /// task, be it due to as a requirement or cancellation of a wake-up.
    node_waker: AtomicWaker,

    /// The marker for the node to avoid this type from being unpin.
    ///
    /// This is required as a result of the self-referential nature of the
    /// type.
    _marker: marker::PhantomPinned,
}

impl fmt::Debug for WakeNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WakeNode").finish_non_exhaustive()
    }
}

impl WakeNode {
    /// Creates a new [`WakeNode`] with the given target [`AtomicWaker`].
    ///
    /// Note that this will need to be pinned in-place to actually partake in a
    /// [`WaitQueue`].
    #[inline]
    pub const fn new(node_waker: AtomicWaker) -> Self {
        let next_node = AtomicPtr::new(ptr::null_mut());

        let marker = marker::PhantomPinned;

        Self {
            next_node,
            node_waker,
            _marker: marker,
        }
    }

    /// Returns a reference to the [`AtomicWaker`] associated with this node.
    #[inline]
    pub const fn waker(&self) -> &AtomicWaker {
        let Self { node_waker, .. } = self;

        node_waker
    }
}

/// A lock-free, unbounded queue implemented as a circular linked list.
///
/// This is used to park multiple [`AtomicWaker`]s as waiters for a specific
/// event, without requiring any additional allocation.
// NOTE(invariant): Requires `repr(C)` to have `WakeNode` be the first field.
#[repr(C)]
pub struct WaitQueue {
    /// The atomic pointer to the head of the queue.
    head_node: Monitor<AtomicPtr<RootOrNode>>,

    /// The count of simultaneous walkers in the wait queue.
    ///
    /// This is required for concurrent wake-all, and wake-and-consume-all
    /// operations.
    // NOTE: When a complete detach is issued the link between detached nodes
    // and the original root node must not be severed if this value is not
    // zero.
    walker_count: Monitor<AtomicUsize>,
}

// SAFETY: WaitQueue is designed for lock-free concurrent access. The internal
// Monitor types use atomic operations for synchronization, and WakeNode access
// is protected by the next_node atomic pointer invariant.
unsafe impl Send for WaitQueue {}

// SAFETY: WaitQueue supports concurrent access from multiple threads. All
// mutations are performed through atomic operations with appropriate ordering.
// The circular list structure is maintained through compare-exchange
// operations.
unsafe impl Sync for WaitQueue {}

impl WaitQueue {
    /// Construct the wait queue at a pinned, uninitialized storage location.
    ///
    /// Pinning is required to maintain invariants upon construction of the wait
    /// queue.
    ///
    /// # Safety
    ///
    /// The storage of this wait queue must not be invalidated or otherwise
    /// deallocated until it is dropped.
    #[inline]
    #[must_use]
    pub const unsafe fn new_in(target_storage: Pin<&mut MaybeUninit<Self>>) -> Pin<&mut Self> {
        // SAFETY: The value is effectively uninitialized and is not moved.
        let target_storage = unsafe { Pin::get_unchecked_mut(target_storage) };

        let simulatenous_walkers = Monitor::new(AtomicUsize::new(usize::MIN));

        let storage_address = target_storage.as_ptr().cast_mut().cast::<RootOrNode>();

        let head_node = Monitor::new(AtomicPtr::new(storage_address));

        let target_value = target_storage.write(Self {
            head_node,
            walker_count: simulatenous_walkers,
        });

        // SAFETY: The caller gave us a mutable reference to this same pinned
        // data.
        unsafe { Pin::new_unchecked(target_value) }
    }

    /// Attempt to insert a node into the wait queue.
    ///
    /// # Errors
    ///
    /// Returns an error if the node is already in the list.
    ///
    /// # Remarks
    ///
    /// This associated function does not imply that insertion can fail, but
    /// only that the node cannot be inserted if it does not satisfy the
    /// "not included in another list" invariant.
    ///
    /// # Safety
    ///
    /// The node must be removed from the list before the memory backing it
    /// is reused or deallocated.
    pub unsafe fn try_insert(self: Pin<&Self>, target_node: Pin<&WakeNode>) -> Result<(), ()> {
        let Self { head_node, .. } = self.get_ref();

        // NOTE(invariant): A null `node->next` implies insertion is possible.
        if !target_node.next_node.load(Ordering::Acquire).is_null() {
            return Err(());
        }

        let monitored_head = Monitor::access(head_node);

        let mut target_address = monitored_head.load(Ordering::Acquire);

        loop {
            // NOTE(invariant): This is done before the compare-exchange to
            // ensure that the circular invariant is maintained.
            target_node
                .next_node
                .store(target_address, Ordering::Release);

            if monitored_head
                .compare_exchange_weak(
                    target_address,
                    core::ptr::from_ref(target_node.get_ref()) as *mut RootOrNode,
                    Ordering::AcqRel,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                break Ok(());
            }

            Monitored::wait(Monitor::engage(head_node));

            target_address = monitored_head.load(Ordering::Acquire);
        }
    }

    /// Attempt to remove a node from the wait queue.
    ///
    /// # Errors
    ///
    /// This function will fail if the specified node is not present in *any*
    /// wait queue ([`RemoveError::NotInQueue`]), or is not in the same wait
    /// queue as this one ([`RemoveError::Outside`]).
    ///
    /// # Remarks
    ///
    /// This performs a detach-filter-reinsert operation that rebuilds the list
    /// without the target node (`O(n)`).
    ///
    /// As consequence, simultaneous walkers engaging the list after the removal
    /// operation has started will observe an empty wait queue.
    ///
    /// Prefer deactivation of the [`WakeNode`] through the
    /// [`WakeNode::deactivate`] associated function, which is `O(1)`.
    pub fn try_remove(self: Pin<&Self>, target_node: Pin<&WakeNode>) -> Result<(), RemoveError> {
        let Self {
            head_node,
            walker_count,
        } = self.get_ref();

        // NOTE(invariant): A null `node->next` implies the node is not in any
        // queue.
        if target_node.next_node.load(Ordering::Acquire).is_null() {
            return Err(RemoveError::NotInQueue);
        }

        let monitored_head = Monitor::access(head_node);

        // Use CAS loop to atomically detach the list - only one thread can
        // succeed at detaching at a time
        let detached_head = loop {
            let current_head = monitored_head.load(Ordering::Acquire);

            // If queue is already empty (severed), fail
            if ptr::addr_eq(current_head.cast::<Self>(), self.get_ref()) {
                if target_node.next_node.load(Ordering::Acquire).is_null() {
                    return Err(RemoveError::NotInQueue);
                }

                return Err(RemoveError::Outside);
            }

            match monitored_head.compare_exchange(
                current_head,
                core::ptr::from_ref(self.get_ref()) as *mut RootOrNode,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => break current_head.cast::<WakeNode>(),
                Err(_) => {
                    // Another thread is modifying, wait and retry
                    Monitored::wait(Monitor::engage(head_node));
                }
            }
        };

        let monitored_walker_count = Monitor::access(walker_count);

        // NOTE: We need to wait until no more walkers are interacting with the
        // queue. This count, once zero, will never grow back as the queue is
        // effectively empty right now (detached head prevents new walkers).
        while monitored_walker_count.load(Ordering::Acquire) != 0 {
            Monitored::wait(Monitor::engage(walker_count));
        }

        let mut current_node = detached_head;

        let mut in_list = false;

        loop {
            if ptr::addr_eq(current_node, self.get_ref()) {
                if in_list {
                    break Ok(());
                }

                break Err(RemoveError::Outside);
            }

            // SAFETY: The next node in a circular list is never null. No other
            // mutable reference to the node can exist.
            let node_ref = unsafe { Pin::new_unchecked(current_node.as_ref().unwrap_unchecked()) };

            current_node = node_ref
                .next_node
                .load(Ordering::Acquire)
                .cast::<WakeNode>();

            let reinsert_node = if ptr::eq(node_ref.get_ref(), target_node.get_ref()) {
                in_list = true;

                false
            } else {
                true
            };

            node_ref.next_node.store(ptr::null_mut(), Ordering::Release);

            if reinsert_node {
                // SAFETY: The node has been inserted previously, so the
                // contract is still maintained for the same wait queue.
                //
                // We set the next node to null here. We can assume this always
                // succeeds.
                unsafe {
                    let _ = self.try_insert(node_ref);
                };
            }
        }
    }
}

/// An error that can occur when attempting to remove a node from a wait queue.
///
/// See the [`WaitQueue::try_remove`] and [`WaitQueue::remove`] associated
/// function family for more information.
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum RemoveError {
    /// The wait node is not in the same wait queue as this one.
    Outside,

    /// The wait node was not in any wait queue.
    NotInQueue,

    /// The removal operation was contended.
    Contended,
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::task::{RawWaker, RawWakerVTable, Waker};

    // Helper function to create a dummy waker for testing
    #[allow(dead_code)]
    unsafe fn dummy_raw_waker() -> RawWaker {
        unsafe fn clone(_: *const ()) -> RawWaker {
            unsafe { dummy_raw_waker() }
        }
        unsafe fn wake(_: *const ()) {}
        unsafe fn wake_by_ref(_: *const ()) {}
        unsafe fn drop(_: *const ()) {}

        const VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);

        RawWaker::new(ptr::null(), &VTABLE)
    }

    #[allow(dead_code)]
    fn dummy_waker() -> Waker {
        unsafe { Waker::from_raw(dummy_raw_waker()) }
    }

    // ============================================================================
    // WakeNode Tests
    // ============================================================================

    #[test]
    fn test_wake_node_new() {
        let atomic_waker = AtomicWaker::null();
        let node = WakeNode::new(atomic_waker);

        // Verify node is not in any queue initially
        assert!(node.next_node.load(Ordering::Acquire).is_null());
    }

    #[test]
    fn test_wake_node_waker_access() {
        let atomic_waker = AtomicWaker::null();
        let node = WakeNode::new(atomic_waker);

        // Verify we can access the waker
        let _waker_ref = node.waker();
        assert!(node.waker().materialize().is_none());
    }

    #[test]
    fn test_wake_node_pinning() {
        let atomic_waker = AtomicWaker::null();
        let node = WakeNode::new(atomic_waker);
        let boxed_node = Box::pin(node);

        // Verify pinned node can be used
        assert!(
            boxed_node
                .as_ref()
                .next_node
                .load(Ordering::Acquire)
                .is_null()
        );
    }

    // ============================================================================
    // WaitQueue Basic Tests
    // ============================================================================

    #[test]
    fn test_wait_queue_new() {
        let mut storage = Box::pin(MaybeUninit::<WaitQueue>::uninit());
        let queue = unsafe { WaitQueue::new_in(storage.as_mut()) };

        // Verify queue is initialized
        let head = queue.head_node.load(Ordering::Acquire);
        assert!(!head.is_null());

        // Head should point to self (empty circular list)
        assert!(ptr::addr_eq(
            head.cast::<WaitQueue>(),
            queue.as_ref().get_ref() as *const _
        ));
    }

    #[test]
    fn test_wait_queue_insert_single_node() {
        let mut queue_storage = Box::pin(MaybeUninit::<WaitQueue>::uninit());
        let queue = unsafe { WaitQueue::new_in(queue_storage.as_mut()) };

        let atomic_waker = AtomicWaker::null();
        let node = Box::pin(WakeNode::new(atomic_waker));

        // Insert should succeed
        let result = unsafe { queue.as_ref().try_insert(node.as_ref()) };
        assert!(result.is_ok());

        // Node should now be in the list
        assert!(!node.next_node.load(Ordering::Acquire).is_null());
    }

    #[test]
    fn test_wait_queue_insert_duplicate_fails() {
        let mut queue_storage = Box::pin(MaybeUninit::<WaitQueue>::uninit());
        let queue = unsafe { WaitQueue::new_in(queue_storage.as_mut()) };

        let atomic_waker = AtomicWaker::null();
        let node = Box::pin(WakeNode::new(atomic_waker));

        // First insert should succeed
        let result = unsafe { queue.as_ref().try_insert(node.as_ref()) };
        assert!(result.is_ok());

        // Second insert of same node should fail
        let result = unsafe { queue.as_ref().try_insert(node.as_ref()) };
        assert!(result.is_err());
    }

    #[test]
    fn test_wait_queue_insert_multiple_nodes() {
        let mut queue_storage = Box::pin(MaybeUninit::<WaitQueue>::uninit());
        let queue = unsafe { WaitQueue::new_in(queue_storage.as_mut()) };

        let nodes: Vec<_> = (0..5)
            .map(|_| Box::pin(WakeNode::new(AtomicWaker::null())))
            .collect();

        // Insert all nodes
        for node in &nodes {
            let result = unsafe { queue.as_ref().try_insert(node.as_ref()) };
            assert!(result.is_ok());
        }

        // Verify all nodes are in the list
        for node in &nodes {
            assert!(!node.next_node.load(Ordering::Acquire).is_null());
        }
    }

    // ============================================================================
    // WaitQueue Remove Tests
    // ============================================================================

    #[test]
    fn test_wait_queue_remove_not_in_queue() {
        let mut queue_storage = Box::pin(MaybeUninit::<WaitQueue>::uninit());
        let queue = unsafe { WaitQueue::new_in(queue_storage.as_mut()) };

        let atomic_waker = AtomicWaker::null();
        let node = Box::pin(WakeNode::new(atomic_waker));

        // Remove should fail - node not in queue
        let result = queue.as_ref().try_remove(node.as_ref());
        assert_eq!(result, Err(RemoveError::NotInQueue));
    }

    #[test]
    fn test_wait_queue_remove_single_node() {
        let mut queue_storage = Box::pin(MaybeUninit::<WaitQueue>::uninit());
        let queue = unsafe { WaitQueue::new_in(queue_storage.as_mut()) };

        let atomic_waker = AtomicWaker::null();
        let node = Box::pin(WakeNode::new(atomic_waker));

        // Insert node
        let result = unsafe { queue.as_ref().try_insert(node.as_ref()) };
        assert!(result.is_ok());

        // Remove node
        let result = queue.as_ref().try_remove(node.as_ref());
        assert!(result.is_ok());

        // Node should no longer be in list
        assert!(node.next_node.load(Ordering::Acquire).is_null());
    }

    #[test]
    fn test_wait_queue_remove_from_multiple() {
        let mut queue_storage = Box::pin(MaybeUninit::<WaitQueue>::uninit());
        let queue = unsafe { WaitQueue::new_in(queue_storage.as_mut()) };

        let nodes: Vec<_> = (0..5)
            .map(|_| Box::pin(WakeNode::new(AtomicWaker::null())))
            .collect();

        // Insert all nodes
        for node in &nodes {
            let result = unsafe { queue.as_ref().try_insert(node.as_ref()) };
            assert!(result.is_ok());
        }

        // Remove middle node
        let result = queue.as_ref().try_remove(nodes[2].as_ref());
        assert!(result.is_ok());

        // Removed node should not be in list
        assert!(nodes[2].next_node.load(Ordering::Acquire).is_null());

        // Other nodes should still be in list
        for i in [0, 1, 3, 4] {
            assert!(!nodes[i].next_node.load(Ordering::Acquire).is_null());
        }
    }

    #[test]
    fn test_wait_queue_remove_all_nodes() {
        let mut queue_storage = Box::pin(MaybeUninit::<WaitQueue>::uninit());
        let queue = unsafe { WaitQueue::new_in(queue_storage.as_mut()) };

        let nodes: Vec<_> = (0..3)
            .map(|_| Box::pin(WakeNode::new(AtomicWaker::null())))
            .collect();

        // Insert all nodes
        for node in &nodes {
            let result = unsafe { queue.as_ref().try_insert(node.as_ref()) };
            assert!(result.is_ok());
        }

        // Remove all nodes
        for node in &nodes {
            let result = queue.as_ref().try_remove(node.as_ref());
            assert!(result.is_ok());
        }

        // All nodes should not be in list
        for node in &nodes {
            assert!(node.next_node.load(Ordering::Acquire).is_null());
        }
    }

    #[test]
    fn test_wait_queue_remove_wrong_queue() {
        let mut queue1_storage = Box::pin(MaybeUninit::<WaitQueue>::uninit());
        let queue1 = unsafe { WaitQueue::new_in(queue1_storage.as_mut()) };

        let mut queue2_storage = Box::pin(MaybeUninit::<WaitQueue>::uninit());
        let queue2 = unsafe { WaitQueue::new_in(queue2_storage.as_mut()) };

        let atomic_waker = AtomicWaker::null();
        let node = Box::pin(WakeNode::new(atomic_waker));

        // Insert into queue1
        let result = unsafe { queue1.as_ref().try_insert(node.as_ref()) };
        assert!(result.is_ok());

        // Try to remove from queue2 - should fail with Outside
        let result = queue2.as_ref().try_remove(node.as_ref());
        assert_eq!(result, Err(RemoveError::Outside));
    }

    // ============================================================================
    // WaitQueue Circular List Invariant Tests
    // ============================================================================

    #[test]
    fn test_wait_queue_circular_invariant_single() {
        let mut queue_storage = Box::pin(MaybeUninit::<WaitQueue>::uninit());
        let queue = unsafe { WaitQueue::new_in(queue_storage.as_mut()) };

        let atomic_waker = AtomicWaker::null();
        let node = Box::pin(WakeNode::new(atomic_waker));

        unsafe { queue.as_ref().try_insert(node.as_ref()).unwrap() };

        // Follow the circular list
        let _head = queue.head_node.load(Ordering::Acquire);
        let node_next = node.next_node.load(Ordering::Acquire);

        // Node should point back to queue root
        assert!(ptr::addr_eq(
            node_next.cast::<WaitQueue>(),
            queue.as_ref().get_ref() as *const _
        ));
    }

    #[test]
    fn test_wait_queue_circular_invariant_multiple() {
        let mut queue_storage = Box::pin(MaybeUninit::<WaitQueue>::uninit());
        let queue = unsafe { WaitQueue::new_in(queue_storage.as_mut()) };

        let nodes: Vec<_> = (0..3)
            .map(|_| Box::pin(WakeNode::new(AtomicWaker::null())))
            .collect();

        for node in &nodes {
            unsafe { queue.as_ref().try_insert(node.as_ref()).unwrap() };
        }

        // Walk the circular list and ensure we return to root
        let mut current = queue.head_node.load(Ordering::Acquire);
        let mut visited = 0;

        for _ in 0..10 {
            // Prevent infinite loop
            if ptr::addr_eq(
                current.cast::<WaitQueue>(),
                queue.as_ref().get_ref() as *const _,
            ) {
                if visited > 0 {
                    break; // Successfully completed circle
                }
            } else {
                visited += 1;
            }

            let node_ptr = current.cast::<WakeNode>();
            let node_ref = unsafe { &*node_ptr };
            current = node_ref.next_node.load(Ordering::Acquire);
        }

        assert_eq!(visited, 3); // Should have visited all 3 nodes
    }

    // ============================================================================
    // Edge Cases and Stress Tests
    // ============================================================================

    #[test]
    fn test_wait_queue_insert_remove_reinsert() {
        let mut queue_storage = Box::pin(MaybeUninit::<WaitQueue>::uninit());
        let queue = unsafe { WaitQueue::new_in(queue_storage.as_mut()) };

        let atomic_waker = AtomicWaker::null();
        let node = Box::pin(WakeNode::new(atomic_waker));

        // Insert
        unsafe { queue.as_ref().try_insert(node.as_ref()).unwrap() };

        // Remove
        queue.as_ref().try_remove(node.as_ref()).unwrap();

        // Re-insert should succeed
        let result = unsafe { queue.as_ref().try_insert(node.as_ref()) };
        assert!(result.is_ok());
    }

    #[test]
    fn test_wait_queue_many_nodes() {
        let mut queue_storage = Box::pin(MaybeUninit::<WaitQueue>::uninit());
        let queue = unsafe { WaitQueue::new_in(queue_storage.as_mut()) };

        let nodes: Vec<_> = (0..20)
            .map(|_| Box::pin(WakeNode::new(AtomicWaker::null())))
            .collect();

        // Insert all nodes
        for node in &nodes {
            let result = unsafe { queue.as_ref().try_insert(node.as_ref()) };
            assert!(result.is_ok());
        }

        // Remove every other node
        for i in (0..20).step_by(2) {
            let result = queue.as_ref().try_remove(nodes[i].as_ref());
            assert!(result.is_ok());
        }

        // Verify removed nodes are not in list
        for i in (0..20).step_by(2) {
            assert!(nodes[i].next_node.load(Ordering::Acquire).is_null());
        }

        // Verify remaining nodes are still in list
        for i in (1..20).step_by(2) {
            assert!(!nodes[i].next_node.load(Ordering::Acquire).is_null());
        }
    }

    #[test]
    fn test_wait_queue_walker_count_starts_zero() {
        let mut queue_storage = Box::pin(MaybeUninit::<WaitQueue>::uninit());
        let queue = unsafe { WaitQueue::new_in(queue_storage.as_mut()) };

        assert_eq!(queue.walker_count.load(Ordering::Acquire), 0);
    }

    #[test]
    fn test_atomic_waker_null() {
        let waker = AtomicWaker::null();
        assert!(waker.materialize().is_none());
    }

    // ============================================================================
    // Memory Safety Tests
    // ============================================================================

    #[test]
    fn test_node_drops_after_removal() {
        let mut queue_storage = Box::pin(MaybeUninit::<WaitQueue>::uninit());
        let queue = unsafe { WaitQueue::new_in(queue_storage.as_mut()) };

        {
            let atomic_waker = AtomicWaker::null();
            let node = Box::pin(WakeNode::new(atomic_waker));

            unsafe { queue.as_ref().try_insert(node.as_ref()).unwrap() };
            queue.as_ref().try_remove(node.as_ref()).unwrap();

            // Node goes out of scope here
        }

        // Queue should still be valid
        let head = queue.head_node.load(Ordering::Acquire);
        assert!(!head.is_null());
    }

    #[test]
    fn test_multiple_queues_independent() {
        let mut queue1_storage = Box::pin(MaybeUninit::<WaitQueue>::uninit());
        let queue1 = unsafe { WaitQueue::new_in(queue1_storage.as_mut()) };

        let mut queue2_storage = Box::pin(MaybeUninit::<WaitQueue>::uninit());
        let queue2 = unsafe { WaitQueue::new_in(queue2_storage.as_mut()) };

        let node1 = Box::pin(WakeNode::new(AtomicWaker::null()));
        let node2 = Box::pin(WakeNode::new(AtomicWaker::null()));

        // Insert into different queues
        unsafe { queue1.as_ref().try_insert(node1.as_ref()).unwrap() };
        unsafe { queue2.as_ref().try_insert(node2.as_ref()).unwrap() };

        // Both should be in their respective queues
        assert!(!node1.next_node.load(Ordering::Acquire).is_null());
        assert!(!node2.next_node.load(Ordering::Acquire).is_null());
    }

    // ============================================================================
    // Concurrent Tests (requires std and threads)
    // ============================================================================

    #[cfg(all(usermode, not(miri)))]
    #[test]
    fn test_concurrent_insertions() {
        use std::sync::Arc;
        use std::thread;

        let queue_storage = Box::pin(MaybeUninit::<WaitQueue>::uninit());
        let queue_storage = unsafe { Box::leak(Pin::into_inner_unchecked(queue_storage)) };
        let queue = unsafe { WaitQueue::new_in(Pin::new_unchecked(queue_storage)) };
        let queue = Arc::new(queue.into_ref());

        let num_threads = 4;
        let nodes_per_thread = 10;

        let mut handles = vec![];
        let all_nodes: Arc<Vec<_>> = Arc::new(
            (0..num_threads * nodes_per_thread)
                .map(|_| Box::pin(WakeNode::new(AtomicWaker::null())))
                .collect(),
        );

        for t in 0..num_threads {
            let nodes = Arc::clone(&all_nodes);
            let q = Arc::clone(&queue);
            let handle = thread::spawn(move || {
                let start = t * nodes_per_thread;
                let end = start + nodes_per_thread;
                for i in start..end {
                    unsafe {
                        // Keep trying until insertion succeeds
                        while q.try_insert(nodes[i].as_ref()).is_err() {
                            thread::yield_now();
                        }
                    }
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all nodes are in the queue
        let inserted_count = all_nodes
            .iter()
            .filter(|node| !node.next_node.load(Ordering::Acquire).is_null())
            .count();

        assert_eq!(inserted_count, num_threads * nodes_per_thread);
    }

    #[cfg(all(usermode, not(miri)))]
    #[test]
    fn test_concurrent_insert_and_remove() {
        use std::sync::Arc;
        use std::sync::atomic::AtomicBool;
        use std::thread;
        use std::time::Duration;

        let queue_storage = Box::pin(MaybeUninit::<WaitQueue>::uninit());
        let queue_storage = unsafe { Box::leak(Pin::into_inner_unchecked(queue_storage)) };
        let queue = unsafe { WaitQueue::new_in(Pin::new_unchecked(queue_storage)) };
        let queue = Arc::new(queue.into_ref());

        let num_inserters = 2;
        let num_removers = 2;
        let nodes_per_inserter = 20;

        let stop = Arc::new(AtomicBool::new(false));
        let all_nodes: Arc<Vec<_>> = Arc::new(
            (0..num_inserters * nodes_per_inserter)
                .map(|_| Box::pin(WakeNode::new(AtomicWaker::null())))
                .collect(),
        );
        let mut inserter_handles = vec![];
        let mut remover_handles = vec![];

        // Spawn inserter threads
        for t in 0..num_inserters {
            let nodes = Arc::clone(&all_nodes);
            let q = Arc::clone(&queue);
            let handle = thread::spawn(move || {
                let start = t * nodes_per_inserter;
                let end = start + nodes_per_inserter;
                for i in start..end {
                    unsafe {
                        while q.try_insert(nodes[i].as_ref()).is_err() {
                            thread::yield_now();
                        }
                    }
                    thread::sleep(Duration::from_micros(10));
                }
            });
            inserter_handles.push(handle);
        }

        // Spawn remover threads
        for _ in 0..num_removers {
            let nodes = Arc::clone(&all_nodes);
            let q = Arc::clone(&queue);
            let stop_flag = Arc::clone(&stop);

            let handle = thread::spawn(move || {
                let mut removed = 0;
                while !stop_flag.load(Ordering::Acquire) {
                    for node in nodes.iter() {
                        if !node.next_node.load(Ordering::Acquire).is_null() {
                            if q.try_remove(node.as_ref()).is_ok() {
                                removed += 1;
                            }
                        }
                    }
                    thread::yield_now();
                }
                removed
            });
            remover_handles.push(handle);
        }

        // Wait for inserters
        for handle in inserter_handles {
            handle.join().unwrap();
        }

        stop.store(true, Ordering::Release);

        // Wait for removers
        let mut total_removed = 0;
        for handle in remover_handles {
            total_removed += handle.join().unwrap();
        }

        // Some nodes should have been removed
        assert!(total_removed > 0);
    }

    #[cfg(all(usermode, not(miri)))]
    #[test]
    fn test_concurrent_removals_same_nodes() {
        use std::sync::Arc;
        use std::sync::atomic::AtomicUsize;
        use std::thread;

        let queue_storage = Box::pin(MaybeUninit::<WaitQueue>::uninit());
        let queue_storage = unsafe { Box::leak(Pin::into_inner_unchecked(queue_storage)) };
        let queue = unsafe { WaitQueue::new_in(Pin::new_unchecked(queue_storage)) };
        let queue = Arc::new(queue.into_ref());

        let num_nodes = 10;
        let num_removers = 4;

        // Create and insert nodes
        let nodes: Arc<Vec<_>> = Arc::new(
            (0..num_nodes)
                .map(|_| Box::pin(WakeNode::new(AtomicWaker::null())))
                .collect(),
        );

        for node in nodes.iter() {
            unsafe {
                queue.try_insert(node.as_ref()).unwrap();
            }
        }

        let successful_removals = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];

        // Spawn multiple threads trying to remove the same nodes
        for _ in 0..num_removers {
            let nodes_clone = Arc::clone(&nodes);
            let q = Arc::clone(&queue);
            let counter = Arc::clone(&successful_removals);

            let handle = thread::spawn(move || {
                let mut local_removed = 0;
                for node in nodes_clone.iter() {
                    if q.try_remove(node.as_ref()).is_ok() {
                        local_removed += 1;
                    }
                }
                counter.fetch_add(local_removed, Ordering::SeqCst);
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Each node should be removed exactly once
        assert_eq!(successful_removals.load(Ordering::SeqCst), num_nodes);

        // All nodes should have null next pointers
        for node in nodes.iter() {
            assert!(node.next_node.load(Ordering::Acquire).is_null());
        }
    }

    #[cfg(all(usermode, not(miri)))]
    #[test]
    fn test_concurrent_insert_remove_stress() {
        use std::sync::Arc;
        use std::sync::atomic::AtomicBool;
        use std::thread;
        use std::time::Duration;

        let queue_storage = Box::pin(MaybeUninit::<WaitQueue>::uninit());
        let queue_storage = unsafe { Box::leak(Pin::into_inner_unchecked(queue_storage)) };
        let queue = unsafe { WaitQueue::new_in(Pin::new_unchecked(queue_storage)) };
        let queue = Arc::new(queue.into_ref());

        let duration = Duration::from_millis(100);
        let stop = Arc::new(AtomicBool::new(false));

        // Pool of nodes that will be reused
        let node_pool: Arc<Vec<_>> = Arc::new(
            (0..50)
                .map(|_| Box::pin(WakeNode::new(AtomicWaker::null())))
                .collect(),
        );

        let mut handles = vec![];

        // Spawning inserter threads
        for _ in 0..3 {
            let nodes = Arc::clone(&node_pool);
            let q = Arc::clone(&queue);
            let stop_flag = Arc::clone(&stop);

            let handle = thread::spawn(move || {
                let mut inserted = 0;
                while !stop_flag.load(Ordering::Acquire) {
                    for node in nodes.iter() {
                        if node.next_node.load(Ordering::Acquire).is_null() {
                            unsafe {
                                if q.try_insert(node.as_ref()).is_ok() {
                                    inserted += 1;
                                }
                            }
                        }
                        thread::yield_now();
                    }
                }
                inserted
            });
            handles.push(handle);
        }

        // Spawn remover threads
        for _ in 0..3 {
            let nodes = Arc::clone(&node_pool);
            let q = Arc::clone(&queue);
            let stop_flag = Arc::clone(&stop);

            let handle = thread::spawn(move || {
                let mut removed = 0;
                while !stop_flag.load(Ordering::Acquire) {
                    for node in nodes.iter() {
                        if !node.next_node.load(Ordering::Acquire).is_null() {
                            if q.try_remove(node.as_ref()).is_ok() {
                                removed += 1;
                            }
                        }
                        thread::yield_now();
                    }
                }
                removed
            });
            handles.push(handle);
        }

        // Let the stress test run
        thread::sleep(duration);
        stop.store(true, Ordering::Release);

        // Collect results
        let mut total_inserted = 0;
        let mut total_removed = 0;

        for (i, handle) in handles.into_iter().enumerate() {
            let count = handle.join().unwrap();
            if i < 3 {
                total_inserted += count;
            } else {
                total_removed += count;
            }
        }

        // Should have done significant work
        assert!(total_inserted > 0);
        assert!(total_removed > 0);
    }

    #[cfg(all(usermode, not(miri)))]
    #[test]
    fn test_concurrent_mixed_operations() {
        use std::sync::Arc;
        use std::sync::atomic::AtomicUsize;
        use std::thread;

        let queue_storage = Box::pin(MaybeUninit::<WaitQueue>::uninit());
        let queue_storage = unsafe { Box::leak(Pin::into_inner_unchecked(queue_storage)) };
        let queue = unsafe { WaitQueue::new_in(Pin::new_unchecked(queue_storage)) };
        let queue = Arc::new(queue.into_ref());

        let nodes_per_thread = 15;
        let num_threads = 4;

        let successful_ops = Arc::new(AtomicUsize::new(0));
        let failed_ops = Arc::new(AtomicUsize::new(0));

        let mut handles = vec![];

        for _ in 0..num_threads {
            let q = Arc::clone(&queue);
            let success_counter = Arc::clone(&successful_ops);
            let fail_counter = Arc::clone(&failed_ops);

            let handle = thread::spawn(move || {
                let nodes: Vec<_> = (0..nodes_per_thread)
                    .map(|_| Box::pin(WakeNode::new(AtomicWaker::null())))
                    .collect();

                // Insert all
                for node in &nodes {
                    unsafe {
                        match q.try_insert(node.as_ref()) {
                            Ok(_) => {
                                success_counter.fetch_add(1, Ordering::SeqCst);
                            }
                            Err(_) => {
                                fail_counter.fetch_add(1, Ordering::SeqCst);
                            }
                        }
                    }
                }

                // Remove all
                for node in &nodes {
                    match q.try_remove(node.as_ref()) {
                        Ok(_) => {
                            success_counter.fetch_add(1, Ordering::SeqCst);
                        }
                        Err(_) => {
                            fail_counter.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Each thread does 2 * nodes_per_thread operations (insert + remove)
        let expected_ops = num_threads * nodes_per_thread * 2;
        let total_ops = successful_ops.load(Ordering::SeqCst) + failed_ops.load(Ordering::SeqCst);

        assert_eq!(total_ops, expected_ops);
        // Most operations should succeed
        assert!(successful_ops.load(Ordering::SeqCst) > expected_ops / 2);
    }
}

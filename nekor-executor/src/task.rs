//! Task data structure for the executor.
//!
//! Tasks are the basic schedulable unit in the executor. They are cooperative
//! and cannot be preempted mid-task.
//!
//! A task is assumed to always be [`pinned`] in-place and to guarantee a static
//! lifetime.
//!
//! Notably, tasks can be also be part of a larger task pool of the same type of
//! task, which can allocate and manage them efficiently on-the-fly.
//!
//! [`pinned`]: core::pin::Pin

use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    ptr::NonNull,
    task::Waker,
};

use nekor_structure::arena::Arena;

use nekor_domain::prelude::Erased;
use nekor_sync::atomic::bitmap::typeutil::{BitMapUsize, InBound};

use crate::{
    run_queue::TaskQueue,
    task::{
        raw::RawTask,
        state::{StateDescriptor, TaskStatus},
    },
    wake::{self, list::WakeNode},
};

pub mod state;

pub mod raw;

// TODO: Okay, got Wakers thought out, we can have as many of them as we want,
// but since they have a single data pointer, they need to sit in an atomic
// single linked list in a waitqueue write a waitqueue tomorrow for that
// also abstract the Future into a representation easier to type erase
// maybe just make an header + union of the future and the output, and make a
// poll fn to it that takes an extra pointer to the output value slot
// we could ensure that the size of the future is at least the same size by
// wrapping it in a wrapper that forces the Future::Output to live across an
// await boundary. or..
//
// union Task<F> where F: Future {
//  future: F,
//  output: F::Output
// }

// for getting the output back, we can have a JoinHandle type which setups a
// pointer to the task and the task has a single pointer
// (Option<NonNull<MaybeUninit<F::Output>>>) to JoinHandle's storage.
// if JoinHandle is dropped, the ptr to the task is used to clear the
// joinhandler pending state, however, the task here is not cancelled, just
// detached from a joinhandle is also a future which is waked when the final
// task is done, which would have written the value, i.e, it acts as an explicit
// join

/// A single task for an executor to drive to completion.
#[derive(Debug)]
pub struct Task {
    /// The status of the task.
    ///
    /// This imposes an ownership invariant on the rest of the task, refer to
    /// [`TaskStatus`] for further information.
    ///
    /// Not largely contested, hence why not cacheline-isolated.
    task_status: TaskStatus,

    /// The runqueue native to this task.
    ///
    /// # Remarks
    ///
    /// This is not a [`Run`] because it requires no cache coherence between
    /// distinct atomic accesses, as the sharing is always direct.
    ///
    /// [`Run`]: super::run_queue::Run
    schedule_queue: Erased<TaskQueue>,

    /// The [`WakeNode`] the [`Task`] uses for runqueue backpressure.
    /// TODO: This is not related to Wakers at all. Move the whole `WakeQueue`
    /// to a generic list implementation.
    runqueue_wait_node: WakeNode,

    /// The raw task handle to the underlying [`Future`].
    raw_task: UnsafeCell<RawTask>,
}

impl Task {
    /// The runqueue that this task is scheduled on.
    #[inline]
    #[must_use]
    pub const fn queue(&self) -> &Erased<TaskQueue> {
        let Self { schedule_queue, .. } = self;

        schedule_queue
    }
}

impl Task {
    /// Attempt to acquire the target [`Task`] exclusively.
    ///
    /// ## Failure
    ///
    /// This will fail **iff**:
    ///
    /// - The target task is not in a dormant state, i.e., it is either pending
    ///   or executing.
    ///
    /// See the distinct task states in [`StateDescriptor`] for further
    /// information.
    #[inline]
    pub fn acquire(target_task: &'static Self) -> Option<Acquired> {
        let Self {
            task_status,
            raw_task,
            ..
        } = target_task;

        match TaskStatus::determine(task_status) {
            StateDescriptor::Dormant(target_state) => {
                if target_state.pending().is_some() {
                    let raw_ptr = raw_task.get();

                    // SAFETY: Cannot be null, as the pointer has been sourced
                    // from an `UnsafeCell`.
                    let mut raw_ptr = unsafe { NonNull::new_unchecked(raw_ptr) };

                    // SAFETY: The pointer is valid and can be used in a mutable
                    // context, as the task has been acquired exclusively.
                    let reference = unsafe { raw_ptr.as_mut() };

                    Some(Acquired(reference))
                } else {
                    None
                }
            }
            StateDescriptor::Pending(..) | StateDescriptor::Executing(..) => None,
        }
    }

    /// Construct a [`Waker`] associated to this task.
    #[inline]
    #[must_use]
    pub const fn waker(&'static self) -> Waker {
        // SAFETY: `Task` (and `Waker`-related code) is thread-safe, as required
        // by the `RawWakerVTable` documentation.
        unsafe {
            Waker::new(
                core::ptr::from_ref::<Self>(self).cast::<()>(),
                &wake::VTABLE,
            )
        }
    }
}

/// A handle to an exclusively-acquired [`RawTask`].
#[repr(transparent)]
pub struct Acquired(&'static mut RawTask);

impl Deref for Acquired {
    type Target = RawTask;

    #[inline]
    fn deref(&self) -> &Self::Target {
        let Self(target_value) = self;

        target_value
    }
}

impl DerefMut for Acquired {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        let &mut Self(ref mut target_value) = self;

        target_value
    }
}

// TODO: Add API to access the task safely.

// SAFETY: The thread-safe access to `Task` is managed by type invariants.
unsafe impl Sync for Task {}

/// A task pool of [`Future`]s of type `F` of size `N`.
///
/// This is the basic unit of work in the executor.
///
/// For a sole task, it is equivalent to a 1-sized [`Arena`].
#[repr(transparent)]
pub struct TaskPool<F, const N: usize>(Arena<F, N>)
where
    F: Future,
    BitMapUsize<N>: InBound;

// TODO: Add Event type to make it possible for Futures to register to be woken
// up

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
    ptr::{NonNull, from_ref},
    task::Waker,
};

use nekor_domain::prelude::Erased;
use nekor_structure::arena::Arena;
use nekor_sync::atomic::bitmap::typeutil::{BitMapUsize, InBound};

use crate::{
    run_queue::TaskQueue,
    task::{
        raw::RawTask,
        state::{StateDescriptor, TaskStatus},
    },
    wake,
};

pub mod state;

pub mod raw;

/// A single task for an executor to drive to completion.
#[derive(Debug)]
#[repr(align(8))]
// NOTE(invariant): Task addresses reserve three low tag bits through eight-byte alignment and
// remain statically allocated and pinned after construction.
pub struct Task {
    /// The status of the task.
    ///
    /// This imposes an ownership invariant on the rest of the task, refer to
    /// [`TaskStatus`] for further information.
    ///
    /// Not largely contested, hence why not cacheline-isolated.
    task_state: TaskStatus,

    /// The runqueue native to this task.
    ///
    /// # Remarks
    ///
    /// This is not a [`Run`] because it requires no cache coherence between
    /// distinct atomic accesses, as the sharing is always direct.
    ///
    /// [`Run`]: super::run_queue::Run
    schedule_queue: Erased<TaskQueue>,

    /// The raw task handle to the underlying [`Future`].
    task_handle: UnsafeCell<RawTask>,
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
    /// - The target task is not in a dormant state, i.e., it is either pending or executing.
    ///
    /// See the distinct task states in [`StateDescriptor`] for further
    /// information.
    #[inline]
    pub fn acquire(target_task: &'static Self) -> Option<Acquired> {
        let Self {
            task_state,
            task_handle,
            ..
        } = target_task;

        match TaskStatus::determine(task_state) {
            StateDescriptor::Dormant(target_state) => {
                target_state.pending().is_some().then(|| {
                    // SAFETY: Cannot be null, as the pointer has been sourced
                    // from an `UnsafeCell`.
                    let mut task_handle = unsafe { NonNull::new_unchecked(UnsafeCell::get(task_handle)) };

                    // SAFETY: The pointer is valid and can be used in a mutable
                    // context, as the task has been acquired exclusively.
                    let task_handle = unsafe { task_handle.as_mut() };

                    Acquired(task_handle)
                })
            },
            StateDescriptor::Pending(..) | StateDescriptor::Executing(..) => None,
        }
    }

    /// Construct a [`Waker`] associated to this task.
    #[inline]
    #[must_use]
    pub const fn waker(&'static self) -> Waker {
        // SAFETY: `Task` (and `Waker`-related code) is thread-safe, as required
        // by the `RawWakerVTable` documentation.
        unsafe { Waker::new(from_ref::<Self>(self).cast::<()>(), &wake::VTABLE) }
    }
}

/// A handle to an exclusively-acquired [`RawTask`].
#[repr(transparent)]
// NOTE(invariant): The static raw task is exclusively borrowed while this handle is held.
pub struct Acquired(&'static mut RawTask);

impl Deref for Acquired {
    type Target = RawTask;

    #[inline]
    fn deref(&self) -> &Self::Target {
        let Self(target_task) = self;

        target_task
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
// NOTE(invariant): The arena retains at most `N` future slots with their reservation state.
pub struct TaskPool<F, const N: usize>(Arena<F, N>)
where
    F: Future,
    BitMapUsize<N>: InBound;

// TODO: Add Event type to make it possible for Futures to register to be woken
// up

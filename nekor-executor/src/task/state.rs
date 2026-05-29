//! Concurrent [`task state`] management.
//!
//! This module exposes a series of new-type items that provide an interface for
//! sound [`task state`] transitions.
//!
//! [`task state`]: TaskState

use core::sync::atomic::{AtomicUsize, Ordering};

/// The state of a particular [`Task`] in the executor.
///
/// [`Task`]: super::Task
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
#[repr(usize)]
pub enum TaskState {
    /// The task is dormant, and has no recognised owner.
    ///
    /// NOTE(invariant): No mutation of internal task state may be made previous
    /// to a transition from a dormant state.
    Dormant,

    /// The task has been claimed by an executor thread, but has not been polled
    /// yet.
    ///
    /// NOTE(invariant): This imposes the existence of a sole owner of the
    /// pending task.
    Pending,

    /// The task has been claimed and is currently being executed by an executor
    /// thread.
    ///
    /// NOTE(invariant): Mirrors the [`TaskState::Pending`] state.
    Executing,
}

/// A task said to be in a dormant state.
///
/// This is a *snapshot* type, used to guide progress towards another state.
///
/// Refer to the dedicated [`documentation`] for this state for further
/// information.
///
/// [`documentation`]: TaskState::Dormant
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct StateDormant<'a>(
    // NOTE(invariant): This assumes an `Acquire` load previous to any state
    // transition.
    &'a AtomicUsize,
);

impl<'a> StateDormant<'a> {
    /// Attempt to transition back into a *pending* state.
    ///
    /// # Failure
    ///
    /// This will only fail if the current *snapshot* is stale and does not
    /// reflect the current task status.
    #[inline]
    #[must_use]
    pub fn pending(&self) -> Option<StatePending<'a>> {
        let &Self(target_state) = self;

        target_state
            .compare_exchange(
                TaskState::Dormant as usize,
                TaskState::Pending as usize,
                // NOTE(invariant): This depends on the aforementioned
                // invariant to publish the new task state.
                Ordering::AcqRel,
                Ordering::Relaxed,
            )
            .ok()
            .map(|_| StatePending(target_state))
    }

    /// Attempt to transition back into a *executing* state.
    ///
    /// # Failure
    ///
    /// This will only fail if the current *snapshot* is stale and does not
    /// reflect the current task status.
    #[inline]
    #[must_use]
    pub fn executing(&self) -> Option<StateExecuting<'a>> {
        let &Self(target_state) = self;

        target_state
            .compare_exchange(
                TaskState::Dormant as usize,
                TaskState::Executing as usize,
                // NOTE(invariant): This depends on the aforementioned
                // invariant to publish the new task state.
                Ordering::AcqRel,
                Ordering::Relaxed,
            )
            .ok()
            .map(|_| StateExecuting(target_state))
    }
}

/// A task said to be in a pending state.
///
/// This is a *snapshot* type, used to guide progress towards another state.
///
/// Refer to the dedicated [`documentation`] for this state for further
/// information.
///
/// [`documentation`]: TaskState::Pending
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct StatePending<'a>(
    // NOTE(invariant): This assumes an `Acquire` load previous to any state
    // transition.
    &'a AtomicUsize,
);

impl<'a> StatePending<'a> {
    /// Attempt to transition back into a *pending* state.
    ///
    /// # Failure
    ///
    /// This will only fail if the current *snapshot* is stale and does not
    /// reflect the current task status.
    #[inline]
    #[must_use]
    pub fn executing(&self) -> Option<StateExecuting<'a>> {
        let &Self(target_state) = self;

        target_state
            .compare_exchange(
                TaskState::Pending as usize,
                TaskState::Executing as usize,
                // NOTE(invariant): This depends on the aforementioned
                // invariant to publish the new task state.
                Ordering::AcqRel,
                Ordering::Relaxed,
            )
            .ok()
            .map(|_| StateExecuting(target_state))
    }

    /// Attempt to transition back into a *dormant* state.
    ///
    /// # Failure
    ///
    /// This will only fail if the current *snapshot* is stale and does not
    /// reflect the current task status.
    ///
    /// # Safety
    ///
    /// The [`Task`] associated to this state must not be accessed under any
    /// circumstance after a successful call has been effectuated.
    #[inline]
    #[must_use]
    pub unsafe fn dormant(&self) -> Option<StateDormant<'a>> {
        let &Self(target_state) = self;

        target_state
            .compare_exchange(
                TaskState::Pending as usize,
                TaskState::Dormant as usize,
                // NOTE(invariant): This depends on the aforementioned
                // invariant to publish the new task state.
                Ordering::AcqRel,
                Ordering::Relaxed,
            )
            .ok()
            .map(|_| StateDormant(target_state))
    }
}

/// A task said to be in an executing state.
///
/// This is a *snapshot* type, used to guide progress towards another state.
///
/// Refer to the dedicated [`documentation`] for this state for further
/// information.
///
/// [`documentation`]: TaskState::Executing
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct StateExecuting<'a>(
    // NOTE(invariant): This assumes an `Acquire` load previous to any state
    // transition.
    &'a AtomicUsize,
);

impl<'a> StateExecuting<'a> {
    /// Attempt to transition back into a *pending* state.
    ///
    /// # Failure
    ///
    /// This will only fail if the current *snapshot* is stale and does not
    /// reflect the current task status.
    #[inline]
    #[must_use]
    pub fn pending(&self) -> Option<StatePending<'a>> {
        let &Self(target_state) = self;

        target_state
            .compare_exchange(
                TaskState::Executing as usize,
                TaskState::Pending as usize,
                // NOTE(invariant): This depends on the aforementioned
                // invariant to publish the new task state.
                Ordering::AcqRel,
                Ordering::Relaxed,
            )
            .ok()
            .map(|_| StatePending(target_state))
    }

    /// Attempt to transition back into a *dormant* state.
    ///
    /// # Failure
    ///
    /// This will only fail if the current *snapshot* is stale and does not
    /// reflect the current task status.
    ///
    /// # Safety
    ///
    /// The [`Task`] associated to this state must not be accessed under any
    /// circumstance after a successful call has been effectuated.
    #[inline]
    #[must_use]
    pub unsafe fn dormant(&self) -> Option<StateDormant<'a>> {
        let &Self(target_state) = self;

        target_state
            .compare_exchange(
                TaskState::Executing as usize,
                TaskState::Dormant as usize,
                // NOTE(invariant): This depends on the aforementioned
                // invariant to publish the new task state.
                Ordering::AcqRel,
                Ordering::Relaxed,
            )
            .ok()
            .map(|_| StateDormant(target_state))
    }
}

/// An enumeration of all valid state machine transitions.
///
/// This serves as an unified interface to lock in to a specific task.
#[derive(Debug, Clone, Copy)]

pub enum StateDescriptor<'a> {
    /// A dormant state.
    ///
    /// See [`StateDormant`] for further information.
    Dormant(StateDormant<'a>),

    /// A pending state.
    ///
    /// See [`StatePending`] or further information.
    Pending(StatePending<'a>),

    /// An executing state.
    ///
    /// See [`StateExecuting`] for further information.
    Executing(StateExecuting<'a>),
}

/// The status of a particular [`Task`] in the executor.
///
/// This is a thread-safe wrapper over [`TaskState`].
#[derive(Debug)]
#[repr(transparent)]
pub struct TaskStatus(AtomicUsize);

impl TaskStatus {
    /// Determine the status of the associated task.
    #[inline]
    pub fn determine(Self(target_state): &Self) -> StateDescriptor<'_> {
        // NOTE(invariant): This satisfies the `Acquire` invariant inside the
        // distinct `State{Dormant,Pending,Executing}` types.
        let target_value = target_state.load(Ordering::Acquire);

        const DORMANT_STATE: usize = TaskState::Dormant as usize;
        const PENDING_STATE: usize = TaskState::Pending as usize;
        const EXECUTING_STATE: usize = TaskState::Executing as usize;

        match target_value {
            DORMANT_STATE => StateDescriptor::Dormant(StateDormant(target_state)),
            PENDING_STATE => StateDescriptor::Pending(StatePending(target_state)),
            EXECUTING_STATE => StateDescriptor::Executing(StateExecuting(target_state)),
            _ => unreachable!(),
        }
    }
}

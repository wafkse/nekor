//! The top-level scheduler type, used to group scheduling-related
//! functionality.

use nekor_structure::queue::BusyOrFull;

use crate::{
    run_queue::Run,
    task::{Acquired, Task},
};

/// The top-level scheduler type, used to group scheduling-related
/// functionality.
pub enum Scheduler {}

impl Scheduler {
    /// Scan queues in priority order and try to claim the first dequeued task.
    ///
    /// Returns `None` if no queue yields a task or if that first task is already
    /// claimed. A failed claim ends this cycle without scanning later queues.
    #[inline]
    #[must_use]
    pub fn cycle() -> Option<Acquired> {
        for target_queue in Run::list() {
            if let Some(target_task) = target_queue.dequeue() {
                return Task::acquire(target_task);
            }
        }

        None
    }

    /// Attempt to schedule the target [`Task`].
    ///
    /// # Errors
    ///
    /// Returns the task when the queue is full or its current reservation cycle
    /// prevents an enqueue. The task is not kept pending by this method.
    #[inline]
    pub fn try_schedule(target_task: &'static Task) -> Result<(), &'static Task> {
        target_task.queue().enqueue(target_task).map_err(BusyOrFull::unwrap)
    }
}

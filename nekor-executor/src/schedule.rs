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
    /// Determine the target [`Task`] to be scheduled next.
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
    /// This will fail if the runqueue is full.
    #[inline]
    pub fn try_schedule(target_task: &'static Task) -> Result<(), &'static Task> {
        target_task
            .queue()
            .enqueue(target_task)
            .map_err(BusyOrFull::unwrap)
    }
}

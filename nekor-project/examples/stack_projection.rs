//! Project pinned fields without a heap allocation.

use core::{marker::PhantomPinned, pin::Pin};

use nekor_project::Project;

/// A task whose state must stay at one address after pinning.
#[derive(Project)]
struct Task {
    /// State whose address is part of the task's pinning contract.
    #[project(pin)]
    state: PhantomPinned,

    /// Ordinary data that remains mutable through projection.
    priority: usize,
}

/// Demonstrate immutable and mutable projections from stack-pinned storage.
fn main() {
    let task = Task {
        state: PhantomPinned,
        priority: 4,
    };
    let mut task = core::pin::pin!(task);

    let TaskProjection { state, priority } = task.as_ref().project();
    let _: Pin<&PhantomPinned> = state;
    assert_eq!(*priority, 4, "immutable projection reads ordinary data");

    let TaskProjectionMut { state, priority } = task.as_mut().project_mut();
    let _: Pin<&mut PhantomPinned> = state;
    *priority = 7;

    let TaskProjection { priority, .. } = task.as_ref().project();
    assert_eq!(*priority, 7, "mutable projection updates ordinary data");
}

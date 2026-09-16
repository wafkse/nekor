//! Project an enum while preserving its active variant.

use core::{marker::PhantomPinned, pin::Pin};

use nekor_project::Project;

/// A small execution state with named, tuple, and unit variants.
#[derive(Project)]
enum State {
    /// An active state with structurally pinned storage.
    Active {
        /// State that must remain at its pinned address.
        #[project(pin)]
        marker: PhantomPinned,

        /// Ordinary generation metadata.
        generation: u32,
    },

    /// A waiting state with a pinned marker and ordinary metadata.
    Waiting(#[project(pin)] PhantomPinned, u32),

    /// A completed state without stored fields.
    Complete,
}

/// Read the active variant through a borrowing projection.
fn main() {
    let mut state = core::pin::pin!(State::Active {
        marker: PhantomPinned,
        generation: 3,
    });

    match state.as_mut().project_mut() {
        StateProjectionMut::Active { marker, generation } => {
            let _: Pin<&mut PhantomPinned> = marker;
            *generation = 4;
        },
        StateProjectionMut::Waiting(..) | StateProjectionMut::Complete => {
            unreachable!("the constructed state is active")
        },
    }

    match state.as_ref().project() {
        StateProjection::Active { marker, generation } => {
            let _: Pin<&PhantomPinned> = marker;
            assert_eq!(*generation, 4, "projection retains the active variant");
        },
        StateProjection::Waiting(..) | StateProjection::Complete => {
            unreachable!("projection cannot replace the active variant")
        },
    }

    let waiting = core::pin::pin!(State::Waiting(PhantomPinned, 5));
    match waiting.as_ref().project() {
        StateProjection::Waiting(marker, generation) => {
            let _: Pin<&PhantomPinned> = marker;
            assert_eq!(*generation, 5, "tuple fields preserve their order");
        },
        StateProjection::Active { .. } | StateProjection::Complete => {
            unreachable!("the constructed state is waiting")
        },
    }

    let complete = core::pin::pin!(State::Complete);
    assert!(
        matches!(complete.as_ref().project(), StateProjection::Complete),
        "unit variants project without fields"
    );
}

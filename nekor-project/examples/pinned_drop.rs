//! Forward ordinary destruction to a pin-aware destructor hook.

use core::{marker::PhantomPinned, pin::Pin};

use nekor_project::{PinnedDrop, Project};

/// A stack-pinned resource with a borrowed close indication.
#[derive(Project)]
#[project(unsafe = Drop)]
struct Resource<'a> {
    /// Structurally pinned state that destruction must not move.
    #[project(pin)]
    state: PhantomPinned,

    /// Borrowed observation updated by the pin-aware hook.
    closed: &'a mut bool,
}

// SAFETY: The ordinary Drop implementation pins this same value and forwards exactly
// once. The hook projects fields without moving pinned state.
unsafe impl PinnedDrop for Resource<'_> {
    /// Mark the resource closed without moving its pinned state.
    ///
    /// # Safety
    ///
    /// This hook must be called exactly once by `Drop` for the same pinned
    /// value before normal field destruction.
    unsafe fn drop(self: Pin<&mut Self>) {
        let ResourceProjectionMut { state, closed } = self.project_mut();
        let _: Pin<&mut PhantomPinned> = state;
        **closed = true;
    }
}

impl Drop for Resource<'_> {
    /// Forward destructor work to the pin-aware hook exactly once.
    fn drop(&mut self) {
        // SAFETY: This destructor does not move the value or its pinned field. The
        // same value stays in place through the forwarding call.
        let pinned = unsafe { Pin::new_unchecked(self) };

        // SAFETY: Drop calls this hook exactly once for the same pinned value and
        // does not access the value after forwarding.
        unsafe { <Self as PinnedDrop>::drop(pinned) };
    }
}

/// Exercise the drop hook with stack storage and no allocation.
fn main() {
    let mut closed = false;

    {
        let _resource = core::pin::pin!(Resource {
            state: PhantomPinned,
            closed: &mut closed,
        });
    }

    assert!(closed, "the pin-aware hook runs during destruction");
}

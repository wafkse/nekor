#![cfg_attr(not(any(test, miri)), no_std)]
#![forbid(
    clippy::all,
    clippy::perf,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::panic,
    clippy::pedantic,
    rustdoc::all
)]

use core::pin::Pin;

/// A trait that describes a potential [`Pin`]-projection.
///
/// # Safety
pub unsafe trait Project {
    /// The projection structure for the type.
    type Projection<'a>
    where
        Self: 'a;

    /// The mutable projection structure for the type.
    type ProjectionMut<'a>
    where
        Self: 'a;

    /// Project the type in an immutable manner into a [`Project::Projection`]
    /// type.
    fn project<'a>(self: Pin<&'a Self>) -> Self::Projection<'a>;

    /// Project the type in a mutable manner into a
    /// [`Project::ProjectionMut`] type.
    fn project_mut<'a>(self: Pin<&'a mut Self>) -> Self::ProjectionMut<'a>;
}

/// A *pinning* [`Drop`] alternative for [`Pin`]-projected types via
/// [`Project`].
///
/// # Safety
///
/// * The type must implement [`Drop`] (this is not a trait bound on `Self` to
/// avoid circularity).
/// * The [`Drop`] implementation of the type must forward to the
///   [`PinnedDrop::drop`] associated function of this trait.
pub unsafe trait PinnedDrop: Project {
    /// [`Drop`] the type whose fields can be structurally-pinned.
    fn drop(self: Pin<&mut Self>);
}

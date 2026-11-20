//! Type-erased access to statically stored futures.
//!
//! [`Schedulable`] futures return [`Finalize`] and satisfy the domain's [`Store`] bounds.
//! [`RawTask`] stores their trait object, while [`Erased`] and [`ErasedMut`] borrow it under
//! the enclosing task's ownership protocol. Application results need separate storage or
//! a communication channel because the executor consumes the completion status.

use core::{
    fmt,
    ops::{Deref, DerefMut},
};

use nekor_domain::prelude::Store;

/// The result of a top-level [`Future`] completion.
///
/// This is primarily used to determine whether a top-level [`Future`] has
/// either succeeded or failed.
#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Debug)]
pub enum Finalize {
    /// A successful completion.
    Success,

    /// A failed completion.
    Failure,
}

/// A trait that encompasses the requirement baseline for a [`Future`] to be
/// schedulable by the asynchronous executor.
///
/// # Notes
///
/// This trait is automatically implemented for an eligible type.
pub trait Schedulable: Future<Output = Finalize> + Store {}

/// Blanket implementation for all schedulable types.
impl<F> Schedulable for F where F: Future<Output = Finalize> + Store + ?Sized {}

/// An immutable access handle provided by [`RawTask`] to a [`Schedulable`]
/// trait object.
#[derive(Copy, Clone)]
#[repr(transparent)]
// NOTE(invariant): The borrowed trait object remains valid for the handle lifetime.
pub struct Erased<'a>(&'a dyn Schedulable);

impl Deref for Erased<'_> {
    type Target = dyn Schedulable;

    #[inline]
    fn deref(&self) -> &Self::Target {
        let Self(target_task) = self;

        *target_task
    }
}

/// An access handle provided by [`RawTask`].
#[repr(transparent)]
// NOTE(invariant): The exclusive trait-object borrow remains valid for the handle lifetime.
pub struct ErasedMut<'a>(&'a mut dyn Schedulable);

impl Deref for ErasedMut<'_> {
    type Target = dyn Schedulable;

    #[inline]
    fn deref(&self) -> &Self::Target {
        let Self(target_task) = self;

        &**target_task
    }
}

impl DerefMut for ErasedMut<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        let &mut Self(ref mut target_task) = self;

        &mut **target_task
    }
}

/// A handle to a raw task.
#[repr(transparent)]
// NOTE(invariant): The stored schedulable task has static storage and is accessed under task
// ownership.
pub struct RawTask(&'static mut dyn Schedulable);

impl RawTask {
    /// Access the underlying [`Schedulable`] trait object in an immutable
    /// manner.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that the upper-level [`Task`] is owned by the
    /// current thread of execution.
    ///
    /// [`Task`]: super::Task
    #[inline]
    #[must_use]
    pub const unsafe fn access(&self) -> Erased<'_> {
        let Self(target_task) = self;

        Erased(&**target_task)
    }

    /// Access the underlying [`Schedulable`] trait object in an mutable manner.
    ///
    /// # Safety
    ///
    /// This associated function has the same ownership requirements as
    /// [`RawTask::access`].
    #[inline]
    pub const unsafe fn access_mut(&mut self) -> ErasedMut<'_> {
        let &mut Self(ref mut target_task) = self;

        ErasedMut(&mut **target_task)
    }
}

impl fmt::Debug for RawTask {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("RawTask").finish_non_exhaustive()
    }
}

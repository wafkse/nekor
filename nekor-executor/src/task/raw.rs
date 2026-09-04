//! Generalized behaviour for [`Future`] types.
//!
//! This is the standard way to operate on [`Future`]s of any type.
//!
//! This does type-erasure in two stages:
//!
//! - By requiring an unit (`()`) [`output type`] for a target [`Future`], `'static` lifetime, and
//!   [`Sync`]-ness.
//! - By wrapping it inside a [`RawTask`], and providing an accessor type: [`Erased`] and
//!   [`ErasedMut`].
//!
//! Note that even if arbitrary [`output type`]s are unsupported from the
//! executor level, value-producing [`Future`]s can be simulated through the use
//! of an asynchronous oneshot channel.
//!
//! [`output type`]: Future::Output

use core::{
    fmt,
    ops::{Deref, DerefMut},
};

use nekor_domain::prelude::Store;

/// The result of a top-level [`Future`] completion.
///
/// This is primarily used to determine whether a top-level [`Future`] has
/// either succeded or failed.
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
/// # Remarks
///
/// This trait is automatically implemented for an elegible type.
pub trait Schedulable: Future<Output = Finalize> + Store {}

/// Blanket implementation for all schedulable types.
impl<F> Schedulable for F where F: Future<Output = Finalize> + Store + ?Sized {}

/// An immutable access handle provided by [`RawTask`] to a [`Schedulable`]
/// trait object.
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct Erased<'a>(&'a dyn Schedulable);

impl Deref for Erased<'_> {
    type Target = dyn Schedulable;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

/// An access handle provided by [`RawTask`].
#[repr(transparent)]
pub struct ErasedMut<'a>(&'a mut dyn Schedulable);

impl Deref for ErasedMut<'_> {
    type Target = dyn Schedulable;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl DerefMut for ErasedMut<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

/// A handle to a raw task.
#[repr(transparent)]
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
        Erased(&*self.0)
    }

    /// Access the underlying [`Schedulable`] trait object in an mutable manner.
    ///
    /// # Safety
    ///
    /// This associated function has the same safety compromises as
    /// [`RawTask::access`].
    #[inline]
    pub const unsafe fn access_mut(&mut self) -> ErasedMut<'_> {
        ErasedMut(&mut *self.0)
    }
}

impl fmt::Debug for RawTask {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("RawTask").finish_non_exhaustive()
    }
}

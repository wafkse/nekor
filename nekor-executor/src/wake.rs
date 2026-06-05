//! Wakeup behavior for tasks inside the executor.
//!
//! # Terminology
//!
//! - *wake source*: an external wakeup trigger, which has access to *one or
//!   more* [`Waker`] instances of the same [`Future`].
//! - *wake sink*: the target [`Future`] that a wakeup trigger affects.
//! - *sleeper*: a [`Future`] who has *at least one* [`Waker`] directly
//!   accessible to an active *wake source*.
//! - *waker*: a driving entity for actuating *wake source*s
//!
//! # Executor Assumptions
//!
//! The executor assumes that a [`Future`]:
//!
//! - May be waiting for potentially `N` distinct wake sources.
//!
//! The executor assumes that a *waker*:
//!
//! - May potentially wake up `M` distinct *sleeper*s.
//!
//! Furthermore, the executor guarantees that each [`Waker`] actuation will
//! result in a respective [`Future::poll`] invocation.
//!
//! ## Waker Structure
//!
//! A [`Waker`] to the executor is a simple pointer to a `'static`,
//! intrinsically-pinned [`Task`].
//!
//! Because of this, there is no waker-related:
//!
//! - External memory allocation, all [`Waker`] instances are completely
//!   self-contained.
//! - Reference counting, as the [`Task`] is statically allocated and thus does
//!   not require deallocation management.
//! - Distinction, as all [`Waker`] instances to the same [`Task`] are
//!   guaranteed to be identical.
//!
//! ### Wakeup Behaviour
//!
//! Wake-ups are performed as a single constant-time complexity (otherwise
//! `O(1)`) per-sleeper operation. En masse wake-ups are assumed to have
//! linear-time (`O(n)`) complexity.
//!
//! # A Multi-Sleeper, Multi-Waker Model
//!
//! To achieve a seamless async executor model, one must allow a particular
//! [`Future`] to:
//!
//! - Await a selection of distinct naturally asynchronous conditions ([^1])
//!   (and possibly hardware-dependant).
//!
//! Finally, one must also permit a *wake source* to recollect an arbitrary
//! number of *sleeper* tasks.
//!
//! ## A Multi-Sleeper List
//!
//! With these basic goals in mind, one way we introduce a multi-sleeper model
//! is through the use of an atomic (hence lock-free) singly-linked circular
//! linked list.
//!
//! This linked list will be embedded inside each (hand-rolled) [`Future`] (and,
//! of course, stored as part of a "wake me up later" operation).
//!
//! This model is implemented as the [`WaitQueue`] structure.
//! [`WaitQueue`] allow a single [`Future`] to register a [`Waker`] collectively
//! with other [`Future`]s in the list.
//!
//! ## Concurrency Management
//!
//! [`WaitQueue`] permits simultaneous lock-free operation between multiple
//! logical threads of execution.
//!
//! Furthermore, the linked list has no risk of suffering from the *ABA
//! Problem*, as:
//!
//! 1. Every [`Task`] is valid for `'static` and pinned in-place.
//! 2. [`Task`] storage is re-used between different tasks of the same type.
//!
//! ## Time Complexity
//!
//! Due to the excellent time complexity of singly-linked circular linked lists,
//! wake-ups and other [`Waker`]-related operations are quite fast.
//!
//! Particularly, this allows:
//!
//! - Constant-time (`O(1)`) [`Future`] wake-up scheduling.
//! - Linear-time (`O(n)`) [`Future`] wait-queue retirement.
//!
//! [^1]: Also known as a fan-out asynchronous model.
//!
//! [`WaitQueue`]: list::WaitQueue

pub mod list;

pub mod list2;

use core::{
    ptr::{self, NonNull},
    sync::atomic::{AtomicPtr, Ordering},
    task::{RawWaker, RawWakerVTable, Waker},
};

use crate::{schedule::Scheduler, task::Task};

/// The virtual function table that is associated to all [`Waker`] instances.
pub static VTABLE: RawWakerVTable = const {
    /// Clone a [`Waker`].
    ///
    /// # Safety
    ///
    /// This assumes that the data pointer has been sourced from a `&'static
    /// Task` reference.
    unsafe fn clone(data: *const ()) -> RawWaker {
        // NOTE: Tasks are always `'static`, no need to handle a reference
        // count.
        RawWaker::new(data, &self::VTABLE)
    }

    /// Wake a [`Task`] by value, effectively consuming it.
    ///
    /// # Safety
    ///
    /// This assumes that the data pointer has been sourced from a `&'static
    /// Task` reference.
    unsafe fn wake(task: *const ()) {
        // SAFETY: Assumed to be a valid `&'static Task`.
        let task: &'static Task =
            unsafe { NonNull::<Task>::new_unchecked(task as *mut _).as_ref() };

        // TODO: Use backpressure here.

        let _ = Scheduler::try_schedule(task);
    }

    /// Properly dispose of a [`Waker`].
    ///
    /// This does nothing, as no dropping logic is required.
    const fn drop(_: *const ()) {
        /* no drop logic required */
    }

    RawWakerVTable::new(clone, wake, wake, drop)
};

/// A [`Waker`] that can be managed in an atomic manner.
///
/// # Remarks
///
/// This requires that any provided [`Waker`] is local to the Nekor executor
/// (i.e., must have the [`default table`] as its own virtual function table).
///
/// Technically, a data pointer and a virtual function table could be atomically
/// stored using a 16-byte atomic compare-and-swap (*DCAS*) instruction (such as
/// `xcmpchg16b` on `x86{,-64}`), but such usage would drastically impact the
/// range of devices that the kernel could support.
///
/// [`default table`]: self::VTABLE
///
/// # Drop
///
/// Since the Nekor executor [`Waker`] virtual function table has a no-op
/// [`Drop`] implementation, this type has no [`Drop`]-related functionality.
#[repr(transparent)]
#[derive(Debug)]
pub struct AtomicWaker(
    // NOTE(invariant): This must be the data pointer of a `Waker` with a
    // Nekor-specific virtual function table.
    AtomicPtr<()>,
);

impl AtomicWaker {
    /// Attempt to instantiate a new [`AtomicWaker`] for the target [`Waker`].
    ///
    /// # Failure
    ///
    /// Fails with [`None`] if the provided [`Waker`] virtual function table
    /// does not match the [`default table`].
    ///
    /// [`default table`]: self::VTABLE
    #[inline]
    #[must_use]
    pub fn new(waker: &Waker) -> Option<Self> {
        if ptr::eq(waker.vtable(), &raw const self::VTABLE) {
            Some(Self(AtomicPtr::new(waker.data().cast_mut())))
        } else {
            None
        }
    }

    /// Instantiate a new [`AtomicWaker`] for the target [`Waker`], without
    /// checking for its virtual function table.
    ///
    /// # Safety
    ///
    /// The target [`Waker`] must be local to the Nekor executor (i.e., must
    /// have the [`default table`] as its own virtual function table).
    ///
    /// [`default table`]: self::VTABLE
    #[inline]
    #[must_use]
    pub unsafe fn new_unchecked(waker: &Waker) -> Self {
        Self(AtomicPtr::new(waker.data().cast_mut()))
    }

    /// Instantiate a new [`AtomicWaker`] that is initialized to *a null value*.
    #[inline]
    #[must_use]
    pub const fn null() -> Self {
        Self(AtomicPtr::new(ptr::null_mut()))
    }

    /// Materialize the [`Waker`] from the [`AtomicWaker`].
    ///
    /// # Failure
    ///
    /// Fails with [`None`] if the [`AtomicWaker`] is not initialized.
    ///
    /// # Remarks
    ///
    /// This method is safe to call multiple times, but it will only return
    /// the same [`Waker`] instance if it is called multiple times.
    ///
    /// The vtable is guaranteed to be the same as the [`default table`].
    ///
    /// [`default table`]: self::VTABLE
    #[inline]
    pub fn materialize(&self) -> Option<Waker> {
        let Self(atomic_address) = self;

        let task_address = atomic_address.load(Ordering::Acquire);

        NonNull::new(task_address).map(|task_address| unsafe {
            Waker::from_raw(RawWaker::new(NonNull::as_ptr(task_address), &self::VTABLE))
        })
    }

    /// Takes the [`Waker`] from the [`AtomicWaker`] and returns it.
    ///
    /// # Failure
    ///
    /// Fails with [`None`] if the [`AtomicWaker`] is not initialized.
    ///
    /// # Remarks
    ///
    /// The vtable is guaranteed to be the same as the [`default table`].
    ///
    /// [`default table`]: self::VTABLE
    #[inline]
    pub fn take(&self) -> Option<Waker> {
        let Self(atomic_address) = self;

        let task_address = atomic_address.swap(ptr::null_mut(), Ordering::AcqRel);

        NonNull::new(task_address).map(|task_address|
            // SAFETY: The pointer points to a valid task, and therefore abides by the vtable's expectations.
            unsafe {
                Waker::from_raw(RawWaker::new(NonNull::as_ptr(task_address), &self::VTABLE))
            })
    }
}

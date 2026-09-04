//! Wakeup behavior for tasks inside the executor.
//!
//! # Terminology
//!
//! - A *wake source* is an external trigger that can make one or more tasks runnable.
//! - A *wake sink* is the task affected by a wake source.
//! - A *sleeper* is a task with an active registration in a wake source.
//! - A *wake target* is a validated static data pointer paired with one of the executor's supported
//!   wake classes.
//!
//! # Executor assumptions
//!
//! Tasks are statically allocated and intrinsically pinned. Their wake targets
//! therefore require no allocation or reference counting. Each supported wake
//! class has a static [`RawWakerVTable`] with trivial clone and drop behavior.
//!
//! # Tagged wake targets
//!
//! [`WakeTarget`] stores a wake-class tag in the unused low bits of an aligned
//! data pointer. The class selects one table from a closed kernel-defined set.
//! Unknown tables and insufficiently aligned data pointers are rejected before
//! a target can be constructed.
//!
//! [`AtomicWaker`] stores either null or one tagged target in a single atomic
//! pointer. Null represents an inactive registration. An atomic exchange arms,
//! cancels, or claims a registration without a separate state word.
//!
//! # Multi-sleeper lists
//!
//! [`WakeList`] is an append-only intrusive list of [`WakeNode`] values. Each
//! node has permanent static membership and one atomic wake slot. Immutable
//! topology avoids physical removal, memory reclamation, and list ABA.
//!
//! # Interrupt safety
//!
//! Wake-slot operations use single atomic exchanges and list traversal never
//! waits on list state owned by an interrupted execution context. Supported
//! wake classes must be safe for concurrent and interrupt-context invocation.
//! Hardware atomics and callbacks may still have target-dependent latency.
//! Wake-all remains linear in the number of permanently linked nodes, so users
//! must bound list length where interrupt latency matters.
//!
//! [`WakeList`]: list::WakeList
//! [`WakeNode`]: list::WakeNode

pub mod list;

#[cfg(test)]
use core::sync::atomic::AtomicUsize;
use core::{
    fmt,
    mem::ManuallyDrop,
    ptr::{self, NonNull},
    sync::atomic::Ordering,
    task::{RawWaker, RawWakerVTable, Waker},
};

use nekor_sync::atomic::tagged::{AtomicTaggedPointer, Field, Tag, TagField, TaggedPointer};

use crate::{schedule::Scheduler, task::Task};

/// The low address bits reserved for a [`WakeClass`] tag.
const WAKE_TAG_MASK: usize = 0b111;

/// The virtual function table associated with task [`Waker`] instances.
pub static VTABLE: RawWakerVTable = const {
    /// Clones a task waker.
    ///
    /// # Safety
    ///
    /// `data` must have been sourced from a valid `&'static Task`.
    unsafe fn clone(data: *const ()) -> RawWaker {
        RawWaker::new(data, &self::VTABLE)
    }

    /// Schedules the addressed task.
    ///
    /// # Safety
    ///
    /// `task` must have been sourced from a valid `&'static Task`.
    unsafe fn wake(task: *const ()) {
        // SAFETY: The vtable contract requires a valid static task address.
        let target_task = unsafe { NonNull::<Task>::new_unchecked(task.cast::<Task>().cast_mut()).as_ref() };

        // TODO: Preserve a durable pending indication when the run queue is busy
        // or full.
        let _schedule_result = Scheduler::try_schedule(target_task);
    }

    /// Disposes of a task waker.
    ///
    /// Task wakers carry no reference count or other owned resource.
    const fn drop(_: *const ()) {}

    RawWakerVTable::new(clone, wake, wake, drop)
};

/// A test-only table for aligned atomic wake counters.
#[cfg(test)]
pub(super) static TEST_VTABLE: RawWakerVTable = const {
    /// Clones a test waker.
    ///
    /// # Safety
    ///
    /// `data` must address a static aligned `AtomicUsize`.
    unsafe fn clone(data: *const ()) -> RawWaker {
        RawWaker::new(data, &self::TEST_VTABLE)
    }

    /// Increments the addressed test counter.
    ///
    /// # Safety
    ///
    /// `data` must address a static aligned `AtomicUsize`.
    unsafe fn wake(data: *const ()) {
        let counter_address = data.cast_mut().cast::<AtomicUsize>();

        // SAFETY: The test vtable contract requires this exact static pointee.
        let target_counter = unsafe { NonNull::new_unchecked(counter_address).as_ref() };

        let _previous_count = target_counter.fetch_add(1, Ordering::SeqCst);
    }

    /// Disposes of a test waker.
    const fn drop(_: *const ()) {}

    RawWakerVTable::new(clone, wake, wake, drop)
};

/// A kernel-supported interpretation of a tagged wake-target pointer.
///
/// Every class must use static aligned data, trivial clone and drop behavior,
/// and a callback that is safe under concurrent interrupt-context invocation.
#[derive(Clone, Copy)]
#[repr(usize)]
enum WakeClass {
    /// A pointer to a static executor [`Task`].
    Task = 0,

    /// A pointer to a static aligned test counter.
    #[cfg(test)]
    Test = 1,
}

impl WakeClass {
    /// Determines the class assigned to a supported raw-waker table.
    #[inline]
    fn from_vtable(target_table: &'static RawWakerVTable) -> Option<Self> {
        if let Some(target) = ptr::eq(target_table, &raw const self::VTABLE).then_some(Self::Task) {
            return Some(target);
        }

        #[cfg(test)]
        if ptr::eq(target_table, &raw const self::TEST_VTABLE) {
            return Some(Self::Test);
        }

        None
    }

    /// Returns the raw-waker table assigned to this class.
    #[inline]
    const fn vtable(self) -> &'static RawWakerVTable {
        match self {
            Self::Task => &self::VTABLE,
            #[cfg(test)]
            Self::Test => &self::TEST_VTABLE,
        }
    }
}

// SAFETY: The three-bit mask reserves one contiguous low-bit field. The
// logical value is exactly one `WakeClass` selected by `TagField`.
unsafe impl Tag for WakeClass {
    type Type = TagField;
    type Value = Self;

    const MASK: usize = WAKE_TAG_MASK;
}

// SAFETY: Every encoded class fits the reserved field. Decoding accepts exactly
// the assigned discriminants and inverts each encoded class.
unsafe impl Field for WakeClass {
    #[inline]
    fn value(self) -> usize {
        self as usize
    }

    #[inline]
    fn from_value(target_value: usize) -> Option<Self> {
        match target_value {
            target_value if target_value == Self::Task as usize => Some(Self::Task),
            #[cfg(test)]
            target_value if target_value == Self::Test as usize => Some(Self::Test),
            _ => None,
        }
    }
}

// NOTE(invariant): The task wake class uses a `Task` data pointer. The explicit
// task alignment guarantees support for every bit required by `WakeClass`.
const _: () = assert!(
    TaggedPointer::<Task, WakeClass>::pointee_supports_tag(),
    "task pointers must support WakeClass tags"
);

/// A validated static wake target from the kernel's supported class set.
///
/// The target is copyable because every supported class has static data and
/// trivial ownership behavior. Copying this value does not clone an owned
/// resource.
// NOTE(invariant): The contained generic tagged pointer has a valid
// `WakeClass`. Its untagged address is valid static data for that class's
// raw-waker table.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct WakeTarget(TaggedPointer<(), WakeClass>);

impl WakeTarget {
    /// Validates and encodes a supported waker.
    ///
    /// Returns [`None`] when the vtable is not in the kernel's closed class
    /// set, when the data pointer is null, or when its address does not
    /// provide all reserved tag bits.
    #[inline]
    #[must_use]
    pub fn new(target_waker: &Waker) -> Option<Self> {
        let target_class = WakeClass::from_vtable(target_waker.vtable())?;
        let target_data = NonNull::new(target_waker.data().cast_mut())?;

        TaggedPointer::new(target_data, target_class).map(Self)
    }

    /// Invokes this target's class-specific wake-by-reference behavior.
    #[inline]
    fn wake_by_ref(self) -> bool {
        let Self(tagged_target) = self;
        let Some((target_data, target_class)) = tagged_target.split() else {
            return false;
        };
        let raw_waker = RawWaker::new(target_data.as_ptr().cast_const(), target_class.vtable());

        // SAFETY: The `WakeTarget` invariant proves that the decoded data and
        // selected vtable form a valid static waker. `ManuallyDrop` preserves
        // wake-by-reference semantics without consuming a logical raw-waker
        // ownership unit.
        let target_waker = ManuallyDrop::new(unsafe { Waker::from_raw(raw_waker) });

        target_waker.wake_by_ref();

        true
    }
}

impl fmt::Debug for WakeTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("WakeTarget").finish_non_exhaustive()
    }
}

// SAFETY: Every constructible target uses a supported `Waker`, whose data is
// static and whose callback satisfies the `Waker` thread-safety contract.
unsafe impl Send for WakeTarget {}

// SAFETY: `WakeTarget` is immutable and every supported callback permits
// concurrent wake-by-reference invocation.
unsafe impl Sync for WakeTarget {}

/// A nullable tagged wake target managed through one atomic pointer.
///
/// Null represents an inactive registration. A non-null value is a validated
/// [`WakeTarget`]. Arm, cancellation, and wake claiming use atomic exchanges so
/// an interrupt never waits for a registration owner.
// NOTE(invariant): The generic atomic value is either null or contains the
// tagged pointer from a valid `WakeTarget`. Only `arm` writes a non-null value.
#[repr(transparent)]
#[derive(Debug)]
pub struct AtomicWaker(AtomicTaggedPointer<(), WakeClass>);

impl AtomicWaker {
    /// Constructs an inactive atomic wake slot.
    #[inline]
    #[must_use]
    pub const fn null() -> Self {
        Self(AtomicTaggedPointer::null())
    }

    /// Arms this slot with a validated target.
    ///
    /// The acquiring read-modify-write synchronizes with a preceding wake claim
    /// on the same slot. This is required by the list's arm-before-check
    /// registration protocol.
    ///
    /// Returns whether another target was active immediately before this arm.
    #[inline]
    #[must_use]
    pub fn arm(&self, target_waker: WakeTarget) -> bool {
        let WakeTarget(tagged_target) = target_waker;
        self.0.swap(Some(tagged_target), Ordering::SeqCst).is_some()
    }

    /// Cancels the currently active target.
    ///
    /// A concurrent wake that claimed the target first may still invoke its
    /// callback after this method returns.
    ///
    /// Returns whether this call cancelled an active target.
    #[inline]
    #[must_use]
    pub fn cancel(&self) -> bool {
        self.0.swap(None, Ordering::SeqCst).is_some()
    }

    /// Determines whether this slot currently contains an active target.
    #[inline]
    #[must_use]
    pub fn is_armed(&self) -> bool {
        !self.0.is_null(Ordering::SeqCst)
    }

    /// Claims and invokes the currently active target.
    ///
    /// This operation clears the slot before invoking the callback, which makes
    /// recursive and concurrent wake attempts coalesce.
    ///
    /// Returns whether a valid active target was claimed and invoked.
    #[inline]
    #[must_use]
    pub fn wake(&self) -> bool {
        let Some(claimed_target) = self.0.swap(None, Ordering::SeqCst) else {
            return false;
        };

        WakeTarget(claimed_target).wake_by_ref()
    }
}

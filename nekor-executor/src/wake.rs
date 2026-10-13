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
    mem::{self, ManuallyDrop},
    num::NonZero,
    ptr::{self, NonNull},
    sync::atomic::{AtomicPtr, Ordering},
    task::{RawWaker, RawWakerVTable, Waker},
};

use nekor_bitwise::{
    field::multi,
    pointer::{Bijection, Tag, TaggedPtr, taggable},
    prelude::Counterpart,
};

use crate::{schedule::Scheduler, task::Task};

/// The address field reserved for a [`WakeClass`] code.
type WakeClassField<'value> = multi::Field<'value, 0, 2, usize>;

/// The mutable counterpart of [`WakeClassField`].
type WakeClassFieldMut<'value> = <WakeClassField<'value> as Counterpart>::Mut;

/// The alignment carrier for erased wake data.
///
/// The decoded wake class determines the actual pointee type used by its callback.
#[repr(align(8))]
// NOTE(invariant): None.
struct WakeData;

/// A non-null erased data pointer paired with its supported wake class.
type WakePointer = TaggedPtr<NonNull<WakeData>, WakeClass>;

/// The virtual function table associated with task [`Waker`] instances.
pub static VTABLE: RawWakerVTable = const {
    /// Clones a task waker.
    ///
    /// # Safety
    ///
    /// `data` must have been sourced from a valid `&'static Task`.
    #[expect(private_single_call_fn, reason = "RawWakerVTable requires a named ABI callback")]
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
    #[expect(private_single_call_fn, reason = "RawWakerVTable requires a named ABI callback")]
    const fn drop(_: *const ()) {}

    RawWakerVTable::new(clone, wake, wake, drop)
};

/// A test-only table for aligned atomic wake counters.
#[cfg(test)]
// NOTE(rationale): Wake-list tests in a sibling module need this static callback table.
pub(in crate::wake) static TEST_VTABLE: RawWakerVTable = const {
    /// Clones a test waker.
    ///
    /// # Safety
    ///
    /// `data` must address a static aligned `AtomicUsize`.
    #[expect(private_single_call_fn, reason = "RawWakerVTable requires a named ABI callback")]
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
    #[expect(private_single_call_fn, reason = "RawWakerVTable requires a named ABI callback")]
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

// SAFETY: Adding one to each discriminant assigns a distinct nonzero code.
// Decoding accepts exactly those codes and inverts the encoding.
unsafe impl Bijection for WakeClass {
    #[inline]
    fn encode(target_value: Self) -> NonZero<usize> {
        // SAFETY: The supported discriminants are zero and, in tests, one, so their
        // assigned codes are respectively one and two.
        unsafe { NonZero::new_unchecked(target_value as usize + 1) }
    }

    #[inline]
    fn decode(target_value: NonZero<usize>) -> Option<Self> {
        match target_value.get() {
            1 => Some(Self::Task),
            #[cfg(test)]
            2 => Some(Self::Test),
            _ => None,
        }
    }
}

// SAFETY: `WakeClassField` reserves a contiguous low field containing every assigned code.
// Its mutable counterpart replaces that field and preserves all other address bits.
unsafe impl Tag for WakeClass {
    const MASK: usize = WakeClassField::mask();

    #[inline]
    fn read(target_value: usize) -> usize {
        usize::from(WakeClassField::wrap(&target_value).value())
    }

    #[inline]
    fn write(mut target_value: usize, target_code: usize) -> usize {
        let target_code = WakeClassField::wrap(&target_code).value();

        _ = WakeClassFieldMut::wrap(&mut target_value).merge(target_code);

        target_value
    }
}

taggable! (
    unsafe impl Taggable<WakeClass> for WakeData;
);

// NOTE: Every valid task address satisfies the erased wake-data alignment.
const _: () = assert!(
    mem::align_of::<Task>() >= mem::align_of::<WakeData>(),
    "task alignment must satisfy the erased wake-data alignment"
);

/// A validated static wake target from the kernel's supported class set.
///
/// The target is copyable because every supported class has static data and
/// trivial ownership behavior. Copying this value does not clone an owned
/// resource.
#[derive(Clone, Copy)]
#[repr(transparent)]
// NOTE(invariant): The recovered data pointer has static lifetime and is valid for the
// stored wake class's vtable.
pub struct WakeTarget(WakePointer);

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
        let target_data = NonNull::new(target_waker.data().cast_mut().cast::<WakeData>())?;

        WakePointer::checked(target_data, target_class).map(Self)
    }

    /// Lowers this target to the packed pointer stored in an atomic wake slot.
    #[inline]
    fn raw(self) -> NonNull<()> {
        let Self(tagged_target) = self;

        let (target_data, target_class) = WakePointer::split(tagged_target);
        let target_code = WakeClass::encode(target_class);
        let target_pointer = target_data
            .as_ptr()
            .map_addr(|target_address| WakeClass::write(target_address, target_code.get()));

        // SAFETY: The assigned nonzero code occupies the reserved low bits, making
        // the packed address nonzero while preserving the data pointer's provenance.
        unsafe { NonNull::new_unchecked(target_pointer.cast::<()>()) }
    }

    /// Recovers a target claimed from an atomic wake slot.
    ///
    /// # Safety
    ///
    /// `target_pointer` must retain the address and provenance produced by [`Self::raw()`].
    #[inline]
    unsafe fn from_raw(target_pointer: NonNull<()>) -> Self {
        let target_address = target_pointer.as_ptr().addr();

        // SAFETY: The caller retains the packed address from `raw`, whose reserved
        // field contains an assigned nonzero code from `WakeClass::encode`.
        let target_code = unsafe { NonZero::new_unchecked(WakeClass::read(target_address)) };

        // SAFETY: The `Bijection` contract guarantees that decoding the assigned
        // code returns the original class independently of the calling context.
        let target_class = unsafe { WakeClass::decode(target_code).unwrap_unchecked() };
        let target_address = WakeClass::write(target_address, 0);
        let target_data = target_pointer.as_ptr().with_addr(target_address).cast::<WakeData>();

        // SAFETY: Clearing the reserved bits recovers the original `NonNull` data
        // pointer guaranteed by `TaggedPtr` and retained by the caller's contract.
        let target_data = unsafe { NonNull::new_unchecked(target_data) };

        // SAFETY: The pointer came from a validated target, so clearing its tag recovers
        // the original aligned data address with its provenance intact.
        let tagged_target = unsafe { WakePointer::new(target_data, target_class) };

        Self(tagged_target)
    }

    /// Invokes this target's class-specific wake-by-reference behavior.
    #[inline]
    fn wake_by_ref(self) -> bool {
        let Self(tagged_target) = self;

        let (target_data, target_class) = WakePointer::split(tagged_target);
        let target_data = target_data.as_ptr().cast::<()>().cast_const();

        let raw_waker = RawWaker::new(target_data, target_class.vtable());

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
#[repr(transparent)]
// NOTE(invariant): The atomic value is null or holds a tagged pointer from a valid `WakeTarget`.
pub struct AtomicWaker(AtomicPtr<()>);

impl AtomicWaker {
    /// Constructs an inactive atomic wake slot.
    #[inline]
    #[must_use]
    pub const fn null() -> Self {
        Self(AtomicPtr::new(ptr::null_mut()))
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
        let Self(target_slot) = self;

        let tagged_target = target_waker.raw();

        !target_slot.swap(tagged_target.as_ptr(), Ordering::SeqCst).is_null()
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
        let Self(target_slot) = self;

        !target_slot.swap(ptr::null_mut(), Ordering::SeqCst).is_null()
    }

    /// Determines whether this slot currently contains an active target.
    #[inline]
    #[must_use]
    pub fn is_armed(&self) -> bool {
        let Self(target_slot) = self;

        !target_slot.load(Ordering::SeqCst).is_null()
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
        let Self(target_slot) = self;

        let claimed_target = target_slot.swap(ptr::null_mut(), Ordering::SeqCst);

        match NonNull::new(claimed_target) {
            Some(claimed_target) => {
                // SAFETY: Only pointers lowered from valid `WakeTarget` values are stored
                // in this slot, and the atomic exchange preserves their provenance.
                unsafe { WakeTarget::from_raw(claimed_target) }.wake_by_ref()
            },
            None => false,
        }
    }
}

impl fmt::Debug for AtomicWaker {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("AtomicWaker").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use alloc::{boxed::Box, vec::Vec};
    use core::{
        mem,
        num::NonZero,
        ptr::{self, NonNull},
        sync::atomic::{AtomicPtr, AtomicUsize, Ordering},
        task::Waker,
    };
    use std::thread;

    use nekor_bitwise::pointer::{Bijection, Tag};

    use super::{AtomicWaker, TEST_VTABLE, WakeClass, WakeData, WakePointer, WakeTarget};

    /// Static data for the counter wake class.
    #[repr(C, align(8))]
    struct AlignedCounter(pub AtomicUsize);

    impl AlignedCounter {
        /// Allocate a counter with permanent test lifetime.
        fn new() -> &'static Self {
            let target_counter = Box::new(Self(AtomicUsize::new(usize::MIN)));

            #[cfg(miri)]
            {
                nekor_miri::memory::leak(target_counter)
            }
            #[cfg(not(miri))]
            {
                Box::leak(target_counter)
            }
        }

        /// Construct a target through the supported counter vtable.
        fn target(&'static self) -> WakeTarget {
            let Self(target_counter) = self;

            let target_address = ptr::from_ref(target_counter).cast::<()>();

            // SAFETY: The counter is static and aligned for `TEST_VTABLE`.
            let target_waker = unsafe { Waker::new(target_address, &TEST_VTABLE) };

            WakeTarget::new(&target_waker).expect("counter has supported class and alignment")
        }
    }

    /// Erased data retains its address and provenance through tagged pointer recovery.
    #[test]
    fn packed_target_round_trips_erased_data() {
        let counter = AlignedCounter::new();
        let target = counter.target();

        let WakeTarget(tagged_pointer) = target;

        let (target_data, target_class) = WakePointer::split(tagged_pointer);
        let AlignedCounter(counter_value) = counter;

        assert!(matches!(target_class, WakeClass::Test));
        assert_eq!(
            target_data.as_ptr().cast::<AtomicUsize>().cast_const(),
            ptr::from_ref(counter_value)
        );
        assert!(target.wake_by_ref());
        assert_eq!(counter_value.load(Ordering::SeqCst), 1);
    }

    /// Supported classes have distinct nonzero codes with inverse decoding.
    #[test]
    fn wake_class_codes_round_trip() {
        for target_class in [WakeClass::Task, WakeClass::Test] {
            let target_code = WakeClass::encode(target_class);
            let decoded_class = WakeClass::decode(target_code).expect("wake class code is assigned");

            let target_address = WakeClass::write(0, target_code.get());

            assert_eq!(target_code.get(), target_class as usize + 1);
            assert_eq!(decoded_class as usize, target_class as usize);
            assert_eq!(WakeClass::read(target_address), target_code.get());
        }
    }

    /// Unassigned nonzero patterns are rejected by the wake-class bijection.
    #[test]
    fn wake_class_rejects_unassigned_values() {
        for target_value in 3..=WakeClass::MASK {
            let target_code = NonZero::new(target_value).expect("unassigned code is nonzero");

            assert!(WakeClass::decode(target_code).is_none());
        }

        assert!(WakeClass::decode(NonZero::<usize>::MAX).is_none());
    }

    /// Atomic lowering retains the nonzero tag and the original static data pointer.
    #[test]
    fn packed_target_round_trips_atomic_storage() {
        let counter = AlignedCounter::new();
        let target = counter.target();
        let target_pointer = target.raw();

        assert_eq!(WakeClass::read(target_pointer.as_ptr().addr()), 2);

        // SAFETY: This pointer was lowered from a valid target immediately above.
        let recovered_target = unsafe { WakeTarget::from_raw(target_pointer) };
        let WakeTarget(tagged_pointer) = recovered_target;
        let (target_data, target_class) = WakePointer::split(tagged_pointer);
        let AlignedCounter(counter_value) = counter;

        assert!(matches!(target_class, WakeClass::Test));
        assert_eq!(
            target_data.as_ptr().cast::<AtomicUsize>().cast_const(),
            ptr::from_ref(counter_value)
        );
        assert!(recovered_target.wake_by_ref());
        assert_eq!(counter_value.load(Ordering::SeqCst), 1);
    }

    /// The task class uses a nonzero code while preserving the original pointer.
    #[test]
    fn task_class_has_nonzero_packed_address_bits() {
        let counter = AlignedCounter::new();
        let target_data = NonNull::from(counter).cast::<WakeData>();
        let tagged_pointer = WakePointer::checked(target_data, WakeClass::Task).expect("aligned data pointer");

        assert!(matches!(WakePointer::tag(tagged_pointer), WakeClass::Task));
        assert_eq!(WakeClass::encode(WakePointer::tag(tagged_pointer)).get(), 1);
        assert_eq!(WakePointer::pointer(tagged_pointer), target_data);
    }

    /// Actual address alignment is checked before a wake pointer can be tagged.
    #[test]
    fn misaligned_wake_data_is_rejected() {
        let counter = AlignedCounter::new();
        let target_data = NonNull::from(counter).cast::<WakeData>();
        let target_pointer = target_data.as_ptr().wrapping_byte_add(1);
        let target_data = NonNull::new(target_pointer).expect("offset static counter address is non-null");

        assert!(WakePointer::checked(target_data, WakeClass::Test).is_none());
    }

    /// A foreign null-data waker has no supported wake target.
    #[test]
    fn foreign_null_waker_is_rejected() {
        assert!(Waker::noop().data().is_null());
        assert!(WakeTarget::new(Waker::noop()).is_none());
    }

    /// Cancellation and repeated claiming preserve the null registration state.
    #[test]
    fn atomic_slot_arms_cancels_and_claims() {
        let counter = AlignedCounter::new();
        let target = counter.target();
        let slot = AtomicWaker::null();

        assert!(!slot.is_armed());
        assert!(!slot.wake());
        assert!(!slot.cancel());
        assert!(!slot.arm(target));
        assert!(slot.is_armed());
        assert!(slot.arm(target));
        assert!(slot.cancel());
        assert!(!slot.cancel());
        assert!(!slot.arm(target));
        assert!(slot.wake());
        assert!(!slot.wake());

        let AlignedCounter(counter_value) = counter;

        assert_eq!(counter_value.load(Ordering::SeqCst), 1);
    }

    /// Concurrent wake attempts claim the packed target exactly once.
    #[test]
    fn concurrent_claim_invokes_once() {
        let counter = AlignedCounter::new();
        let slot = AtomicWaker::null();

        assert!(!slot.arm(counter.target()));

        thread::scope(|scope| {
            let mut handles = Vec::new();

            for _ in 0..8 {
                handles.push(scope.spawn(|| slot.wake()));
            }

            let claimed_count = handles
                .into_iter()
                .map(|handle| usize::from(handle.join().expect("wake worker panicked")))
                .sum::<usize>();

            assert_eq!(claimed_count, 1);
        });

        let AlignedCounter(counter_value) = counter;

        assert_eq!(counter_value.load(Ordering::SeqCst), 1);
        assert!(!slot.is_armed());
    }

    /// Packing and atomic registration retain a pointer-sized representation.
    #[test]
    fn wake_storage_is_pointer_sized() {
        assert_eq!(mem::size_of::<WakeTarget>(), mem::size_of::<*mut ()>());
        assert_eq!(mem::size_of::<AtomicWaker>(), mem::size_of::<AtomicPtr<()>>());
        assert_eq!(mem::align_of::<AtomicWaker>(), mem::align_of::<AtomicPtr<()>>());
    }
}

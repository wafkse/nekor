//! Per-priority task queues and their linker-collected handles.
//!
//! [`Run::queue`] resolves a shared queue in a priority-specific domain. [`Run::global`]
//! emits a handle in a linker section whose zero-padded priority suffix sorts numerically.
//! [`Run::list`] borrows the collected handles between the linker boundary symbols.
//!
//! Queue operations use the bounded concurrent queue in `nekor-structure`. A failed enqueue
//! can mean contention or a full queue, so callers must decide how to keep work pending.
//! The queue-capacity feature selects one power-of-two capacity for all priorities.
//!
//! # Storage
//!
//! The platform must keep emitted queue handles aligned and contiguous between
//! `__nekor_queue_start` and `__nekor_queue_end`. Their pointers refer to domain storage
//! with program lifetime. Cache padding separates handles according to the target's
//! configured alignment. It does not prove a particular processor's cache geometry.

use core::{
    arch, ffi, fmt, mem,
    ops::Deref,
    ptr::{self, NonNull},
    slice,
    sync::atomic::AtomicPtr,
};

use nekor_aal::cache::prelude::CachePadded;
use nekor_domain::{
    domain::{Adapter, Domain},
    prelude::{Erased, Static, Store},
};
use nekor_structure::queue::Queue;

use crate::task::Task;

/// A macro to define various `cfg`-dependant task queue sizes.
///
/// This uses the feature `"task-queue-size-{N}"`, where `log2(N) ∈ ℤ` to
/// represent each preset queue size.
///
/// Each one of these feature flags are exclusive to one another, as [`Queue`]
/// only supports sizes that are an integer power of two.
///
/// This is implemented for queue sizes up to `65536` (`2 ^ 16`), which is
/// overkill for most if not all applications.
macro_rules! size {
    () => {};
    ($(($target_size:literal, $feature:literal)),+ $(,)?) => {
        const {
            $(
                if cfg!(feature = $feature) {
                    $target_size
                } else
            )+
            {
                panic!(concat!(
                    "No task size feature enabled, please enable one of the following:",
                    $(" task-queue-size-", $feature),+
                ))
            }
        }
    };
}

/// The static [`Domain`] for all [`Run`] queues.
enum Priority<const P: u16> {}

/// The [`Adapter`] for the [`Priority`] domain.
#[repr(transparent)]
// NOTE(invariant): The wrapped value remains associated with priority `P` at the domain boundary.
struct Prioritized<T, const P: u16>(T);

// SAFETY: `Prioritized` is `repr(transparent)` over a single `T`.
unsafe impl<T, const P: u16> Adapter for Prioritized<T, P>
where
    T: Store,
{
    type Target = T;
}

impl<const P: u16> Domain for Priority<P> {
    type Adapter<T>
        = Prioritized<T, P>
    where
        T: Store;
}

/// An uninhabited helper type used as an umbrella for task runqueue size
/// constants.
enum Size {}

impl Size {
    /// The preset runqueue size for each priority level.
    pub const QUEUE: usize = size!(
        (1, "task-queue-size-1"),
        (2, "task-queue-size-2"),
        (4, "task-queue-size-4"),
        (8, "task-queue-size-8"),
        (16, "task-queue-size-16"),
        (32, "task-queue-size-32"),
        (64, "task-queue-size-64"),
        (128, "task-queue-size-128"),
        (256, "task-queue-size-256"),
        (512, "task-queue-size-512"),
        (1024, "task-queue-size-1024"),
        (2048, "task-queue-size-2048"),
        (4096, "task-queue-size-4096"),
        (8192, "task-queue-size-8192"),
        (0x4000, "task-queue-size-16384"),
        (0x8000, "task-queue-size-32768"),
        (0x0001_0000, "task-queue-size-65536"),
    );
}

/// A new-type to encompass the global runqueue list.
#[repr(transparent)]
// NOTE(invariant): The stored slice has static storage for every runqueue handle it contains.
pub struct RunList(&'static [Run]);

impl RunList {
    /// Wrap the global runqueue list in this new-type.
    #[inline]
    pub const fn wrap(target_list: &'static [Run]) -> Self {
        Self(target_list)
    }

    /// Unwrap the global runqueue list from this new-type.
    #[inline]
    #[must_use]
    pub const fn unwrap(&self) -> &'static [Run] {
        let &Self(target_list) = self;

        target_list
    }
}

/// The [`Deref`] implementation for [`RunList`].
///
/// This artitifically shortens the lifetime of the list.
///
/// To preserve the `'static` lifetime, use [`RunList::unwrap`].
impl Deref for RunList {
    type Target = [Run];

    #[inline]
    fn deref(&self) -> &Self::Target {
        let &Self(target_list) = self;

        target_list
    }
}

impl fmt::Debug for RunList {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let &Self(target_list) = self;

        f.debug_list().entries(target_list).finish_non_exhaustive()
    }
}

/// A type alias to the [`Queue`] type used for [`Task`] scheduling.
///
/// This is the queue type used for all per-priority queues, as priority is
/// simply a type-level aspect.
pub type TaskQueue = Queue<&'static Task, { Size::QUEUE }>;

/// A shared handle to the queue associated with one priority.
///
/// Multiple handles for the same priority resolve the same domain-backed queue.
#[repr(transparent)]
// NOTE(invariant): The padded erased pointer refers to a statically allocated queue for its
// priority.
pub struct Run(
    // NOTE(cache): This is cache-padded to avoid the interference from
    // multiple cores accessing the global runqueue slice.
    CachePadded<Erased<TaskQueue>>,
);

impl Run {
    /// Independently construct a new run queue handle for the target priority
    /// level `P`.
    ///
    /// # Remarks
    ///
    /// The [`Run`] will point to the same runqueue for the target priority
    /// level, regardless of where it is initialized.
    ///
    /// For a bare reference to a globally-available runqueue list, use
    /// [`Run::global`].
    #[inline]
    #[must_use]
    pub const fn queue<const P: u16>() -> Self {
        Self(CachePadded::new(Erased::pointer::<Priority<P>>()))
    }

    /// Determine the global runqueue handle for the target priority level `P`.
    ///
    /// For an in-stack construction of a runqueue handle, use [`Run::queue`].
    #[inline]
    #[must_use]
    pub const fn global<const P: u16>() -> &'static Self {
        /// A `fn item` whose sole purpose is to materialize the required bytes
        /// for a [`Run`] queue instance of priority `P`.
        ///
        /// ## Implementation Details
        ///
        /// This is a *naked* function, in its code bytes is a single
        /// pointer-sized pc-relative value to the [`TaskQueue`] handle.
        ///
        /// # Safety
        ///
        /// * This function must never be called, as it is only used to reserve space for the
        ///   runqueue.
        ///
        /// Note that the safety call requirement will be superceded once the
        /// `custom` ABI is stable.
        ///
        /// Since this item is local to [`Run::global`], safety notes are inside
        /// the `naked_asm!` invocation. They will be refered by their
        /// associated number.
        #[unsafe(link_section = concat!(env!("KERNEL_SCHEDULE_QUEUE_SECTION"), ".setup"))]
        #[unsafe(naked)]
        unsafe extern "C" fn queue<const P: u16>() -> ! {
            /// Determine the `N`-th digit of a target `u16`.
            const fn digit<const N: usize>(number: u16) -> usize {
                const MAX_DIGITS: usize = // ceil(log10(u16::MAX))
                    5;

                assert!(N <= MAX_DIGITS, "invalid digit index");

                let mut number = number as usize;

                let mut digit_count = 1;

                while digit_count < N {
                    number /= 10;

                    digit_count += 1;
                }

                number % 10
            }

            arch::naked_asm!(
                // SAFETY(1): This is the offset to the actual storage bytes. Not necesarily aligned.
                ".{target_size}byte 2f - .",
                concat!(
                    ".pushsection",
                    " ",
                    env!("KERNEL_SCHEDULE_QUEUE_SECTION"),
                    // NOTE: This requires digit-based formatting to ensure that numeric ordering is preserved during lexicographic sorting by the linker script.
                    ".priority.{p5}{p4}{p3}{p2}{p1}",
                    " ",
                    ", \"aw\", @progbits"
                ),
                // SAFETY(2): Align to runqueue boundary, then a per-field approach.
                ".balign {queue_alignment}",
                "2:",
                ".balign {pointer_alignment}",
                ".{target_size}byte {target_storage}",
                ".balign {atomic_alignment}",
                ".{target_size}byte 0x00",
                // SAFETY(3): Add any final alignment padding.
                ".balign {queue_alignment}",
                ".popsection",
                queue_alignment = const mem::align_of::<Run>(),
                pointer_alignment = const mem::align_of::<fn() -> &'static Run>(),
                atomic_alignment = const mem::align_of::<AtomicPtr<Run>>(),
                target_size = const mem::size_of::<usize>(),
                target_storage = sym Static::value_default_in::<TaskQueue, Priority<P>>,
                // NOTE(rationale): See rationale at template site for `p{1..5}`.
                p1 = const digit::<1>(P),
                p2 = const digit::<2>(P),
                p3 = const digit::<3>(P),
                p4 = const digit::<4>(P),
                p5 = const digit::<5>(P),
                options(att_syntax),
            );
        }

        let queue_storage = queue::<P> as *const usize;

        // SAFETY: The naked item emits a pointer-sized relative offset at this address,
        // and `ptr::read_unaligned` accepts the template's initial unaligned location.
        let pointer_offset = unsafe { ptr::read_unaligned(queue_storage) };

        // SAFETY: The function pointer is non-null. The encoded offset reaches
        // storage aligned for `Run` and initialized by the linker template.
        unsafe {
            NonNull::new_unchecked(queue_storage.cast_mut().cast::<u8>())
                .byte_add(pointer_offset)
                .cast::<Self>()
                .as_ref()
        }
    }

    /// Determine the global runqueue list, which is a static slice of all
    /// [`Run`] queues, ordered by priority level.
    #[inline]
    #[must_use]
    pub const fn list() -> &'static [Self] {
        // SAFETY: These declarations name storage boundaries emitted by the
        // platform linker script, they are not foreign symbols.
        unsafe extern "C" {
            /// The start of the global runqueue list.
            static __nekor_queue_start: ffi::c_void;

            /// The end of the global runqueue list.
            ///
            /// NOTE: This is not the *last* [`Run`], but rather a pointer to
            /// where the list is terminated.
            static __nekor_queue_end: ffi::c_void;
        }

        // SAFETY: The platform linker script places aligned Run handles contiguously
        // between these resident boundary symbols, Furthermore, the end symbol is one element past
        // the final handle, so their distance counts exactly the initialized elements.
        unsafe {
            let start = (&raw const __nekor_queue_start).cast::<Self>();
            let end = (&raw const __nekor_queue_end).cast::<Self>();

            // FIXME(unstable): Use `slice::from_ptr_range` once it is
            // stabilized.
            slice::from_raw_parts(start, end.offset_from_unsigned(start))
        }
    }
}

impl Deref for Run {
    type Target = TaskQueue;

    #[inline]
    fn deref(&self) -> &Self::Target {
        let Self(target_queue) = self;

        Erased::value(target_queue)
    }
}

impl fmt::Debug for Run {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self(target_queue) = self;

        write!(f, "<runqueue governed by @{:?}", Erased::constructor(target_queue))
    }
}

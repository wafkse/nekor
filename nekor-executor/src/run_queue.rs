//! Priority-based task queue system with per-priority global runqueues.
//!
//! This module provides a lock-free, multi-producer multi-consumer task queue
//! for each priority level, using compile-time memory section management via
//! the `nekor-domain` crate.
//!
//! For a given priority `P` (`u16`, where `0` is the highest priority), a
//! [`Run`] represents a handle to a global [`TaskQueue`] dedicated to tasks of
//! that priority. These queues are backed by a `Queue<&'static Task, {
//! Size::QUEUE }>` type, with a fixed, power-of-two size defined by the `Size`
//! enum.
//!
//! Each priority queue is emitted into a dedicated linker section named
//! `.nekor.queue.priority.{P}`, which are then merged and sorted by the linker
//! script into a single global `.nekor.queue` section. The [`RunList`] type
//! wraps this global list as a slice of all priority queues, ordered by
//! priority.
//!
//! The [`Run::queue`] constructor provides a new handle to the queue for a
//! given priority, while [`Run::global`] returns a reference to the globally
//! available queue instance.
//!
//! The system leverages careful alignment, cache padding, and `naked` functions
//! with linker section annotations to provide zero-cost, globally consistent
//! runqueue handles.
//!
//! # Global Run-Queue List
//!
//! The global list of runqueues is defined entirely by linker symbols emitted
//! per priority level. This allows the executor to iterate through all queues
//! in priority order without additional runtime bookkeeping.
//!
//! # Technical Notes
//!
//! - Each [`Run`] is cache-padded and wraps an [`Erased<TaskQueue>`] to avoid
//!   false sharing and enable lock-free access.
//! - The global runqueue list slice is constructed from linker-provided
//!   start/end symbols (`__nekor_queue_start` / `__nekor_queue_end`).
//! - Queue sizes are configurable via `cfg` feature flags
//!   (`task-queue-size-{N}`), which must be powers of two.
//! - The system relies heavily on `unsafe` code for low-level pointer and
//!   section manipulation; correctness is ensured by the static layout and
//!   linker guarantees.
//! - The queue section name is provided by the `KERNEL_SCHEDULE_QUEUE_SECTION`
//!   environment variable at compile-time.
//!
//! [`Erased`]: nekor_domain::prelude::Erased

use core::{
    arch, ffi, fmt, mem,
    ops::Deref,
    ptr::{self, NonNull},
    slice,
    sync::atomic::AtomicPtr,
};

use nekor_domain::{
    domain::{Adapter, Domain},
    prelude::{Erased, Static, Store},
};

use nekor_aal::cache::prelude::CachePadded;
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
    (
        $($target_size:literal),+
    ) => {
        tokel::stream!(
            const {
                'a: {
                    $(
                        #[cfg(feature = [<
                            "task-queue-size-"
                            // NOTE: This macro parameter can be embedded within a None-delimited token
                            // group, so we forcibly flatten whatever the macro expansion engine gives us.
                            [< $target_size >]:flatten
                        >]:to_string:concatenate)]
                        break 'a $target_size;
                    )*

                    #[allow(unreachable_code)]
                    {
                        unreachable!(
                            concat!(
                                "No task size feature enabled, please enable one of the following:",
                                $(
                                    " ",

                                    "`",

                                    "task-queue-size-", stringify!($target_size), "`"
                                ),*
                            )
                        )
                    }
                }
            }
        )
    };
}

/// The static [`Domain`] for all [`Run`] queues.
enum Priority<const P: u16> {}

/// The [`Adapter`] for the [`Priority`] domain.
#[repr(transparent)]
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
        1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768, 65536
    );
}

/// A new-type to encompass the global runqueue list.
#[repr(transparent)]
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

/// A lock-free runqueue handle that is known to be exclusive for a priority
/// level.
///
/// This is a no-cost wrapper over a [`Erased`] smart pointer.
#[repr(transparent)]
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
        /// * This function must never be called, as it is only used to reserve
        ///   space for the runqueue.
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
                const MAX_DIGITS: usize = /* ceil(log10(u16::MAX)) */ 5;

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
            );
        }

        let queue_storage = queue::<P> as *const usize;

        // SAFETY: Safe, as the pointer is sourced from a *naked fn* with
        // hand-written contents. (see note [1])
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
        // SAFETY: [see next comment]
        unsafe extern "C" {
            /// The start of the global runqueue list.
            static __nekor_queue_start: ffi::c_void;

            /// The end of the global runqueue list.
            ///
            /// NOTE: This is not the *last* [`Run`], but rather a pointer to
            /// where the list is terminated.
            static __nekor_queue_end: ffi::c_void;
        }

        // SAFETY: These are magic symbols that are defined by the linker
        // script, which are statically guaranteed to define the runqueue list.
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
        let Self(task_queue) = self;

        Erased::value(task_queue)
    }
}

impl fmt::Debug for Run {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self(task_queue) = self;

        write!(
            f,
            "<runqueue governed by @{:?}",
            Erased::constructor(task_queue)
        )
    }
}

use core::{
    cell::UnsafeCell,
    marker::{self},
    mem::MaybeUninit,
    ptr::NonNull,
};

use nekor_aal::signal::prelude::Monitored;

use nekor_backoff::{prelude::Backoff, retry::Retry};

use nekor_sync::atomic::bitmap::{
    AtomicBitmap,
    at::At,
    conditional::{Status, Unset},
    mode::{
        Mode,
        cooperative::Cooperative,
        exclusive::{Exclusive, Outcome, Reason},
    },
    typeutil::{BitMapUsize, InBound},
};

use crate::slotted::state::{InitializationState, ReserveState, SlotState};

/// A reserved slot in the queue.
struct Reserved<'a, const N: usize, T>
where
    BitMapUsize<N>: InBound,
{
    /// The bit index allocated for this reserve.
    reserve_index: u32,

    /// The state of the queue.
    queue_state: &'a SlotState<N>,

    /// A marker to indicate reference of some `T`.
    marker: marker::PhantomData<&'a T>,
}

impl<'a, const N: usize, T> Reserved<'a, N, T>
where
    BitMapUsize<N>: InBound,
{
    /// Attempt to acquire a reserve in the queue backed by the target
    /// [`SlotState`].
    ///
    /// For an alternative with self-imposed limits and backoff, see
    /// [`Reserved::acquire_with`].
    #[inline]
    fn reserve(target_state: &'a SlotState<N>) -> Option<Self> {
        Self::reserve_with(target_state, &mut Retry::unlimited(Backoff::minimal()))
    }

    /// Attempt to acquire a reserve in the queue backed by the target
    /// [`SlotState`], with explicit backoff and limit imposition.
    fn reserve_with(
        target_state: &'a SlotState<N>,
        backoff_state: &mut <Exclusive as Mode>::State,
    ) -> Option<Self> {
        let (ref reserve_state, ref initialization_state) = (
            SlotState::reserve(target_state),
            SlotState::initialization(target_state),
        );
        let initial_index = u32::try_from(N - 1).unwrap_or_else(|_| {
            // `InBound` is only implemented for bitmap widths representable
            // by the platform index type.
            unreachable!()
        });

        loop {
            let reserve_at = AtomicBitmap::at_rightmost(reserve_state).map_or_else(
                || AtomicBitmap::try_at(reserve_state, initial_index),
                At::right,
            );

            match reserve_at.filter(SlotState::<N>::filter) {
                Some(ref reserve_at) => match reserve_at.one_with::<Exclusive>(backoff_state) {
                    Outcome::Success(..) => {
                        let reserve_index = At::index(reserve_at);

                        while AtomicBitmap::condition(
                            initialization_state,
                            Unset(1 << reserve_index),
                        ) == Status::Unmet
                        {
                            Monitored::wait(AtomicBitmap::monitor(initialization_state));
                        }

                        break Some(Self {
                            reserve_index,
                            queue_state: target_state,
                            marker: marker::PhantomData,
                        });
                    }
                    Outcome::Failure(Reason::Unchanged, ..) => unreachable!(),
                    Outcome::Failure(Reason::Contended, ..) => (),
                    Outcome::Failure(Reason::Limited, ..) => break None,
                },
                None => break None,
            }
        }
    }
}

/// An initialized slot in the queue.
struct Initialized<'a, const N: usize, T>(marker::PhantomData<&'a T>)
where
    BitMapUsize<N>: InBound;

impl<'a, const N: usize, T> Initialized<'a, N, T>
where
    BitMapUsize<N>: InBound,
{
    /// Initialize the provided [`Reserve`] in the target storage.
    ///
    /// # Safety
    ///
    /// The target storage must be for the specific [`SlotState`] in which the
    /// [`Reserve`] was created.
    unsafe fn slot(
        Reserved {
            reserve_index,
            queue_state,
            ..
        }: Reserved<'a, N, T>,
        target_value: T,
        target_storage: &'a [UnsafeCell<MaybeUninit<T>>; N],
    ) {
        let Some(mut target_storage) = target_storage
            .get(reserve_index as usize)
            .map(UnsafeCell::get)
            .and_then(NonNull::new)
        else {
            unreachable!("a reserved queue index must be in bounds")
        };

        // SAFETY: The slot is exclusively assigned to this reservation and
        // belongs to the corresponding storage array.
        let uninit_ref = unsafe { target_storage.as_mut() };

        MaybeUninit::write(uninit_ref, target_value);

        let _ = InitializationState::bitmap(queue_state.initialization())
            .at(reserve_index)
            .one::<Cooperative>();
    }
}

struct Acquired<'a, const N: usize, T>
where
    BitMapUsize<N>: InBound,
{
    /// The bit index that was acquired.
    acquired_index: u32,

    /// Marker type to indicate ownership of a `T` for an `'a` lifetime.
    marker: marker::PhantomData<&'a T>,
}

impl<'a, const N: usize, T> Acquired<'a, N, T>
where
    BitMapUsize<N>: InBound,
{
    /// Attempt to acquire a reserve in the queue backed by the target
    /// [`SlotState`].
    ///
    /// For an alternative with self-imposed limits and backoff, see
    /// [`Reserved::acquire_with`].
    #[inline]
    fn acquire(target_state: &'a SlotState<N>) -> Option<Self> {
        Self::acquire_with(target_state, &mut Retry::unlimited(Backoff::minimal()))
    }

    /// Attempt to acquire a reserve in the queue backed by the target
    /// [`SlotState`], with explicit backoff and limit imposition.
    fn acquire_with(
        target_state: &'a SlotState<N>,
        backoff_state: &mut <Exclusive as Mode>::State,
    ) -> Option<Self> {
        let initialization_state = &SlotState::initialization(target_state);

        loop {
            // NOTE: No need for filtering the `At` engagement vector here, as the engaged
            // bit was filtered during the reserve.
            match AtomicBitmap::at_leftmost(initialization_state) {
                Some(ref init_at) => match init_at.zero_with::<Exclusive>(backoff_state) {
                    Outcome::Success(..) => {
                        break Some(Self {
                            acquired_index: init_at.index(),
                            marker: marker::PhantomData,
                        });
                    }

                    Outcome::Failure(Reason::Unchanged | Reason::Contended, ..) => (),

                    Outcome::Failure(Reason::Limited, ..) => break None,
                },
                None => break None,
            }
        }
    }
}

/// A lock-free, concurrent first-in-first-out (*FIFO*) queue.
///
/// This queue uses bitmap-based slot management to provide lock-free enqueue
/// and dequeue operations. The queue has a fixed capacity of `N` elements.
///
/// # Capacity
///
/// The queue can hold at most `N` elements, where `N` must be less than or
/// equal to [`usize::BITS`]. Attempting to enqueue when full will return a
/// [`BusyOrFull`] error.
///
/// # Busy State
///
/// This queue can become busy under special conditions, where at maximum a full
/// `N`-element dequeue can restore normal operation.
///
/// This primarily happens when concurrent queue dequeues are not performed in a
/// sufficiently steady rate compared to queue enqueues. In this case, the queue
/// will be considered busy and it will be impossible to enqueue further
/// elements until a full "flush" (i.e., a total dequeue of every existing
/// element) is performed.
///
/// # Interrupt Safety
///
/// This type is interrupt-safe, and can be used without engaging in a Local
/// Critical Section (*LCS*).
///
/// # FIFO Ordering
///
/// The queue maintains FIFO ordering through its bitmap operations:
/// - Enqueue finds slots using rightmost-one + right (or MSB if empty), growing
///   leftward.
/// - Dequeue finds slots using leftmost-one, consuming from the oldest end.
#[derive(Debug)]
pub struct Queue<T, const N: usize = { usize::BITS as _ }>
where
    BitMapUsize<N>: InBound,
{
    /// The slot state managing reservation and initialization bitmaps.
    slot_state: SlotState<N>,

    /// The `N`-element array where the queue elements are stored.
    target_storage: [UnsafeCell<MaybeUninit<T>>; N],
}

/// The reason for a [`BusyOrFull`] failure.
#[derive(Debug, Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub enum BusyOrFullReason {
    /// The [`Queue`] was busy.
    Busy,

    /// The [`Queue`] was completely full.
    Full,
}

/// An error type indicating whether the [`Queue`] is either busy or completely
/// full.
///
/// This includes the source value of type `T` that could not be enqueued.
#[derive(Debug, Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct BusyOrFull<T>(pub T);

impl<T> BusyOrFull<T> {
    /// Unwrap the source value of type `T` from this [`BusyOrFull`] error.
    #[inline]
    pub fn unwrap(self) -> T {
        let Self(target_value) = self;

        target_value
    }
}

impl<T, const N: usize> Queue<T, N>
where
    BitMapUsize<N>: InBound,
{
    /// Construct a new, empty `N`-element [`Queue`].
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        let slot_state = SlotState::zeroed();
        let target_storage = [const { UnsafeCell::new(MaybeUninit::uninit()) }; N];

        Self {
            slot_state,
            target_storage,
        }
    }

    /// Attempt to enqueue an element of type `T` onto this [`Queue`].
    ///
    /// This operation reserves a slot, writes the value, and marks it as
    /// initialized, making it available for dequeue operations.
    ///
    /// # Errors
    ///
    /// If the [`Queue`] is either busy or full, a [`BusyOrFull`] error is
    /// returned containing the value that could not be enqueued.
    ///
    /// # Busy vs Full
    ///
    /// - *Full*: All `N` slots are actively in use (reserved or initialized).
    /// - *Busy*: Concurrent operations and inherent bitmap positional layout
    ///   are preventing progress, but slots may become available in the future.
    ///
    /// In most cases, retrying after a brief delay will succeed once dequeue
    /// operations complete.
    #[inline]
    pub fn enqueue(&self, target_value: T) -> Result<(), BusyOrFull<T>> {
        let Self {
            slot_state,
            target_storage,
        } = self;

        match Reserved::<N, T>::reserve(slot_state) {
            Some(reserved) => {
                // SAFETY: The provided storage array is backed by the same `SlotState` that was
                // acquired.
                unsafe { Initialized::slot(reserved, target_value, target_storage) };

                Ok(())
            }
            None => Err(BusyOrFull(target_value)),
        }
    }

    /// Attempt to dequeue an element of type `T` from this [`Queue`].
    ///
    /// This operation acquires the oldest initialized slot, reads the value,
    /// and returns the slot to the empty state.
    ///
    /// # Returns
    ///
    /// - `Some(value)` if an element was successfully dequeued.
    /// - `None` if the queue is empty.
    ///
    /// # FIFO Ordering
    ///
    /// This operation always returns the oldest enqueued element that has not
    /// yet been dequeued, maintaining strict FIFO semantics.
    #[inline]
    pub fn dequeue(&self) -> Option<T> {
        let Self {
            slot_state,
            target_storage,
        } = self;

        let Acquired { acquired_index, .. } = Acquired::<N, T>::acquire(slot_state)?;
        let Some(mut target_storage) = target_storage
            .get(acquired_index as usize)
            .map(UnsafeCell::get)
            .and_then(NonNull::new)
        else {
            unreachable!("an acquired queue index must be in bounds")
        };

        // SAFETY: The acquired initialized slot is exclusively assigned to
        // this operation and belongs to the corresponding storage array.
        let init_ref = unsafe { target_storage.as_mut() };

        // SAFETY: Acquiring the initialization bit proves the value exists.
        let target_value = unsafe { init_ref.assume_init_read() };

        // The reservation bit is exclusively owned after consuming its
        // initialization bit, so the clear cannot be contended away.
        let _ = ReserveState::bitmap(slot_state.reserve())
            .at(acquired_index)
            .zero::<Cooperative>();

        Some(target_value)
    }
}

// SAFETY: Queue can be safely shared across threads because all operations use
// atomic bitmap operations for synchronization, and the storage array is
// protected by the reservation invariants.
unsafe impl<T, const N: usize> Sync for Queue<T, N>
where
    T: Send,
    BitMapUsize<N>: InBound,
{
}

impl<T, const N: usize> Drop for Queue<T, N>
where
    BitMapUsize<N>: InBound,
{
    fn drop(&mut self) {
        let &mut Self {
            ref slot_state,
            ref mut target_storage,
        } = self;

        // Drop all initialized values by checking the initialization bitmap.
        let init_snapshot = slot_state.initialization().snapshot();

        for slot_index in 0..N {
            let slot_mask = 1usize << slot_index;

            // Check if this slot is initialized.
            if (init_snapshot & slot_mask) != 0 {
                // SAFETY: The initialization bit guarantees this slot contains a valid value.
                unsafe {
                    let target_slot = target_storage.get_unchecked_mut(slot_index).get_mut();

                    let _ = target_slot.assume_init_read();
                    // Value is dropped here automatically
                }
            }
        }
    }
}

impl<T, const N: usize> Default for Queue<T, N>
where
    BitMapUsize<N>: InBound,
{
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_queue_is_empty() {
        let queue: Queue<i32, 8> = Queue::new();
        assert!(queue.dequeue().is_none());
    }

    #[test]
    fn test_enqueue_single_element() {
        let queue: Queue<i32, 8> = Queue::new();
        assert!(queue.enqueue(42).is_ok());
    }

    #[test]
    fn test_enqueue_dequeue_single_element() {
        let queue: Queue<i32, 8> = Queue::new();
        assert!(queue.enqueue(42).is_ok());
        assert_eq!(queue.dequeue(), Some(42));
        assert!(queue.dequeue().is_none());
    }

    #[test]
    fn test_enqueue_dequeue_multiple_elements() {
        let queue: Queue<i32, 8> = Queue::new();

        for i in 0..5 {
            assert!(queue.enqueue(i).is_ok());
        }

        for i in 0..5 {
            assert_eq!(queue.dequeue(), Some(i));
        }

        assert!(queue.dequeue().is_none());
    }

    #[test]
    fn test_enqueue_until_full() {
        let queue: Queue<i32, 8> = Queue::new();

        // Fill the queue
        for i in 0..8 {
            assert!(queue.enqueue(i).is_ok(), "Failed to enqueue at index {i}");
        }

        // Next enqueue should fail
        assert!(queue.enqueue(999).is_err());
    }

    #[test]
    fn test_queue_fifo_ordering() {
        let queue: Queue<i32, 16> = Queue::new();

        // Enqueue elements 0..10
        for i in 0..10 {
            assert!(queue.enqueue(i).is_ok());
        }

        // Dequeue should return elements in FIFO order
        for i in 0..10 {
            assert_eq!(queue.dequeue(), Some(i));
        }
    }

    #[test]
    fn test_interleaved_enqueue_dequeue() {
        let queue: Queue<i32, 8> = Queue::new();

        assert!(queue.enqueue(1).is_ok());
        assert!(queue.enqueue(2).is_ok());
        assert_eq!(queue.dequeue(), Some(1));
        assert!(queue.enqueue(3).is_ok());
        assert_eq!(queue.dequeue(), Some(2));
        assert_eq!(queue.dequeue(), Some(3));
        assert!(queue.dequeue().is_none());
    }

    #[test]
    fn test_fill_and_drain_cycle() {
        let queue: Queue<i32, 8> = Queue::new();

        // First cycle
        for i in 0..8 {
            assert!(queue.enqueue(i).is_ok());
        }
        for i in 0..8 {
            assert_eq!(queue.dequeue(), Some(i));
        }

        // Second cycle
        for i in 10..18 {
            assert!(queue.enqueue(i).is_ok());
        }
        for i in 10..18 {
            assert_eq!(queue.dequeue(), Some(i));
        }
    }

    #[test]
    fn test_queue_with_non_copy_types() {
        let queue: Queue<String, 8> = Queue::new();

        assert!(queue.enqueue(String::from("hello")).is_ok());
        assert!(queue.enqueue(String::from("world")).is_ok());

        assert_eq!(queue.dequeue(), Some(String::from("hello")));
        assert_eq!(queue.dequeue(), Some(String::from("world")));
        assert!(queue.dequeue().is_none());
    }

    #[test]
    fn test_drop_semantics() {
        use core::sync::atomic::{AtomicUsize, Ordering};

        static DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

        struct DropCounter;

        impl Drop for DropCounter {
            fn drop(&mut self) {
                DROP_COUNT.fetch_add(1, Ordering::SeqCst);
            }
        }

        {
            let queue: Queue<DropCounter, 8> = Queue::new();

            // Enqueue and dequeue items
            for _ in 0..5 {
                assert!(queue.enqueue(DropCounter).is_ok());
            }

            for _ in 0..3 {
                let _ = queue.dequeue();
            }

            // 3 items dropped so far
            assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 3);
        }

        // Queue dropped, remaining 2 items should be dropped
        assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 5);
    }
}

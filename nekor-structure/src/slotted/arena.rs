//! A static array data structure that allocates elements in a constant-time
//! manner.

use core::{cell::UnsafeCell, mem::MaybeUninit, ptr};

use nekor_backoff::{prelude::Backoff, retry::Retry};
use nekor_sync::atomic::bitmap::{
    AtomicBitmap,
    at::At,
    mode::{
        cooperative::Cooperative,
        exclusive::{Exclusive, Outcome, Reason},
    },
    typeutil::{BitMapUsize, InBound},
};

use crate::slotted::state::{InitializationState, ReserveState, SlotState};

/// A thread-safe arena for allocation and deallocation of values of type `T`.
pub struct Arena<T, const N: usize>
where
    BitMapUsize<N>: InBound,
{
    /// The state of the arena.
    ///
    /// This is used to keep track of the allocation state of the downstream
    /// storage array.
    arena_state: SlotState<N>,

    /// The storage for the arena.
    ///
    /// The elements in this array are initialized as per each set bit in the
    /// arena state.
    arena_storage: [UnsafeCell<MaybeUninit<T>>; N],
}

impl<T, const N: usize> Arena<T, N>
where
    BitMapUsize<N>: InBound,
{
    /// Construct a new `N`-element arena.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        let arena_state = SlotState::zeroed();

        let arena_storage = [const { UnsafeCell::new(MaybeUninit::uninit()) }; N];

        Self {
            arena_state,
            arena_storage,
        }
    }
}

impl<T, const N: usize> Default for Arena<T, N>
where
    BitMapUsize<N>: InBound,
{
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> Arena<T, N>
where
    BitMapUsize<N>: InBound,
{
    /// Acquire a reservation for a new element in the arena.
    ///
    /// This permits deferring the allocation of the element until it is
    /// actually needed.
    #[inline]
    pub fn reserve(&self) -> Option<Reserve<'_>> {
        /// Find the lowest zeroed bit in the target [`usize`].
        #[inline]
        const fn lowest_zero(target_value: usize) -> Option<u32> {
            const USIZE_BITS: u32 = usize::BITS;

            match usize::trailing_ones(target_value) {
                // NOTE: A value with `K` trailing ones has its lowest zeroed
                // bit exactly at index `K`.
                lowest_index @ 0..USIZE_BITS => Some(lowest_index),
                // NOTE: Every bit is set.
                USIZE_BITS.. => None,
            }
        }

        let Self { arena_state, .. } = self;

        let reserve_state = SlotState::reserve(arena_state);

        let mut target_state = Retry::unlimited(Backoff::minimal());

        loop {
            match AtomicBitmap::at_predicate(ReserveState::bitmap(reserve_state), lowest_zero)
                .filter(SlotState::<N>::filter)
            {
                Some(ref at_bit) => match at_bit.one_with::<Exclusive>(&mut target_state) {
                    Outcome::Success(..) => {
                        break Some(Reserve(reserve_state, At::index(at_bit)));
                    }
                    Outcome::Failure(Reason::Contended | Reason::Unchanged, ..) => (),
                    Outcome::Failure(Reason::Limited, ..) => {
                        unreachable!()
                    }
                },
                None => break None,
            }
        }
    }

    /// Determine the storage slot for the specified reservation.
    ///
    /// This will return `None` if the reservation is not for this arena.
    #[inline]
    pub fn slot(&self, reserve: &Reserve) -> Option<&UnsafeCell<MaybeUninit<T>>> {
        let Self {
            arena_state,
            arena_storage,
            ..
        } = self;

        let &Reserve(reserve_bitmap, reserve_index) = reserve;

        ptr::eq(
            ReserveState::bitmap(SlotState::reserve(arena_state)),
            ReserveState::bitmap(reserve_bitmap),
        )
        .then(|| arena_storage.get(reserve_index as usize))
        .flatten()
    }

    /// Attempt to allocate into this arena.
    ///
    /// # Errors
    ///
    /// Returns the original value when every arena slot is reserved.
    #[inline]
    pub fn allocate(&self, value: T) -> Result<InArena<'_, T>, T> {
        let Self {
            arena_storage,
            arena_state,
            ..
        } = self;

        match Self::reserve(self) {
            Some(target_reserve) => {
                let storage_index = target_reserve.index() as usize;

                // SAFETY:
                //
                // The slot index is guaranteed to be in-bounds due
                // to a preliminary check.
                //
                // Due to the previous compare-and-swap operation,
                // it is ensured that
                // the current thread has exclusive access
                // to the pointer, and, since the pointer is sourced
                // from an `UnsafeCell`,
                // casting to a mutable reference is valid.
                //
                // The `unwrap_unchecked` call is also deemed valid,
                // as a pointer sourced
                // from `UnsafeCell` is deemed to be always be
                // non-null.
                let value = unsafe {
                    arena_storage
                        .get_unchecked(storage_index)
                        .get()
                        .as_mut()
                        .unwrap_unchecked()
                        .write(value)
                };

                let _ = InitializationState::bitmap(SlotState::initialization(arena_state))
                    .at(target_reserve.index())
                    .one::<Cooperative>();

                Ok(InArena(
                    MaybeUninit::new(value),
                    target_reserve,
                    SlotState::initialization(arena_state),
                ))
            }
            None => Err(value),
        }
    }
}

// SAFETY: Stored values may move with the arena only when `T` is safe to send.
unsafe impl<T, const N: usize> Send for Arena<T, N>
where
    T: Send,
    BitMapUsize<N>: InBound,
{
}

// SAFETY: Atomic slot ownership serializes access, and moving values between
// threads is only permitted when `T` is safe to send.
unsafe impl<T, const N: usize> Sync for Arena<T, N>
where
    T: Send,
    BitMapUsize<N>: InBound,
{
}

impl<T, const N: usize> Drop for Arena<T, N>
where
    BitMapUsize<N>: InBound,
{
    fn drop(&mut self) {
        let &mut Self {
            ref arena_state,
            ref mut arena_storage,
        } = self;

        // NOTE: An `InArena` handle drops its value and clears its bits, so any
        // initialization bit that remains set here belongs to a forgotten
        // handle, whose value is dropped now.
        let init_snapshot = arena_state.initialization().snapshot();

        for slot_index in 0..N {
            let slot_mask = 1usize << slot_index;

            if (init_snapshot & slot_mask) != 0 {
                // SAFETY: The initialization bit guarantees this slot contains
                // a valid value.
                unsafe {
                    let target_slot = arena_storage.get_unchecked_mut(slot_index).get_mut();

                    let _ = target_slot.assume_init_read();
                }
            }
        }
    }
}

/// A reservation in the [`Arena`].
///
/// This can be used to to nonpreemptively reserve a [`MaybeUninit`]-based slot.
pub struct Reserve<'a>(ReserveState<'a>, u32);

impl Reserve<'_> {
    /// Determine the array index this allocation is for.
    #[inline]
    #[must_use]
    pub const fn index(&self) -> u32 {
        let &Self(.., target_index) = self;

        target_index
    }
}

impl Drop for Reserve<'_> {
    fn drop(&mut self) {
        let &mut Self(reserve_state, alloc_index, ..) = self;

        let _ = ReserveState::bitmap(reserve_state)
            .at(alloc_index)
            .zero::<Cooperative>();
    }
}

/// A handle to an active allocation in a [`Arena`].
pub struct InArena<'a, T>(
    // NOTE(invariant):
    //  This is `Some` and valid as long as allocation lives and the handle
    // hasn't been dropped.
    //
    // FIXME: Make this a MaybeUninit and just take up the unsafe access cost. We can have 2 niches
    // to exploit.
    MaybeUninit<&'a mut T>,
    Reserve<'a>,
    // NOTE(invariant): This belongs to the same `SlotState` as the `Reserve`.
    InitializationState<'a>,
);

impl<T> InArena<'_, T> {
    /// Access the data backed by this handle.
    #[inline]
    #[must_use]
    pub const fn data(&self) -> &T {
        let Self(target_data, ..) = self;

        // SAFETY: The `MaybeUninit` is always initialized for the lifetime of the
        // `InArena` type.
        unsafe { target_data.assume_init_ref() }
    }

    /// Access the data backed by this handle, in a mutable manner.
    #[inline]
    #[must_use]
    pub const fn data_mut(&mut self) -> &mut T {
        let &mut Self(ref mut target_data, ..) = self;

        // SAFETY: The `MaybeUninit` is always initialized for the lifetime of the
        // `InArena` type.
        unsafe { target_data.assume_init_mut() }
    }

    /// Access the allocation information for this handle.
    #[inline]
    #[must_use]
    pub const fn allocation(&self) -> &Reserve<'_> {
        let Self(_, allocation, ..) = self;

        allocation
    }
}

impl<T> Drop for InArena<'_, T> {
    #[inline]
    fn drop(&mut self) {
        let &mut Self(ref mut target_value, ref target_reserve, initialization_state) = self;

        // SAFETY: The `MaybeUninit` is always initialized for the lifetime of the
        // `InArena` type.
        let target_value = unsafe { MaybeUninit::assume_init_read(target_value) };

        // SAFETY: This handle exclusively owns the value in the slot, and the
        // slot cannot be reused until the initialization and reservation bits
        // are cleared afterwards.
        unsafe { ptr::drop_in_place(ptr::from_mut(target_value)) };

        // NOTE: This mirrors the slotted teardown order: the initialization
        // bit is cleared first, and the reservation bit is cleared by the
        // `Reserve` field drop that follows.
        let _ = InitializationState::bitmap(initialization_state)
            .at(Reserve::index(target_reserve))
            .zero::<Cooperative>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserve_reuses_lowest_free_slot() {
        let arena: Arena<u32, 8> = Arena::new();

        let first = arena.allocate(1).expect("arena has free slots");
        let second = arena.allocate(2).expect("arena has free slots");
        let third = arena.allocate(3).expect("arena has free slots");

        // Free the middle slot, fragmenting the reservation bitmap.
        drop(second);

        // This used to livelock: the lowest-zero scan skipped past the freed
        // slot onto an already-reserved bit and retried forever.
        let refill = arena.allocate(4).expect("freed slot is reusable");

        assert_eq!(*InArena::data(&refill), 4);

        drop((first, third, refill));
    }

    #[test]
    fn drop_runs_value_destructors() {
        use core::sync::atomic::{AtomicUsize, Ordering};

        static DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

        struct Counted;

        impl Drop for Counted {
            fn drop(&mut self) {
                DROP_COUNT.fetch_add(1, Ordering::SeqCst);
            }
        }

        let arena: Arena<Counted, 4> = Arena::new();

        let Ok(held) = arena.allocate(Counted) else {
            unreachable!("arena has free slots")
        };
        let Ok(leaked) = arena.allocate(Counted) else {
            unreachable!("arena has free slots")
        };

        // Dropping the handle used to be a no-op on the stored value.
        drop(held);
        assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 1);

        // A forgotten handle leaves its value for the arena to clean up.
        core::mem::forget(leaked);

        drop(arena);
        assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 2);
    }
}

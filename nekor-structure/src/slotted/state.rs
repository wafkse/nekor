//! The internal state of a slotted data structure.

use core::ops::Deref;

use nekor_sync::atomic::bitmap::{
    AtomicBitmap,
    at::At,
    typeutil::{BitMapUsize, InBound},
};

/// A runtime slot index validated against a slotted structure width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
// NOTE(invariant): The stored bit index is less than the width of the slot state that produced it.
pub(in crate::slotted) struct SlotIndex(u32);

impl SlotIndex {
    /// Validates the engaged bit index against an `N`-slot state.
    #[inline]
    pub(in crate::slotted) fn from_at<const N: usize>(at: &At<'_>) -> Option<Self>
    where
        BitMapUsize<N>: InBound,
    {
        let target_index = At::index(at);

        usize::try_from(target_index)
            .ok()
            .filter(|&target_index| target_index < N)
            .map(|_| Self(target_index))
    }

    /// Returns the bitmap bit index.
    #[inline]
    pub(in crate::slotted) const fn bit(self) -> u32 {
        let Self(target_index) = self;

        target_index
    }

    /// Returns the storage array index.
    #[inline]
    pub(in crate::slotted) fn array(self) -> usize {
        let Self(target_index) = self;

        match usize::try_from(target_index) {
            Ok(target_index) => target_index,
            Err(..) => unreachable!(),
        }
    }
}

/// A type to represent a the *reserve state* in a [`SlotState`].
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
// NOTE(invariant): The referenced bitmap is the reservation bitmap of its owning slot state.
pub struct ReserveState<'a>(&'a AtomicBitmap);

impl<'a> ReserveState<'a> {
    /// Access the [`AtomicBitmap`] contained within this [`ReserveState`].
    #[inline]
    #[must_use]
    pub const fn bitmap(self) -> &'a AtomicBitmap {
        let Self(target_bitmap) = self;

        target_bitmap
    }
}

impl Deref for ReserveState<'_> {
    type Target = AtomicBitmap;

    #[inline]
    fn deref(&self) -> &Self::Target {
        let &Self(target_bitmap) = self;

        target_bitmap
    }
}

/// A type to represent a the *initialization state* in a [`SlotState`].
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
// NOTE(invariant): The referenced bitmap is the initialization bitmap of its owning slot state.
pub struct InitializationState<'a>(&'a AtomicBitmap);

impl<'a> InitializationState<'a> {
    /// Access the [`AtomicBitmap`] contained within this
    /// [`InitializationState`].
    #[inline]
    #[must_use]
    pub const fn bitmap(self) -> &'a AtomicBitmap {
        let Self(target_bitmap) = self;

        target_bitmap
    }
}

impl Deref for InitializationState<'_> {
    type Target = AtomicBitmap;

    #[inline]
    fn deref(&self) -> &Self::Target {
        let &Self(target_bitmap) = self;

        target_bitmap
    }
}

/// A self-contained lock-free slot state.
///
/// This imposes controlled, concurrent access to a higher-level construct's
/// *storage array*.
///
/// The slot state is composed of two `N`-bit atomic bitmaps, a *reservation
/// state*, and an *initialization state*, where `N` corresponds to a nonzero
/// positive integer less than or equal to [`usize::BITS`].
///
/// Each bitmap indicates the downstream state of the *storage array*,
/// particularly:
///     - The *Reservation State*: Indicates the mutable, mutually exclusive state of the `I`th slot
///       in the storage array. This can be *relied upon in unsafe code*, as it is a strong
///       *invariant* for the same `(slot-state, storage-array)` 2-tuple.
///     - The *Initialization State*: Indicates the initialization and availability of the `I`th
///       slot in the storage array. Storage slot availability implies that any future dequeue
///       operation may acquire and *consume* the value in the downstream slot `I` in the storage
///       array. This can be *relied upon in unsafe code*, as it is a strong *invariant* for the
///       same `(slot-state, storage-array)` 2-tuple.
///
/// The downstream *storage array* must be exactly `N`-elements long.
#[derive(Debug)]
// NOTE(invariant): Every initialized slot is also reserved in the paired reservation bitmap.
pub struct SlotState<const N: usize>
where
    BitMapUsize<N>: InBound,
{
    /// The reservation state of the lock-free slot state.
    ///
    /// In this bitmap, a set `I`th bit implies mutable, mutual exclusive access
    /// to the downstream slot `I` in the storage array.
    reserve_state: AtomicBitmap,

    /// The initialization state of the lock-free slot state.
    ///
    /// In this bitmap, a set `I`th bit implies the initialization of the value
    /// in the downstream slot `I` in the storage array.
    initialization_state: AtomicBitmap,
}

impl<const N: usize> SlotState<N>
where
    BitMapUsize<N>: InBound,
{
    /// Zero-initialize a brand-new [`SlotState`] with no elements reserved nor
    /// initialized.
    #[inline]
    #[must_use]
    pub const fn zeroed() -> Self {
        let (reserve_state, initialization_state) = (AtomicBitmap::zeroed(), AtomicBitmap::zeroed());

        Self {
            reserve_state,
            initialization_state,
        }
    }
}

impl<const N: usize> SlotState<N>
where
    BitMapUsize<N>: InBound,
{
    /// Determine the *reserve state* of this [`SlotState`].
    #[inline]
    #[must_use]
    pub const fn reserve(&self) -> ReserveState<'_> {
        let Self { reserve_state, .. } = self;

        ReserveState(reserve_state)
    }

    /// Determine the *initialization state* of this [`SlotState`].
    #[inline]
    #[must_use]
    pub const fn initialization(&self) -> InitializationState<'_> {
        let Self {
            initialization_state, ..
        } = self;

        InitializationState(initialization_state)
    }
}

impl<const N: usize> SlotState<N>
where
    BitMapUsize<N>: InBound,
{
    /// Validates an engaged bitmap bit as an index in this slot state.
    #[inline]
    pub(in crate::slotted) fn index(at: &At<'_>) -> Option<SlotIndex> {
        SlotIndex::from_at::<N>(at)
    }
}

use core::marker;

use crate::bit::Bitwise;

/// A newtype that validates that a bit indice [`Selected::0`] is indeed valid
/// for an integer of type `B`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
#[repr(transparent)]
pub struct Selected<B>(u32, marker::PhantomData<B>)
where
    B: Bitwise;

impl<B> Selected<B>
where
    B: Bitwise,
{
    /// Construct a [`Selected`] newtype for a target bit indice.
    ///
    /// # Panics
    ///
    /// This will panic if the provided indice is not within the bitwise bounds
    /// of the integer of type `B`.
    #[inline]
    #[must_use]
    pub const fn new(target_indice: u32) -> Self {
        Self::try_new(target_indice).expect("not within integer bitwise bounds")
    }

    /// Attempt to construct a [`Selected`] newtype for a target bit indice.
    #[inline]
    #[must_use]
    pub const fn try_new(target_indice: u32) -> Option<Self> {
        if target_indice < B::BITS {
            Some(Self(target_indice, marker::PhantomData))
        } else {
            None
        }
    }

    /// Determine the [`Selected`] for the bit that is leftwards to the selected
    /// one.
    #[inline]
    #[must_use]
    pub const fn left(&self) -> Option<Self> {
        let &Self(target_indice, ..) = self;

        if let Some(target_indice) = target_indice.checked_add(1) {
            Self::try_new(target_indice)
        } else {
            None
        }
    }

    /// Determine the [`Selected`] for the bit that is leftwards to the selected
    /// one.
    #[inline]
    #[must_use]
    pub const fn right(&self) -> Option<Self> {
        let &Self(target_indice, ..) = self;

        if let Some(target_indice) = target_indice.checked_sub(1) {
            Self::try_new(target_indice)
        } else {
            None
        }
    }

    /// Determine the bit indice of this [`Selected`].
    #[inline]
    #[must_use]
    pub const fn index(&self) -> u32 {
        let &Self(target_indice, ..) = self;

        target_indice
    }
}

/// A newtype that validates that a bit interval [`Interval::0`] is indeed valid
/// for an integer of type `B`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Interval<B>((u32, u32), marker::PhantomData<B>)
where
    B: Bitwise;

impl<B> Interval<B>
where
    B: Bitwise,
{
    /// Attempt to construct a [`Interval`] newtype for a target bit interval.
    #[inline]
    #[must_use]
    pub const fn new(range_start: u32, range_end: u32) -> Option<Self> {
        if let Some(..) = Selected::<B>::try_new(range_end) {
            Some(Self((range_start, range_end), marker::PhantomData))
        } else {
            None
        }
    }

    /// Determine the start of this [`Interval`].
    #[inline]
    #[must_use]
    pub const fn start(&self) -> u32 {
        let &Self((interval_start, ..), ..) = self;

        interval_start
    }

    /// Determine the end of this [`Interval`].
    #[inline]
    #[must_use]
    pub const fn end(&self) -> u32 {
        let &Self((.., interval_end), ..) = self;

        interval_end
    }
}

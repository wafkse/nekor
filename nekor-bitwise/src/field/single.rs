//! Single-bit bitwise fields for an already-existing integer.
//!
//! See [`Bit`] and [`BitMut`] for additional information.

use crate::{
    bit::{BitAt, BitAtExtractor, state::State},
    field::counterpart::Counterpart,
};

/// A managed immutable handle to the `N`-th bit in the type `I`.
#[derive(Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Bit<'a, I, const N: usize>(&'a I)
where
    I: BitAt<N>;

impl<'a, I, const N: usize> Counterpart for Bit<'a, I, N>
where
    I: BitAt<N>,
{
    type Mut = BitMut<'a, I, N>;

    type Immut = Self;
}

impl<'a, I, const N: usize> Bit<'a, I, N>
where
    I: BitAt<N>,
{
    /// Wrap the target value's bit in a [`Bit`] struct.
    #[inline]
    pub const fn wrap(target_value: &'a I) -> Self {
        Self(target_value)
    }
}

impl<I, const N: usize> Bit<'_, I, N>
where
    I: BitAt<N>,
{
    /// Determine the [`State`] of this [`Bit`].
    #[inline]
    #[must_use]
    pub fn state(&self) -> State {
        let &Self(target_value) = self;

        BitAt::<N>::get(target_value)
    }

    /// Instantiate a new copy of the underlying register but with the target
    /// bit cleared.
    #[inline]
    #[must_use]
    pub fn cleared(&self) -> I {
        let &Self(target_value) = self;

        BitAt::<N>::cleared(target_value)
    }

    /// Instantiate a new copy of the underlying register but with the target
    /// bit set.
    #[inline]
    #[must_use]
    pub fn enabled(&self) -> I {
        let &Self(target_value) = self;

        BitAt::<N>::enabled(target_value)
    }
}

/// A managed mutable handle to the `N`-th bit in the type `I`.
#[derive(Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct BitMut<'a, I, const N: usize>(&'a mut I)
where
    I: BitAt<N>;

impl<'a, I, const N: usize> BitMut<'a, I, N>
where
    I: BitAt<N>,
{
    /// Wrap the target value's bit in a [`BitMut`] struct.
    #[inline]
    pub const fn wrap(target_value: &'a mut I) -> Self {
        Self(target_value)
    }
}

impl<I, const N: usize> BitMut<'_, I, N>
where
    I: BitAt<N>,
{
    /// Determine the [`State`] of this [`BitMut`].
    #[inline]
    #[must_use]
    pub fn state(&self) -> State {
        let Self(target_value) = self;

        BitAt::<N>::get(*target_value)
    }

    /// Instantiate a new copy of the underlying register but with the target
    /// bit cleared.
    #[inline]
    #[must_use]
    pub fn cleared(&self) -> I {
        let &Self(ref target_value) = self;

        BitAt::<N>::cleared(target_value)
    }

    /// Instantiate a new copy of the underlying register but with the target
    /// bit set.
    #[inline]
    #[must_use]
    pub fn enabled(&self) -> I {
        let &Self(ref target_value) = self;

        BitAt::<N>::enabled(target_value)
    }

    /// Override the [`State`] of this [`BitMut`].
    #[inline]
    pub fn set(&mut self, target_state: State) -> State {
        let &mut Self(ref mut target_value) = self;

        BitAt::<N>::set(*target_value, target_state)
    }

    /// Toggle the [`State`] of this [`BitMut`].
    ///
    /// This yields the previous state to the caller.
    #[inline]
    pub fn toggle(&mut self) -> State {
        let &mut Self(ref mut target_value) = self;

        BitAt::<N>::toggle(*target_value)
    }
}

impl<'a, I, const N: usize> Counterpart for BitMut<'a, I, N>
where
    I: BitAt<N>,
{
    type Mut = Self;

    type Immut = Bit<'a, I, N>;
}

/// A helper macro to implement the distinct combinations of the [`BitMut`]
/// implementors.
macro_rules! bit_mut {
    () => {};
    (
        $(
            #[$target_meta:meta]
        )*

        $($target_type:ident),+ $(,)?
    ) => {
       $(
           impl<'a, const N: usize> BitMut<'a, $target_type, N>
           where
               $target_type: BitAt<N>,
           {
               /// Determine the [`State`] of this [`BitMut`].
               #[inline]
               pub const fn const_state(&self) -> State {
                   let &Self(ref target_value) = self;

                   BitAtExtractor::<N, $target_type>::get(*target_value)
               }

               /// Override the [`State`] of this [`BitMut`].
               #[inline]
               pub const fn const_set(&mut self, target_state: State) -> State {
                   let &mut Self(ref mut target_value) = self;

                   BitAtExtractor::<N, $target_type>::set(*target_value, target_state)
               }

               /// Toggle the [`State`] of this [`BitMut`].
               ///
               /// This yields the previous state to the caller.
               #[inline]
               pub const fn const_toggle(&mut self) -> State {
                   let &mut Self(ref mut target_value) = self;

                   BitAtExtractor::<N, $target_type>::toggle(*target_value)
               }
           }
       )+
    };
}

bit_mut!(u8, u16, u32, u64);

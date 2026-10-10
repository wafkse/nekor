//! Single-bit bitwise fields for an already-existing integer.
//!
//! See [`Bit`] and [`BitMut`] for additional information.

use crate::{
    bit::{BitAt, BitAtExtractor, state::State},
    field::counterpart::Counterpart,
};

/// A managed immutable handle to the `N`-th bit in the type `I`.
///
/// Equality and ordering compare the selected [`State`].
///
/// Indices outside the integer's width are rejected by the type bound.
///
/// ```compile_fail
/// use nekor_bitwise::field::single::Bit;
///
/// let _ = Bit::<u128, 128>::wrap(&u128::MIN);
/// ```
#[derive(Debug)]
#[repr(transparent)]
// NOTE(invariant): The `BitAt<N>` bound proves that `N` names a valid bit in the borrowed value.
pub struct Bit<'a, I, const N: usize>(&'a I)
where
    I: BitAt<N>;

impl<'a, I, const N: usize> Counterpart for Bit<'a, I, N>
where
    I: BitAt<N>,
{
    type Immut = Self;
    type Mut = BitMut<'a, I, N>;
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
///
/// Equality and ordering compare the selected [`State`], as with [`Bit`].
#[derive(Debug)]
#[repr(transparent)]
// NOTE(invariant): The `BitAt<N>` bound proves that `N` names a valid bit in the borrowed value.
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
        let Self(target_value) = self;

        BitAt::<N>::cleared(*target_value)
    }

    /// Instantiate a new copy of the underlying register but with the target
    /// bit set.
    #[inline]
    #[must_use]
    pub fn enabled(&self) -> I {
        let Self(target_value) = self;

        BitAt::<N>::enabled(*target_value)
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
    type Immut = Bit<'a, I, N>;
    type Mut = Self;
}

/// A helper macro to implement the distinct combinations of the [`BitMut`]
/// implementors.
macro_rules! mutable {
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

mutable!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);

/// Expand to a const-compatible implementation of [`Bit`].
macro_rules! constant {
    ($($target_type:ty),+ $(,)?) => {
        $(
            impl<'a, const N: usize> Bit<'a, $target_type, N>
            where
                $target_type: BitAt<N>,
            {
                /// Determine the [`State`] of this [`Bit`] in const-evaluable code.
                #[inline]
                #[must_use]
                pub const fn const_state(&self) -> State {
                    let &Self(target_value) = self;

                    BitAtExtractor::<N, $target_type>::get(target_value)
                }
            }
        )+
    };
}

constant!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);

/// Implement comparisons over a static bit's projected state.
macro_rules! compare {
    () => {};
    ($($target_type:ident),* $(,)?) => {
        $crate::reexport::tokel::stream! {
            $(
            impl<I, const N: usize> PartialEq for $target_type<'_, I, N>
            where
                I: BitAt<N>,
            {
                #[inline]
                fn eq(&self, target_value: &Self) -> bool {
                    self.state() == target_value.state()
                }
            }

            impl<I, const N: usize> Eq for $target_type<'_, I, N> where I: BitAt<N> {}

            impl<I, const N: usize> Ord for $target_type<'_, I, N>
            where
                I: BitAt<N>,
            {
                #[inline]
                fn cmp(&self, target_value: &Self) -> core::cmp::Ordering {
                    self.state().cmp(&target_value.state())
                }
            }

            impl<I, const N: usize> PartialOrd for $target_type<'_, I, N>
            where
                I: BitAt<N>,
            {
                #[inline]
                fn partial_cmp(&self, target_value: &Self) -> Option<core::cmp::Ordering> {
                    Some(self.cmp(target_value))
                }
            }

            )*
        }


    };
}

compare![Bit, BitMut];

#[cfg(test)]
mod tests {
    //! Test projected states and const operations across integer widths.

    use super::{Bit, BitMut};
    use crate::prelude::State;

    /// Bit comparisons use their projected states.
    #[test]
    fn projected_comparisons() {
        let left = 0x01_u8;
        let right = 0xff_u8;

        assert_eq!(Bit::<u8, 0>::wrap(&left), Bit::<u8, 0>::wrap(&right));
        assert!(Bit::<u8, 0>::wrap(&0x02) < Bit::<u8, 0>::wrap(&0x01));

        let mut left = left;
        let mut right = right;

        assert_eq!(BitMut::<u8, 0>::wrap(&mut left), BitMut::<u8, 0>::wrap(&mut right));
    }

    /// Generate const-evaluated highest-bit checks for each primitive.
    macro_rules! highest {
        () => {};
        ($($target_type:ident),* $(,)?) => {
            crate::reexport::tokel::stream! {
            {
                $(
                {
                    const N: usize = $target_type::BITS as usize - 1;
                    const VALUE: ($target_type, State, State, State, State) = {
                        let mut value: $target_type = 0;
                        let mut bit = BitMut::<$target_type, N>::wrap(&mut value);

                        let before = bit.const_set(State::Set);
                        let after = bit.const_state();
                        let previous = bit.const_toggle();
                        let cleared = bit.const_state();

                        (value, before, after, previous, cleared)
                    };

                    assert_eq!(VALUE, (0, State::Cleared, State::Set, State::Set, State::Cleared));
                    assert_eq!(Bit::<$target_type, N>::wrap(&$target_type::MAX).const_state(),
                        if $target_type::MIN == 0 { State::Set } else { State::Cleared });
                }

                )*
            }
        }


        };
    }

    /// Mutable and immutable const operations reach each primitive's highest bit.
    #[test]
    fn primitive_const_parity() {
        highest!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);
    }
}

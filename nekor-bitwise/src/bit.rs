//! Traits and generic types for working with bits.

pub mod state;

use core::marker;

use nekor_primitive::scalar::Scalar;

use crate::{bit::state::State, field::dynamic::select::Selected};

/// A trait that serves as metadata for a target bitwise type.
pub trait Bitwise: Scalar {
    /// The number of bits that this type is composed of.
    const BITS: u32;
}

/// A trait that permits mutation of selected bits inside an integer.
pub trait BitOp: Bitwise {
    /// Determine the [`State`] of the target bit.
    fn get(&self, target_indice: Selected<Self>) -> State;

    /// Set the [`State`] of the target bit.
    ///
    /// Yields back the previous state of the bit.
    fn set(&mut self, target_indice: Selected<Self>, target_state: State) -> State;

    /// Instantiate a new value of this type with the only target bit set.
    fn single(target_indice: Selected<Self>) -> Self;

    /// Toggle the state of the target bit.
    ///
    /// Yields back the previous state of the bit.
    #[inline]
    fn toggle(&mut self, target_indice: Selected<Self>) -> State {
        let target_state = self.get(target_indice);

        match target_state {
            State::Set => self.set(target_indice, State::Cleared),
            State::Cleared => self.set(target_indice, State::Set),
        }
    }

    /// Copy this value, but with the target bit toggled.
    #[inline]
    fn toggled(&self, target_indice: Selected<Self>) -> Self {
        let mut target_copy = *self;

        target_copy.toggle(target_indice);

        target_copy
    }

    /// Copy this value, but with the target bit cleared.
    #[inline]
    fn cleared(&self, target_indice: Selected<Self>) -> Self {
        let mut target_copy = *self;

        target_copy.set(target_indice, State::Cleared);

        target_copy
    }

    /// Copy this value, but with the target bit set.
    #[inline]
    fn enabled(&self, target_indice: Selected<Self>) -> Self {
        let mut target_copy = *self;

        target_copy.set(target_indice, State::Set);

        target_copy
    }
}

/// A trait that permits mutation of a bit `N` that has been selected at
/// compile-time.
pub trait BitAt<const N: usize>: Bitwise {
    /// Determine the [`State`] of the target bit.
    fn get(&self) -> State;

    /// Set the [`State`] of the target bit.
    ///
    /// Yields back the previous state of the bit.
    fn set(&mut self, target_state: State) -> State;

    /// Instantiate a new value of this type with the only target bit set.
    fn single() -> Self;

    /// Toggle the state of the target bit.
    ///
    /// Yields back the previous state of the bit.
    #[inline]
    fn toggle(&mut self) -> State {
        let target_state = self.get();

        match target_state {
            State::Set => self.set(State::Cleared),
            State::Cleared => self.set(State::Set),
        }
    }

    /// Copy this value, but with the target bit toggled.
    #[inline]
    fn toggled(&self) -> Self {
        let mut target_copy = *self;

        target_copy.toggle();

        target_copy
    }

    /// Copy this value, but with the target bit cleared.
    #[inline]
    fn cleared(&self) -> Self {
        let mut target_copy = *self;

        target_copy.set(State::Cleared);

        target_copy
    }

    /// Copy this value, but with the target bit set.
    #[inline]
    fn enabled(&self) -> Self {
        let mut target_copy = *self;

        target_copy.set(State::Set);

        target_copy
    }
}

/// A helper type to make use of the [`BitAt`] trait const-fn compatible.
pub struct BitAtExtractor<const N: usize, P>(marker::PhantomData<P>)
where
    P: BitAt<N>;

/// Macro to help implement the various traits and associated functions required
/// for crate functionality.
macro_rules! bits {
    () => {};
    (
        [$($target_index:literal),*] for $target_type:ty
    ) => {
        impl Bitwise for $target_type {
            const BITS: u32 = Self::BITS;
        }

        impl BitOp for $target_type
        where
            Self: Scalar,
        {
            #[inline]
            fn single(target_indice: Selected<Self>) -> Self {
                1 << target_indice.index()
            }

            #[inline]
            fn set(&mut self, target_indice: Selected<Self>, target_state: State) -> State {

                let set_state = <Self as BitOp>::get(self, target_indice);

                match target_state {
                    State::Set => {
                        *self = (*self | <Self as BitOp>::single(target_indice));
                    }

                    State::Cleared => {
                        *self = (*self & !<Self as BitOp>::single(target_indice));
                    }
                }

                set_state
            }

            #[inline]
            fn get(&self, target_indice: Selected<Self>) -> State {
                let target_bit = *self & <Self as BitOp>::single(target_indice);

                match target_bit {
                    0 => State::Cleared,
                    _ => State::Set,
                }
            }
        }

        impl<const N: usize> BitAtExtractor<N, $target_type>
            where $target_type: BitAt<N>
        {
            const TARGET_MASK: $target_type = 1 << N;

            /// A const-fn version of the [`Bits::single`] associated function.
            #[inline]
            pub const fn single() -> $target_type {
                Self::TARGET_MASK
            }

            /// A const-fn version of the [`Bits::set`] associated function.
            #[inline]
            pub const fn set(target_value: &mut $target_type, target_state: State) -> State {
                let set_state = Self::get(target_value);

                match target_state {
                    State::Set => {
                        *target_value = (*target_value | Self::TARGET_MASK);
                    }

                    State::Cleared => {
                        *target_value = (*target_value & !Self::TARGET_MASK);
                    }
                }

                set_state
            }

            /// A const-fn version of the [`Bits::get`] associated function.
            #[inline]
            pub const fn get(target_value: &$target_type) -> State {
                let target_bit = *target_value & Self::TARGET_MASK;

                match target_bit {
                    0 => State::Cleared,
                    _ => State::Set,
                }
            }

            /// A const-fn version of the [`Bits::toggle`] associated function.
            #[inline]
            pub const fn toggle(target_value: &mut $target_type) -> State {
                let ref target_state = Self::get(target_value);

                Self::set(target_value, State::complement(target_state))
            }
        }

        $(
            impl BitAt<{ $target_index }> for $target_type
            where
                Self: Scalar,
            {
                #[inline]
                fn single() -> Self {
                    BitAtExtractor::<$target_index, $target_type>::single()
                }

                #[inline]
                fn set(&mut self, target_state: State) -> State {
                    BitAtExtractor::<$target_index, $target_type>::set(self, target_state)
                }

                #[inline]
                fn get(&self) -> State {
                    BitAtExtractor::<$target_index, $target_type>::get(self)
                }
            }
        )+
    };
}

bits!([0, 1, 2, 3, 4, 5, 6, 7] for u8);

bits!([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15] for u16);

bits!([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31] for u32);

bits!([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
    32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63] for u64);

#[cfg(target_pointer_width = "16")]
bits!([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15] for usize);

#[cfg(target_pointer_width = "32")]
bits!([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31] for usize);

#[cfg(target_pointer_width = "64")]
bits!([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
        32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63] for usize);

bits!([0, 1, 2, 3, 4, 5, 6, 7] for i8);

bits!([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15] for i16);

bits!([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31] for i32);

bits!([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
    32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63] for i64);

#[cfg(target_pointer_width = "16")]
bits!([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15] for isize);

#[cfg(target_pointer_width = "32")]
bits!([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31] for isize);

#[cfg(target_pointer_width = "64")]
bits!([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
    32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63] for isize);

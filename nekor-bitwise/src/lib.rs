#![no_std]
#![recursion_limit = "384"]

//! Various utilities to work with singular or sequences of bits.

pub mod field;

pub mod bit;

pub mod endian;

pub mod prelude {
    //! The prelude module for the `nekor-bitwise` crate.

    pub use nekor_bitwise_extract::{Extract, Extractor};
    pub use nekor_bitwise_size::{For, For2, Size};

    pub use crate::{
        bit::{BitAt, state::State},
        endian::{Be, Le},
        field::{
            counterpart::Counterpart,
            dynamic::{
                multi::{FieldDyn, FieldDynMut},
                select::{Interval, Selected},
                single::{BitDyn, BitDynMut},
            },
            multi::{Field, FieldMut},
            single::{Bit, BitMut},
        },
    };
}

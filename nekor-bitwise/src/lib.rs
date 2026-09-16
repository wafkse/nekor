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
            multi::{
                Field, FieldMut, U8High4, U8High4Mut, U8Low4, U8Low4Mut, U16High8, U16High8Mut, U16Low8, U16Low8Mut,
                U32High16, U32High16Mut, U32Low16, U32Low16Mut, U64High32, U64High32Mut, U64Low32, U64Low32Mut,
                U128High64, U128High64Mut, U128Low64, U128Low64Mut,
            },
            single::{Bit, BitMut},
        },
    };
}

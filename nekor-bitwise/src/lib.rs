#![no_std]
#![recursion_limit = "384"]
#![forbid(
    clippy::all,
    clippy::perf,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::panic,
    clippy::pedantic,
    rustdoc::all
)]
//! Various utilities to work with singular or sequences of bits.

pub mod field;

pub mod bit;

pub mod endian;

pub mod prelude {
    //! The prelude module for the `nekor-bitwise` crate.

    pub use crate::{
        bit::{BitAt, state::State},
        endian::{Be, Le},
        field::{
            counterpart::Counterpart,
            dynamic::{
                select::{Interval, Selected},
                single::{BitDyn, BitDynMut},
            },
            multi::{Field, FieldMut},
            single::{Bit, BitMut},
        },
    };

    pub use nekor_bitwise_extract::{Extract, Extractor};

    pub use nekor_bitwise_size::{For, For2, Size};
}

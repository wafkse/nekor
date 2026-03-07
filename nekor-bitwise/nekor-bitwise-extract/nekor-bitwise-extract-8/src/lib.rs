#![no_std]
#![forbid(
    clippy::all,
    clippy::perf,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::panic,
    clippy::pedantic,
    unsafe_code,
    rustdoc::all
)]
#![doc = include_str!("../../README.md")]

pub mod downstream {
    //! The downstream metadata this crate unificates.

    pub use nekor_bitwise_extract_8_8::MetadataU8U8;
}

use crate::downstream::MetadataU8U8;

use nekor_bitwise_extract_core::delegate;

delegate!(become u8 as MetadataU8 for [u8 become MetadataU8U8]);

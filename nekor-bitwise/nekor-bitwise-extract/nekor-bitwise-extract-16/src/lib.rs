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

    pub use nekor_bitwise_extract_16_8::MetadataU16U8;
    pub use nekor_bitwise_extract_16_16::MetadataU16U16;
}

use nekor_bitwise_extract_core::delegate;

use crate::downstream::{MetadataU16U8, MetadataU16U16};

delegate!(become u16 as MetadataU16 for [u8 become MetadataU16U8, u16 become MetadataU16U16]);

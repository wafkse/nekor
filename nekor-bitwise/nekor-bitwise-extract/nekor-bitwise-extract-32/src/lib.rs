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

    pub use nekor_bitwise_extract_32_8::MetadataU32U8;
    pub use nekor_bitwise_extract_32_16::MetadataU32U16;
    pub use nekor_bitwise_extract_32_32::MetadataU32U32;
}

use nekor_bitwise_extract_core::delegate;

use crate::downstream::{MetadataU32U8, MetadataU32U16, MetadataU32U32};

delegate!(become u32 as MetadataU32 for [u8 become MetadataU32U8, u16 become MetadataU32U16, u32 become MetadataU32U32]);

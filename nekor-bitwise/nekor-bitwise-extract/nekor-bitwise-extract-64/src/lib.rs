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

    pub use nekor_bitwise_extract_64_8::MetadataU64U8;
    pub use nekor_bitwise_extract_64_16::MetadataU64U16;
    pub use nekor_bitwise_extract_64_32::MetadataU64U32;
    pub use nekor_bitwise_extract_64_64::MetadataU64U64;
}

use nekor_bitwise_extract_core::delegate;

use crate::downstream::{MetadataU64U8, MetadataU64U16, MetadataU64U32, MetadataU64U64};

delegate!(become u64 as MetadataU64 for [u8 become MetadataU64U8, u16 become MetadataU64U16, u32 become MetadataU64U32, u64 become MetadataU64U64]);

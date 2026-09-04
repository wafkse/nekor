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

    pub use nekor_bitwise_extract_128_8::MetadataU128U8;
    pub use nekor_bitwise_extract_128_16::MetadataU128U16;
    pub use nekor_bitwise_extract_128_32::MetadataU128U32;
    pub use nekor_bitwise_extract_128_64::MetadataU128U64;
    pub use nekor_bitwise_extract_128_128::MetadataU128U128;
}

use nekor_bitwise_extract_core::delegate;

use crate::downstream::{MetadataU128U8, MetadataU128U16, MetadataU128U32, MetadataU128U64, MetadataU128U128};

delegate!(become u128 as MetadataU128 for [
    u8 become MetadataU128U8,
    u16 become MetadataU128U16,
    u32 become MetadataU128U32,
    u64 become MetadataU128U64,
    u128 become MetadataU128U128
]);

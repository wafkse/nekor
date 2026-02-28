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

use nekor_bitwise_extract_core::{implement, metadata};

metadata!(
    become for u128 -> u128 as MetadataU128U128
);

implement!(for u128 -> u128 as MetadataU128U128);

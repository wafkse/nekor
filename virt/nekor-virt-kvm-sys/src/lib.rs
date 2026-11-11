#![doc = include_str!("../README.md")]

#[cfg(any(feature = "stable", feature = "lts"))]
pub mod ffi;

#[cfg(any(feature = "stable", feature = "lts"))]
mod generated;

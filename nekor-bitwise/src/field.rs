//! Multi-bit and single-bit bitwise fields that are either known at
//! compile-time or runtime.
//!
//! For runtime adapters over bits, see the [`dynamic`] module.

pub mod multi;

pub mod single;

pub mod counterpart;

pub mod dynamic;

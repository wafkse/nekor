#![forbid(
    clippy::all,
    clippy::perf,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::panic,
    clippy::pedantic,
    rustdoc::all
)]
//! The build orchestration crate for the Nekor Unikernel.

pub mod orchestrate;

pub mod platform;

pub mod invoke;

pub mod manifest;

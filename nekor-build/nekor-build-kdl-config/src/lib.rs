#![forbid(
    clippy::all,
    clippy::perf,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::panic,
    clippy::pedantic,
    rustdoc::all
)]
//! Typed KDL configuration support for build tooling.

pub mod annotation;
pub mod document;
pub mod error;
pub mod merge;
pub mod name;
pub mod node;
pub mod overlay;
pub mod prelude;
pub mod scalar;
pub mod source;

mod parse;
mod serialize;

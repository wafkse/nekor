#![no_std]
#![forbid(
    clippy::all,
    clippy::perf,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::panic,
    clippy::pedantic,
    rustdoc::all
)]
//! Machine primitive values for the Nekor Operating System.

pub mod identity;

pub mod scalar;

pub mod prelude {
    //! The prelude module provides convenient imports for common types and
    //! traits.

    pub use crate::scalar::Scalar;

    pub use crate::identity::{One, Zero};
}

#![cfg_attr(not(any(test, miri, usermode)), no_std)]
#![forbid(
    clippy::all,
    clippy::perf,
    clippy::unwrap_used,
    clippy::panic,
    clippy::pedantic,
    rustdoc::all
)]
#![deny(clippy::nursery)]
//! Architecture-level interprocessor signaling.

pub mod arch;

pub mod monitor;

pub mod prelude {
    //! A prelude for the *Architecture Abstraction Layer* inter-agent signaling
    //! and communication.
    //!
    //! This includes the most commonly used types and traits.

    pub use crate::monitor::{Monitor, Monitored};
}

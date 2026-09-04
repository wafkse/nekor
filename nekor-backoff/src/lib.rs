#![cfg_attr(not(any(test, miri, usermode)), no_std)]
//! Kernel-wide backoff and retry control for contended operations.
//!
//! This crate provides utilities for managing retry strategies in lock-free
//! and contended synchronization scenarios. It includes exponential backoff
//! algorithms and retry attempt limiting.
//!
//! # Core Components
//!
//! The core components of this crate are:
//!
//! * [`Backoff`]: for cycle-based controlled backoff
//! * [`Limit`]: for well-defined self-imposed limits

pub mod backoff;

pub mod limit;

pub mod retry;

pub mod prelude {
    //! A prelude for the `nekor-backoff` crate.
    //!
    //! This re-exports the most important items and modules.

    pub use crate::{
        backoff::{Backoff, BackoffState, ExponentialBase},
        limit::Limit,
        retry::Retry,
    };
}

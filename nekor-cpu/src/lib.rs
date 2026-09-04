#![no_std]

//! Utilities for working with per-cpu data.

mod domain;

pub mod limit;

pub mod processor;

pub mod local;

pub mod prelude {
    pub use crate::{
        limit::Cores,
        local::{Local, PerCpu},
        processor::{CoreId, Handout, Reutilization},
    };
}

#![no_std]
#![doc = include_str!("../README.md")]

extern crate alloc;

pub mod baseline;

pub mod known;

pub mod prelude {
    //! Exports the most commonly used items in the `nekor-virt-model` crate.

    pub use crate::{
        baseline::Arch,
        known::{Aarch64, X86_64},
    };
}

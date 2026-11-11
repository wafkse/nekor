#![no_std]
#![doc = include_str!("../README.md")]

pub mod baseline;

pub mod prelude {
    //! Exports the most commonly used items in the `eustace-hypervisor` crate.

    pub use crate::baseline::Backend;
}

//! The baseline module for the `nekor-virt-model` crate.
//!
//! This is used for baseline traits and other items that are required for architectural mododeling.

/// A marker that identifies a particular Instruction Set Architecture.
pub trait Arch: 'static {}

#![cfg_attr(not(any(test, miri, usermode)), no_std)]

//! Compile-time hardware register descriptions.
//!
//! A register combines a value type, a numeric address, and an access gate.
//! The address is deliberately uninterpreted here. MMIO code may treat it as a
//! memory location while an architecture layer may treat it as a register
//! index.
//!
//! Direct volatile memory access remains available through [`memory::Memory`].

pub mod address;

pub mod memory;

pub mod mode;

pub mod prelude {
    //! The prelude module provides the most commonly used register descriptions
    //! and access mechanisms from this crate.

    pub use crate::{
        address::Address,
        memory::{Memory, Volatile},
        mode::{Gated, Ro, Rw, Unaccessible, Wo},
    };
}

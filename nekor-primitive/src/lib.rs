#![no_std]

//! Machine primitive values for the Nekor Operating System.

pub mod identity;

pub mod scalar;

pub mod prelude {
    //! The prelude module provides convenient imports for common types and
    //! traits.

    pub use crate::{
        identity::{One, Zero},
        scalar::Scalar,
    };
}

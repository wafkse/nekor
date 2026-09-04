#![cfg_attr(not(any(test, miri, usermode)), no_std)]

//! Architecture-agnostic processor feature detection.

pub mod arch;

pub mod feature;

pub mod prelude {
    //! The prelude module for the `nekor_aal_feature`.

    pub use crate::feature::{Feature, Present, Signal};
}

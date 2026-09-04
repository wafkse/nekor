#![cfg_attr(not(any(test, miri)), no_std)]

//! Static storage for the Nekor unikernel.

pub mod arch;

pub mod store;

pub mod domain;

pub mod lazy;

pub mod zeroed;

pub mod prelude {
    //! A prelude that re-exports the items that are most likely to be used.

    pub use crate::{
        arch::{Container, Header, InitializationStage},
        domain::{Adapter, Domain, arbitrary::Arbitrary, preset::Preset},
        lazy::{Erased, Lazy},
        store::{Static, Store},
        zeroed::{Zeroable, Zeroed},
    };
}

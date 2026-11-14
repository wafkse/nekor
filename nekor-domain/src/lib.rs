#![cfg_attr(not(any(test, miri)), no_std)]
//! Static storage for the Nekor unikernel.

pub mod lowlevel;

pub mod store;

pub mod domain;

pub mod lazy;

pub mod zeroed;

pub mod prelude {
    //! A prelude that re-exports the items that are most likely to be used.

    pub use crate::{
        domain::{Adapter, Domain, arbitrary::Arbitrary, preset::Preset},
        lazy::{Erased, Lazy},
        lowlevel::{Container, Header, InitializationStage},
        store::{Static, Store},
        zeroed::{Zeroable, Zeroed},
    };
}

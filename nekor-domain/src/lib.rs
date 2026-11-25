#![cfg_attr(not(any(test, miri)), no_std)]
//! Static storage for the Nekor unikernel.

pub mod store;

pub mod domain;

pub mod lazy;

pub mod prelude {
    //! A prelude that re-exports the items that are most likely to be used.

    pub use crate::{
        domain::{Adapter, Domain, arbitrary::Arbitrary, preset::Preset},
        lazy::{Erased, Lazy},
        store::{
            Static, Store,
            lowlevel::{Container, Header, InitializationStage},
            zeroed::{Zeroable, Zeroed},
        },
    };
}

#![no_std]
#![doc = include_str!("../README.md")]

#[doc(hidden)]
pub mod lowlevel;

#[cfg(any(feature = "stable", feature = "lts"))]
#[doc(hidden)]
pub mod generated;

#[cfg(feature = "lts")]
pub use generated::lts;
#[cfg(feature = "stable")]
pub use generated::stable;

#[cfg(any(feature = "stable", feature = "lts"))]
cfg_select! {
    all(feature = "stable", feature = "lts") => {
        pub use crate::stable as preferred;
    },
    feature = "stable" => {
        pub use crate::stable as preferred;
    },
    feature = "lts" => {
        pub use crate::lts as preferred;
    },
}

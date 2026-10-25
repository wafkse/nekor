#![no_std]
#![doc = include_str!("../README.md")]

/// Raw KVM UAPI from the pinned Linux LTS release.
#[cfg(feature = "lts")]
#[expect(
    nonstandard_style,
    clippy::all,
    clippy::pedantic,
    clippy::restriction,
    clippy::missing_docs_in_private_items
)]
pub mod lts;

/// Raw KVM UAPI from the pinned Linux stable release.
#[cfg(feature = "stable")]
#[expect(
    nonstandard_style,
    clippy::all,
    clippy::pedantic,
    clippy::restriction,
    clippy::missing_docs_in_private_items
)]
pub mod stable;

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

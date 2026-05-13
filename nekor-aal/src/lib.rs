#![cfg_attr(not(any(test, miri, usermode)), no_std)]
#![forbid(
    clippy::all,
    clippy::perf,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::panic,
    clippy::pedantic,
    rustdoc::all
)]
//! The umbrella crate for the *Architecture Abstraction Layer* (*AAL*) in
//! *Nekor*.

pub use nekor_aal_arch as arch;

pub use nekor_aal_cache as cache;

pub use nekor_aal_critical as critical;

pub use nekor_aal_feature as feature;

pub use nekor_aal_hotpatch as hotpatch;

pub use nekor_aal_local as local;

pub use nekor_aal_signal as signal;

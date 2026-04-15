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
//! Critical Section support at the architecture level.
//!
//! In *Nekor*, there are many possible *Critical Sections* (*CS*) that the
//! kernel can acquire.
//!
//! - *Local Critical Sections* (*LCS*): [`lcs`].
//!
//! - *Global Critical Sections* (*GCS*): [`gcs`]
//!
//! - *Machine-Stop Critical Sections* (*`MsCS`*): [`mscs`]
//!
//! - *Stable-point Critical Sections* (*`SpCS`*): [`spcs`]

pub mod arch;

pub mod lcs;

pub mod gcs;

pub mod mscs;

pub mod spcs;

#![cfg_attr(
    not(clippy),
    forbid(
        clippy::all,
        clippy::perf,
        clippy::nursery,
        clippy::unwrap_used,
        clippy::panic,
        clippy::pedantic,
        rustdoc::all
    )
)]
// Third-party derives emit scoped Clippy allowances. Preserve strict lints for
// authored code while permitting those generated scopes during Clippy runs.
#![cfg_attr(
    clippy,
    deny(
        clippy::all,
        clippy::perf,
        clippy::nursery,
        clippy::unwrap_used,
        clippy::panic,
        clippy::pedantic,
        rustdoc::all
    )
)]
#![allow(clippy::unnecessary_clippy_cfg)]
//! The build orchestration crate for the Nekor Unikernel.

pub mod orchestrate;

pub mod platform;

pub mod invoke;

pub mod manifest;

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
//! Cpu-local storage access.
//!
//! This is largely implemented outside of this module. However, this requires
//! that the *Architecture Abstraction Layer* expose an interface to access a
//! pointer-sized value. Particularly, this corresponds to the [`CoreId`] type.
//!
//! For instance, on `x86`, either `fs` or `gs` is used, but they are accessed
//! through an indirection: e.g.:
//!
//! ```asm
//! read_cpu_area:
//!     movq %gs:0h, %rax
//! ```
//!
//! Done otherwise, such as a read to the `IA32_GS_BASE` through `rdmsr`, which
//! is serializing (`rdmsrns` nonwithstanding), or use of `rdgsbase`, which
//! requires the `FSGSBASE` extension and explicit support would be too
//! restrictive in terms of what processors can run the kernel.
//!
//! This ends up in requiring a small [`usize`]-sized per-CPU area to use as
//! either `fs` or `gs` base.
//!
//! All other per-CPU data is managed by a specialized `PerCpu` abstration.
//!
//! In other words, this requires that all *AAL* provide a platform-specific way
//! to access a single [`usize`]. This behavior is encapsulated through
//! [`Area`]. The area is [`Copy`], and therefore has no further semantics an
//! *AAL* has to take into account.
//!
//! [`Area`]: crate::area::Area

pub mod arch;

pub mod area;

pub mod prelude {
    //! A prelude to expose the most commonly-used items of this crate.

    pub use crate::area::Area;
}

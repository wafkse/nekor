#![cfg_attr(not(any(test, miri, usermode)), no_std)]
//! The primary library interface for Nekor.
//!
//! Nekor is composed of focused facility crates. This crate provides the
//! canonical public namespace by re-exporting those facilities without
//! flattening their APIs.

pub use nekor_aal as aal;
pub use nekor_backoff as backoff;
pub use nekor_bitwise as bitwise;
pub use nekor_boot as boot;
pub use nekor_cpu as cpu;
pub use nekor_domain as domain;
pub use nekor_executor as executor;
pub use nekor_hal as hal;
pub use nekor_platform as platform;
pub use nekor_primitive as primitive;
pub use nekor_project as project;
pub use nekor_register as register;
pub use nekor_socal as socal;
pub use nekor_structure as structure;
pub use nekor_sync as sync;

/// Enter the Nekor runtime.
///
/// The executable target delegates its platform entry symbol to this function,
/// keeping the package's runtime entry path owned by the library interface.
#[inline(never)]
pub fn entry() -> ! {
    loop {
        core::hint::spin_loop();
    }
}

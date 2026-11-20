#![cfg_attr(not(any(test, miri, usermode)), no_std)]

//! Access to a pointer-sized processor-local value.
//!
//! [`area::Area`] stores an opaque value used by higher-level processor identity code. On
//! the x86-64 kernel path it is read from or written to the word at `GS` offset zero. The
//! platform must establish that segment base and its backing before these APIs are called.
//! Usermode builds use a host thread-local cell instead, so that value follows the host
//! thread and does not establish physical processor affinity.

pub mod arch;

pub mod area;

pub mod prelude {
    //! A prelude to expose the most commonly-used items of this crate.

    pub use crate::area::Area;
}

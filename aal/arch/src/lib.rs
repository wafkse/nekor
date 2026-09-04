#![cfg_attr(not(any(test, miri, usermode)), no_std)]

//! Architecture-specific modules that are not related to any *AAL* subsystem.

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod x86;

#[cfg(target_arch = "x86_64")]
pub mod x86_64;

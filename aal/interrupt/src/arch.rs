//! The architecture-specific modules for interrupt functionality.

#[cfg(target_arch = "x86_64")]
pub mod x86;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod x86_64;

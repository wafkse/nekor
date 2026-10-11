//! Architecture-specific module for the `nekor-backoff` crate.
//!
//! This encompasses the per-architecture functionality that is used to perform energy-efficient
//! backoff operations.
//!
//! For instance, on `x86`, instructions like `pause`, or `tpause`, are used for relaxing the
//! processor during backoff operations, as there is no scheduler to preempt us.

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod x86;

#[cfg(target_arch = "x86_64")]
pub mod x86_64;

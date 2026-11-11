//! KVM resource types and their ownership operations.
//!
//! The KVM handle creates machines, which own guest memory and virtual processors. Their
//! lifetimes keep descriptors and mappings live while the guest can access them.

pub mod state;

#[cfg(any(feature = "stable", feature = "lts"))]
pub mod machine;

#[cfg(any(feature = "stable", feature = "lts"))]
mod memory;

#[cfg(any(feature = "stable", feature = "lts"))]
pub mod vcpu;

#[cfg(any(feature = "stable", feature = "lts"))]
mod run;

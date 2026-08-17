//! Cross-modifying-code formal model.
//!
//! # Why?
//!
//! In a multi-CPU machine, code that is actively rewriting portions of itself
//! and plans to have *foreign CPU cores* needs to guarantee that such new
//! instruction stream is coherent and fully visible:
//!
//! - to distinct CPU-cores at some machine-wide, stable *Virtual Address*.
//! - to a distinct or same CPU-core but at a distinct *Virtual Address*.
//!
//! Reference: <https://cr.openjdk.org/~jrose/jvm/hotspot-cmc.html>

use core::num::NonZero;
use core::ptr::NonNull;

use nekor_aal_cache::line::Cacheline;

/// A target for either a *CMC-Acquire* or *CMC-Publish* abstract operation.
#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct Target(NonNull<Cacheline>, NonZero<usize>);

/// An umbrella type for the *CMC-Publish* abstract operation.
#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub enum Publish {}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
impl Publish {
    /// Perform a *CMC-Publish* operation for a set of [`Target`]s.
    #[inline]
    pub fn now(target_list: impl Iterator<Item = Target>) {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        use crate::arch::x86::PublishX86;

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        PublishX86::now(target_list);
    }
}

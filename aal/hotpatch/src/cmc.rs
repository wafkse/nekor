//! Instruction-stream publication support for runtime patching.
//!
//! A processor modifying code must arrange for every processor executing that code to
//! observe a complete instruction stream. Atomic data stores alone do not establish
//! this relationship, and executable aliases may need their own publication protocol.
//!
//! The x86 implementation currently serializes only the calling processor. It ignores
//! the supplied targets and performs no rendezvous with other processors. Callers must
//! provide that coordination before relying on remote execution of a new diversion.
//!
//! Background: <https://cr.openjdk.org/~jrose/jvm/hotspot-cmc.html>

use core::{convert::Infallible, num::NonZero, ptr::NonNull};

use nekor_aal_cache::line::Cacheline;

/// A target for either a *CMC-Acquire* or *CMC-Publish* abstract operation.
#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Hash)]
// NOTE(invariant): The cacheline pointer is non-null and the target count is nonzero.
pub struct Target(NonNull<Cacheline>, NonZero<usize>);

/// An umbrella type for the *CMC-Publish* abstract operation.
#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub enum Publish {
    /// An impossible marker variant; this type is used only as a namespace.
    __Variant(Infallible),
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
impl Publish {
    /// Serialize the calling processor after a code update.
    ///
    /// The current x86 implementation does not inspect the targets or notify other
    /// processors. Their instruction-stream synchronization remains the caller's work.
    #[inline]
    pub fn now(target_list: impl Iterator<Item = Target>) {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        use crate::arch::x86::PublishX86;

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        PublishX86::now(target_list);
    }
}

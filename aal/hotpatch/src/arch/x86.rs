//! `x86` architecture support.

use nekor_aal_arch::x86::serialize::{IRet, Serialize};

use nekor_aal_feature::{
    arch::x86::qualified,
    prelude::{Feature, Present, Signal},
};

use crate::cmc::Target;

/// An architecture-specific implementation for the abstract *CMC-Publish*
/// operation.
pub enum PublishX86 {}

impl PublishX86 {
    /// Perform a *CMC-Publish* operation in a coordinated fashion.
    #[inline]
    pub fn now(_: impl Iterator<Item = Target>) {
        let serialize_support_signal = &qualified::Serialize::supported();

        if Signal::is(serialize_support_signal, Present::Yes) {
            // SAFETY: The Serialize::now() function is safe to call as it is
            // only called when the Serialize feature is supported.
            unsafe { Serialize::now() };
        } else {
            #[cfg(target_arch = "x86")]
            IRet::x86();

            #[cfg(target_arch = "x86_64")]
            IRet::x86_64();
        }
    }
}

/// An architecture-specific implementation for the abstract *CMC-Acquire*
/// operation.
pub enum AcquireX86 {}

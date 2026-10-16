#![no_std]
#![doc = include_str!("../README.md")]

use nekor_virt_model::prelude::Arch;

extern crate alloc;

pub mod kvm {
    //! Module for the `KVM` hypervisor.

    pub use nekor_virt_kvm::*;
}

pub mod whp {
    //! Module for the `WHP` hypervisor.

    pub use nekor_virt_whp::*;
}

/// A representation of a particular hypervisor backend.
///
/// For example, on Linux, `KVM` is generally used. On Windows, the `WHP` (abbreviated from `Windows
/// Hypervisor Example`) permits a usermode-managed virtual-machine.
pub trait Backend<A>
where
    A: Arch,
{
}

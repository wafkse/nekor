#![no_std]
#![doc = include_str!("../README.md")]

extern crate alloc;

pub mod baseline;

pub mod kvm {
    //! Module for the `KVM` hypervisor.

    pub use nekor_virt_kvm::*;
}

pub mod whp {
    //! Module for the `WHP` hypervisor.

    pub use nekor_virt_whp::*;
}

pub mod prelude {
    //! Exports the most commonly used items in the `nekor-virt-hypervisor` crate.

    pub use crate::baseline::Backend;
}

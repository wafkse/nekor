#![doc = include_str!("../README.md")]

extern crate alloc;

use eustace_kvm_sys as sys;

pub mod arch;

pub mod error;

pub mod resource;

#[cfg(any(feature = "stable", feature = "lts"))]
pub mod run;

pub mod prelude {
    //! The `eustace-kvm` crate prelude.
    //!
    //! It re-exports common types used to open KVM, configure guest memory, and handle vCPU exits.

    pub use eustace_address::gpa::Gpa;
    #[cfg(any(feature = "stable", feature = "lts"))]
    pub use eustace_arch::x86::entry::RealModeEntryPoint;
    #[cfg(any(feature = "stable", feature = "lts"))]
    pub use eustace_ioport::{
        access::{IoDirection, IoElementCount, IoElementSize, IoPortAccess},
        port::IoPort,
    };
    #[cfg(any(feature = "stable", feature = "lts"))]
    pub use eustace_memory::{region::GuestMemoryRegion, size::GuestMemorySize};
    #[cfg(any(feature = "stable", feature = "lts"))]
    pub use eustace_mmio::access::{MmioAccess, MmioByteCount, MmioRead, MmioWrite};

    pub use crate::{
        error::{KvmError, Result},
        resource::state::Kvm,
    };
    #[cfg(any(feature = "stable", feature = "lts"))]
    pub use crate::{
        resource::{machine::Machine, vcpu::Vcpu},
        run::{ExitReason, RunExit},
    };
}

//! KVM operations grouped by device, virtual machine, virtual processor, and memory mapping.

pub mod arch {
    //! KVM operations specific to the guest architecture.

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub mod x86;
}

pub mod host;

pub mod device;

pub mod vm;

pub mod vcpu;

pub mod mapping;

pub mod run;

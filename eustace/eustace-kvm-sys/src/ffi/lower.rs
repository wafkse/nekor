//! Lowering layer for interfacing with KVM and the C-based bridge.

pub mod arch {
    //! Architecture-specific module tree for interaction with KVM.

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub mod x86;
}

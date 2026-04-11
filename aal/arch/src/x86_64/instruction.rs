//! Instructions specific to `x86-64`.

pub use crate::x86::instruction::*;

pub mod idtr {
    //! Instructions related to the *Interrupt Descriptor Table Register*.

    use core::{arch, mem::MaybeUninit};

    use crate::x86::{
        descriptor::{DescriptorTablePointer, Idt},
        mode::Bits64,
    };

    /// Load a *Interrupt Descriptor Table* in *Long Mode*.
    ///
    /// # Safety
    ///
    /// - The caller must have a *privilege level of zero*.
    /// - The provided [`DescriptorTablePointer<Idt>`] must be valid,
    ///   particularly no bad interrupt handler can be invoked through the
    ///   provided descriptor.
    #[inline]
    pub unsafe fn lidtq(target_address: &'static DescriptorTablePointer<Idt, Bits64>) {
        // SAFETY: The safety of this instruction has been guaranteed by the caller.
        unsafe {
            arch::asm!(
                "lidtq [{}]",
                in(reg) target_address,
                options(nostack, preserves_flags, readonly, att_syntax)
            )
        }
    }

    /// Store the currently-active (*IDTR*) *Interrupt Descriptor Table* in
    /// *Long Mode*.
    ///
    /// # Remarks
    ///
    /// * The caller must have a *privilege level of zero*, or have the
    ///   `CR4.UMIP` bit disabled.
    #[inline]
    pub unsafe fn sidtq(target_address: &mut MaybeUninit<DescriptorTablePointer<Idt, Bits64>>) {
        // SAFETY: The safety of this instruction has been guaranteed by the caller.
        unsafe {
            arch::asm!(
                "sidtq [{}]",
                in(reg) target_address,
                options(nostack, preserves_flags, att_syntax)
            )
        }
    }
}

pub mod fsgsbase {
    //! Instructions provided by the `FSGSBASE` architectural extension.

    use core::arch;

    /// Read the "%gs" extended segment register base value.
    ///
    /// # Safety
    ///
    /// * The `FSGSBASE` architectural extension must be supported.
    /// * The `CR4.FSGSBASE` bit must be set (*1*).
    #[inline]
    pub unsafe fn rdgsbase() -> u64 {
        let gsbase: u64;

        // SAFETY: The instruction is supported by the processor, as guaranteed by the
        // caller.
        unsafe {
            arch::asm!(
                "rdgsbaseq {}",
                lateout(reg) gsbase
            );
        }

        gsbase
    }

    /// Read the "%fs" extended segment register base value.
    ///
    /// # Safety
    ///
    /// * The `FSGSBASE` architectural extension must be supported.
    ///
    /// * The `CR4.FSGSBASE` bit must be set (*1*).
    #[inline]
    pub unsafe fn rdfsbase() -> u64 {
        let fsbase: u64;

        // SAFETY: The instruction is supported by the processor, as guaranteed by the
        // caller.
        unsafe {
            arch::asm!(
                "rdfsbaseq {}",
                lateout(reg) fsbase
            );
        }

        fsbase
    }

    /// Write to the "%fs" extended segment register base value.
    ///
    /// # Safety
    ///
    /// * The `FSGSBASE` architectural extension must be supported.
    ///
    /// * The `CR4.FSGSBASE` bit must be set (*1*).
    ///
    /// * The provided base address must be canonical.
    #[inline]
    pub unsafe fn wrfsbase(fsbase: u64) {
        // SAFETY: The instruction is supported by the processor, as guaranteed by the
        // caller.
        unsafe {
            arch::asm!(
                "wrfsbaseq {}",
                in(reg) fsbase
            );
        }
    }

    /// Write to the "%gs" extended segment register base value.
    ///
    /// # Safety
    ///
    /// * The `FSGSBASE` architectural extension must be supported.
    ///
    /// * The `CR4.FSGSBASE` bit must be set (*1*).
    ///
    /// * The provided base address must be canonical.
    #[inline]
    pub unsafe fn wrgsbase(gsbase: u64) {
        // SAFETY: The instruction is supported by the processor, as guaranteed by the
        // caller.
        unsafe {
            arch::asm!(
                "wrgsbaseq {}",
                in(reg) gsbase
            );
        }
    }
}

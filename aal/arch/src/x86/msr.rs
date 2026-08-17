//! Model-specific register interaction.

use core::arch;

/// The value of a model-specific register. Always 64-bit.
pub type MsrValue = u64;

/// An enumeration possible model-specific register.
#[repr(u32)]
pub enum Msr {
    /// The stack pointer (`{E,R}SP`) after a `sysenter` invocation.
    ///
    /// This functions both in 32- and 64-bit mode, hence why no register size
    /// is hinted in the name.
    SysEnterSp = 0x175,

    /// The instruction pointer (`{E,R}IP`) after a `sysenter` invocation.
    ///
    /// This functions both in 32- and 64-bit mode, hence why no register size
    /// is hinted in the name.
    SysEnterIp = 0x176,

    /// The recipient address of the system-wide system call handler.
    LStar = 0xC000_0082,

    /// The recipient address of the system-wide system call handler.
    ///
    /// This is akin to [`Msr::LStar`], but is destined for 32-bit userspace
    /// running under compatibility mode.
    CStar = 0xC000_0083,

    /// The mask used to determine which `RFLAGS` bits to clear in a system call
    /// transition.
    ///
    /// Each bit in this register will correspond to a cleared bit in `RFLAGS`.
    SfMask = 0xC000_0084,

    /// The Extended Feature Enable Register (EFER).
    ///
    /// Used for `sys{call,ret}` enable and long mode.
    Efer = 0xC000_0080,

    /// The core-specific `FS.Base` register.
    FsBase = 0xC000_0100,

    /// The core-specific `GS.Base` register.
    GsBase = 0xC000_0101,

    /// The kernel-specific `GS.Base` register.
    ///
    /// Swapped with [`Msr::GsBase`] on every `swapgs` instruction during a
    /// `CPL` context transition.
    KernelGsBase = 0xC000_0102,
}

impl Msr {
    /// Read the target model-specific register as specified.
    ///
    /// # Safety
    ///
    /// The caller processor context must have a "Current Privilege Level" of
    /// zero (*0*).
    ///
    /// If the specified [`Msr`] is deemed *non-standard*, the model-specific
    /// register value must be present in this processor model.
    #[inline]
    #[must_use = "a model-specific register read is expensive and should be consumed"]
    pub unsafe fn read(target_register: Self) -> MsrValue {
        let (high, low): (u64, u64);

        // SAFETY: Reading a MSR has no safety concerns, other than having the
        // required privilege level.
        unsafe {
            arch::asm!(
                "rdmsr",
                in("ecx") target_register as u32,
                lateout("edx") high,
                lateout("eax") low,
                options(nostack, nomem, preserves_flags)
            );
        }

        high << u32::BITS | low
    }

    /// Read the target model-specific register as specified.
    ///
    /// # Remarks
    ///
    /// A call to this associated function generates an absolute compiler fence
    /// of ordering [`Ordering::SeqCst`].
    ///
    /// # Safety
    ///
    /// The caller processor context must have a `CPL = 0`, and a valid register
    /// value must be provided.
    ///
    /// Furthermore, the specific model-specific register must not pose any
    /// safety or architectural hazards by itself.
    ///
    /// Note that regardless of safety annotations, one must prefer safe
    /// wrappers for model-specific registers than directly invoking this
    /// function.
    #[inline]
    pub unsafe fn write(target_register: Self, target_value: MsrValue) {
        let [low_0, low_1, low_2, low_3, high_0, high_1, high_2, high_3] =
            target_value.to_le_bytes();
        let low = u32::from_le_bytes([low_0, low_1, low_2, low_3]);
        let high = u32::from_le_bytes([high_0, high_1, high_2, high_3]);

        // SAFETY: Concerns are per-MSR, cannot be exhaustively mentioned.
        unsafe {
            arch::asm!(
                "wrmsr",
                in("ecx") target_register as u32,
                in("edx") high,
                in("eax") low,
                options(nostack, preserves_flags, nomem)
            );
        }
    }
}

//! Module to implement accelerated read and restore of "%fs.base"
//! and "%gs.base" extended segment register bases.
//!
//! This contains all the implementations to serve as an unified sub-module
//! rather than a set of cooperating items.

use core::arch;

use nekor_aal_agnostic::low_level::{Contextual, LowLevel};
#[cfg(not(usermode))]
use nekor_aal_arch::x86::msr::{
    Msr,
    segment::{MsrFsBase, MsrGsBase},
};
use nekor_aal_arch::x86_64::register::Base;
use nekor_aal_hotpatch::{
    patch::site::Diversion,
    prelude::{Chosen, Delegated, Delegator, Eventual},
};

/// A 2-tuple struct composed of the "%fs" and "%gs" base register values.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(C)]
// NOTE(invariant): The two raw words preserve FS base then GS base in fixed field order.
pub struct FsGsBase(Base, Base);

impl FsGsBase {
    /// Construct an architectural base-register pair.
    #[inline]
    #[must_use]
    pub const fn new(fs: Base, gs: Base) -> Self {
        Self(fs, gs)
    }

    /// Borrow the FS base image.
    #[inline]
    #[must_use]
    pub const fn fs(&self) -> &Base {
        let Self(fs, ..) = self;

        fs
    }

    /// Borrow the GS base image.
    #[inline]
    #[must_use]
    pub const fn gs(&self) -> &Base {
        let Self(.., gs) = self;

        gs
    }
}

/// A newtype wrapper over the [`LowLevel`] type that is context-specific that
/// describes low-level context-save logic. `CsEntry` stands for `Context-save
/// Entry`.
#[repr(transparent)]
// NOTE(invariant): The private field prevents constructing a context-save entry from a bare value.
pub(in crate::arch::x86_64) struct CsEntry<T>(LowLevel<T>);

// SAFETY: This is being implemented for a `LowLevel` wrapper type.
unsafe impl<T> Contextual for CsEntry<T> {}

/// The [`Delegator`] for the selection of either `rdmsr` or the `FSGSBASE`
/// architectural extension.
#[derive(Clone, Copy)]
pub(in crate::arch::x86_64) enum ReadFsgsbaseDelegator {}

// SAFETY: Selection is fixed at build time. The caller guarantees CPL0 for
// `rdmsr` and enables `FSGSBASE` before using `rdfsbase` or `rdgsbase`.
unsafe impl Delegator for ReadFsgsbaseDelegator {
    type Target = cfg_select! {
        usermode => RdFsgsbaseDelegate,
        _ => RdmsrDelegate,
    };
    type Value = ();

    #[inline]
    fn choose(_: &'static Self::Value) -> Chosen<Self, CsEntry<()>, FsGsBase> {
        Chosen::delegated::<Self::Target>()
    }
}

// SAFETY: Both delegates read FS and GS in the same order. The caller keeps
// either target usable until all CPUs see the patch.
unsafe impl Eventual for ReadFsgsbaseDelegator {}

/// The [`Delegator`] for the selection of either `wrmsr` or the `FSGSBASE`
/// architectural extension.
#[derive(Clone, Copy)]
pub(in crate::arch::x86_64) enum WriteFsgsbaseDelegator {}

// SAFETY: Selection is fixed at build time. The caller guarantees CPL0 for
// `wrmsr` and enables `FSGSBASE` before using `wrfsbase` or `wrgsbase`.
unsafe impl Delegator for WriteFsgsbaseDelegator {
    type Target = cfg_select! {
        usermode => WrFsgsbaseDelegate,
        _ => WrmsrDelegate,
    };
    type Value = ();

    #[inline]
    fn choose(_: &'static Self::Value) -> Chosen<Self, CsEntry<FsGsBase>, ()> {
        Chosen::delegated::<Self::Target>()
    }
}

// SAFETY: Both delegates set FS and GS in the same order. The caller keeps
// either target usable until all CPUs see the patch.
unsafe impl Eventual for WriteFsgsbaseDelegator {}

/// Install the `FSGSBASE` read and write delegates.
///
/// Non-usermode builds start with the model-specific register delegates. Usermode
/// builds start with the `FSGSBASE` delegates.
/// A successful write to either patchsite takes effect independently.
///
/// # Safety
///
/// * Every CPU that can run the new delegates must support `FSGSBASE` and have `CR4.FSGSBASE` set.
///   Keep that bit set while either patchsite can jump to them.
/// * Patch storage must be writable. Publish the new jumps as required by
///   [`Patchsite::stale`](nekor_aal_hotpatch::patch::site::Patchsite::stale).
/// * Any CPU that can still reach an MSR delegate must execute it at CPL0.
///
/// # Errors
///
/// Returns the diversion observed when another writer wins a patchsite update.
#[inline]
pub unsafe fn accelerate() -> Result<(), Diversion> {
    let target_read = Chosen::<ReadFsgsbaseDelegator, CsEntry<()>, FsGsBase>::delegated::<RdFsgsbaseDelegate>();
    let target_write = Chosen::<WriteFsgsbaseDelegator, CsEntry<FsGsBase>, ()>::delegated::<WrFsgsbaseDelegate>();

    // SAFETY: The caller enables `FSGSBASE` on each CPU, provides writable patch
    // storage, and publishes the jumps. Both implementations use the same FS and GS order.
    unsafe {
        Chosen::patchsite().stale(target_read)?;
        Chosen::patchsite().stale(target_write)?;
    }

    Ok(())
}

/// A delegate for that uses `rdmsr` for reading the "%fs.base" and
/// "%gs.base" values.
#[cfg(not(usermode))]
#[derive(Clone, Copy)]
pub(in crate::arch::x86_64) enum RdmsrDelegate {}

/// The delegate implementation for the [`RdmsrDelegate`] type.
///
/// # Safety
///
/// This delegated implementation must only be called when the *Current
/// Privilege Level* is zero. (*0*).
// SAFETY: The trampoline runs at CPL0 and returns the FS base in `%rax` and the
// GS base in `%rdx`, in the order specified by `FsGsBase`.
#[cfg(not(usermode))]
unsafe impl Delegated for RdmsrDelegate {
    type Input = CsEntry<()>;
    type Output = FsGsBase;

    #[inline]
    fn implementation(entry: Self::Input) -> Self::Output {
        Self::trampoline(entry)
    }

    /// The specialisation to read `fsbase` and `gsbase` through the
    /// hotpatch subsystem.
    ///
    /// These cannot be implemented in bare Rust as the compiler can use
    /// registers that are not yet saved.
    #[unsafe(naked)]
    extern "sysv64-unwind" fn trampoline(_: Self::Input) -> Self::Output {
        arch::naked_asm!(
            "movl ${}, %ecx",
            "rdmsr",
            "shlq $32, %rdx",
            "orq %rdx, %rax",
            "movq %rax, %r8",
            //
            "movl ${}, %ecx",
            "rdmsr",
            "shlq $32, %rdx",
            "orq %rax, %rdx",
            "movq %r8, %rax",
            //
            "retq",
            const <MsrFsBase as Msr>::ADDRESS,
            const <MsrGsBase as Msr>::ADDRESS,
            options(att_syntax)
        )
    }
}

/// A delegate for that uses the `FSGSBASE` instruction family for reading
/// the "%fs.base" and "%gs.base" values.
#[derive(Clone, Copy)]
pub(in crate::arch::x86_64) enum RdFsgsbaseDelegate {}

/// The delegate implementation for the [`RdFsgsbaseDelegate`] type.
///
/// # Safety
///
/// * The `CR4.FSGSBASE` enablement bit must be set (*1*).
/// * The `FSGSBASE` architectural feature must be available.
// SAFETY: The caller guarantees that `FSGSBASE` is supported and
// `CR4.FSGSBASE` is set. The return registers are the same as for `rdmsr`.
unsafe impl Delegated for RdFsgsbaseDelegate {
    type Input = CsEntry<()>;
    type Output = FsGsBase;

    #[inline]
    fn implementation(entry: Self::Input) -> Self::Output {
        Self::trampoline(entry)
    }

    /// The specialisation to read `fsbase` and `gsbase` through the
    /// hotpatch subsystem.
    ///
    /// These cannot be implemented in bare Rust as the compiler can use
    /// registers that are not yet saved.
    #[unsafe(naked)]
    extern "sysv64-unwind" fn trampoline(_: Self::Input) -> Self::Output {
        arch::naked_asm!("rdfsbaseq %rax", "rdgsbaseq %rdx", "retq", options(att_syntax))
    }
}

/// A delegate for that uses `wrmsr` for writing the "%fs.base" and
/// "%gs.base" values.
#[cfg(not(usermode))]
#[derive(Clone, Copy)]
pub(in crate::arch::x86_64) enum WrmsrDelegate {}

/// The delegate implementation for the [`WrmsrDelegate`] type.
///
/// # Safety
///
/// This delegated implementation must only be called when the *Current
/// Privilege Level* is zero. (*0*).
// SAFETY: The trampoline runs at CPL0. The caller places canonical FS and GS
// bases in `%rdi` and `%rsi`, respectively.
#[cfg(not(usermode))]
unsafe impl Delegated for WrmsrDelegate {
    type Input = CsEntry<FsGsBase>;
    type Output = ();

    #[inline]
    fn implementation(entry: Self::Input) {
        Self::trampoline(entry);
    }

    /// The specialisation to write `fsbase` and `gsbase` through the
    /// hotpatch subsystem.
    ///
    /// These cannot be implemented in bare Rust as the compiler can use
    /// registers that are not yet saved.
    #[unsafe(naked)]
    extern "sysv64-unwind" fn trampoline(_: Self::Input) {
        arch::naked_asm!(
            "movl ${}, %ecx",
            "movq %rdi, %rax",
            "movq %rdi, %rdx",
            "shrq $32, %rdx",
            "wrmsr",
            //
            "movl ${}, %ecx",
            "movq %rsi, %rax",
            "movq %rsi, %rdx",
            "shrq $32, %rdx",
            "wrmsr",
            //
            "retq",
            const <MsrFsBase as Msr>::ADDRESS,
            const <MsrGsBase as Msr>::ADDRESS,
            options(att_syntax)
        )
    }
}

/// A delegate for that uses the `FSGSBASE` instruction family for writing
/// the "%fs.base" and "%gs.base" values.
#[derive(Clone, Copy)]
pub(in crate::arch::x86_64) enum WrFsgsbaseDelegate {}

/// The delegate implementation for the [`WrFsgsbaseDelegate`] type.
///
/// # Safety
///
/// * The `CR4.FSGSBASE` enablement bit must be set (*1*).
/// * The `FSGSBASE` architectural feature must be available.
// SAFETY: The caller guarantees that `FSGSBASE` is supported and
// `CR4.FSGSBASE` is set. It supplies canonical bases in the same registers as for `wrmsr`.
unsafe impl Delegated for WrFsgsbaseDelegate {
    type Input = CsEntry<FsGsBase>;
    type Output = ();

    #[inline]
    fn implementation(entry: Self::Input) {
        Self::trampoline(entry);
    }

    /// The specialisation to write `fsbase` and `gsbase` through the
    /// hotpatch subsystem.
    ///
    /// These cannot be implemented in bare Rust as the compiler can use
    /// registers that are not yet saved.
    #[unsafe(naked)]
    extern "sysv64-unwind" fn trampoline(_: Self::Input) {
        arch::naked_asm!("wrfsbaseq %rdi", "wrgsbaseq %rsi", "retq", options(att_syntax))
    }
}

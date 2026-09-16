//! Module to implement accelerated read and restore of "%fs.base"
//! and "%gs.base" extended segment register bases.
//!
//! This contains all the implementations to serve as an unified sub-module
//! rather than a set of cooperating items.

use core::{arch, ffi};

use nekor_aal_arch::x86::msr::{FsBase, GsBase, Msr};
use nekor_aal_hotpatch::prelude::{Chosen, Delegated, Delegator};

use crate::arch::x86_64::context::CsEntry;

/// A 2-tuple struct composed of the "%fs" and "%gs" base register values.
#[derive(Clone, Copy, Eq, PartialEq)]
#[repr(C)]
pub struct FsGsBase(u64, u64);

/// An enumeration that determines usage the regular `rdmsr` instruction or
/// the new `FSGSBASE` instruction family to read the "%fs.base" and
/// "%gs.base" registers.
#[derive(Clone, Copy, Eq, PartialEq, Default)]
pub enum RdmsrOrFsgsbase {
    /// Use the `rdmsr` instruction.
    #[default]
    Rdmsr,

    /// Use the `rdfsbase` and `rdgsbase` instructions.
    Fsgsbase,
}

/// The [`Delegator`] for the selection of either `rdmsr` or the `FSGSBASE`
/// architectural extension.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ReadFsgsbaseDelegator {}

// SAFETY: The selection is pure and done only on the input value.
unsafe impl Delegator for ReadFsgsbaseDelegator {
    // NOTE: We use `rdmsr` by-default as it is readily available.
    type Target = RdmsrDelegate;
    type Value = RdmsrOrFsgsbase;

    fn choose(
        target_value: &'static Self::Value,
    ) -> Chosen<Self, <Self::Target as Delegated>::Input, <Self::Target as Delegated>::Output> {
        match *target_value {
            RdmsrOrFsgsbase::Rdmsr => Chosen::delegated::<RdmsrDelegate>(),
            // FIXME: Sanity check for Fsgsbase feature here.
            RdmsrOrFsgsbase::Fsgsbase => Chosen::delegated::<RdFsgsbaseDelegate>(),
        }
    }
}

/// A delegate for that uses `rdmsr` for reading the "%fs.base" and
/// "%gs.base" values.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum RdmsrDelegate {}

/// The delegate implementation for the [`RdmsrDelegate`] type.
///
/// # Safety
///
/// This delegated implementation must only be called when the *Current
/// Privilege Level* is zero. (*0*).
// SAFETY: The trampoline is only used by low-level code executing at CPL 0,
// which satisfies the delegate's architectural privilege requirement.
unsafe impl Delegated for RdmsrDelegate {
    type Input = CsEntry<ffi::c_void>;
    type Output = FsGsBase;

    fn implementation(_: Self::Input) -> Self::Output {
        unreachable!()
    }

    /// The specialisation to read `fsbase` and `gsbase` through the
    /// hotpatch subsystem.
    ///
    /// These cannot be implemented in bare Rust as the compiler can use
    /// registers that are not yet saved.
    #[unsafe(naked)]
    extern "sysv64" fn trampoline(_: Self::Input) -> Self::Output {
        // SAFETY: The *Current Privilege Level* is zero, as asserted by the input type.
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
            "orq %rdx, %rax",
            "movq %rax, %rdx",
            //
            "movq %r8, %rax",
            "retq",
            const <FsBase as Msr>::ADDRESS,
            const <GsBase as Msr>::ADDRESS,
            options(att_syntax)
        )
    }
}

/// A delegate for that uses the `FSGSBASE` instruction family for reading
/// the "%fs.base" and "%gs.base" values.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum RdFsgsbaseDelegate {}

/// The delegate implementation for the [`RdFsgsbaseDelegate`] type.
///
/// # Safety
///
/// * The `CR4.FSGSBASE` enablement bit must be set (*1*).
/// * The [`Fsgsbase`] architectural feature must available.
// SAFETY: The trampoline is only used after the FSGSBASE architectural
// feature has been enabled, which satisfies the delegate's instruction
// availability requirement.
unsafe impl Delegated for RdFsgsbaseDelegate {
    type Input = CsEntry<ffi::c_void>;
    type Output = FsGsBase;

    fn implementation(_: Self::Input) -> Self::Output {
        unreachable!()
    }

    /// The specialisation to read `fsbase` and `gsbase` through the
    /// hotpatch subsystem.
    ///
    /// These cannot be implemented in bare Rust as the compiler can use
    /// registers that are not yet saved.
    #[unsafe(naked)]
    extern "sysv64" fn trampoline(_: Self::Input) -> Self::Output {
        // SAFETY:
        // * The `CR4.FSGSBASE` enablement bit is set (*1*).
        // * The [`Fsgsbase`] architectural feature is available.
        arch::naked_asm!("rdfsbaseq %rax", "rdgsbaseq %rdx", "retq", options(att_syntax))
    }
}

/// An enumeration that determines usage the regular `wrmsr` instruction or
/// the new `FSGSBASE` instruction family to read the "%fs.base" and
/// "%gs.base" registers.
#[derive(Clone, Copy, Eq, PartialEq, Default)]
pub enum WrmsrOrFsgsbase {
    /// Use the `wrmsr` instruction.
    #[default]
    Wrmsr,

    /// Use the `wrfsbase` and `wrgsbase` instructions.
    Fsgsbase,
}

/// The [`Delegator`] for the selection of either `wrmsr` or the `FSGSBASE`
/// architectural extension.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum WriteFsgsbaseDelegator {}

// SAFETY: The selection is pure and done only on the input value.
unsafe impl Delegator for WriteFsgsbaseDelegator {
    // NOTE: We use `wrmsr` by-default as it is readily available.
    type Target = WrmsrDelegate;
    type Value = WrmsrOrFsgsbase;

    fn choose(
        target_value: &'static Self::Value,
    ) -> Chosen<Self, <Self::Target as Delegated>::Input, <Self::Target as Delegated>::Output> {
        match *target_value {
            WrmsrOrFsgsbase::Wrmsr => Chosen::delegated::<WrmsrDelegate>(),
            // FIXME: Sanity check for Fsgsbase feature here.
            WrmsrOrFsgsbase::Fsgsbase => Chosen::delegated::<WrFsgsbaseDelegate>(),
        }
    }
}

/// A delegate for that uses `wrmsr` for reading the "%fs.base" and
/// "%gs.base" values.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum WrmsrDelegate {}

/// The delegate implementation for the [`WrmsrDelegate`] type.
///
/// # Safety
///
/// This delegated implementation must only be called when the *Current
/// Privilege Level* is zero. (*0*).
// SAFETY: The trampoline is only used by low-level code executing at CPL 0,
// which satisfies the delegate's architectural privilege requirement.
unsafe impl Delegated for WrmsrDelegate {
    type Input = CsEntry<FsGsBase>;
    type Output = ffi::c_void;

    fn implementation(_: Self::Input) -> Self::Output {
        unreachable!()
    }

    /// The specialisation to write `fsbase` and `gsbase` through the
    /// hotpatch subsystem.
    ///
    /// These cannot be implemented in bare Rust as the compiler can use
    /// registers that are not yet saved.
    #[unsafe(naked)]
    extern "sysv64" fn trampoline(_: Self::Input) -> Self::Output {
        // SAFETY: The *Current Privilege Level* is zero.
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
            "xorl %eax, %eax",
            "xorl %edx, %edx",
            //
            "retq",
            const <FsBase as Msr>::ADDRESS,
            const <GsBase as Msr>::ADDRESS,
            options(att_syntax)
        )
    }
}

/// A delegate for that uses the `FSGSBASE` instruction family for reading
/// the "%fs.base" and "%gs.base" values.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum WrFsgsbaseDelegate {}

/// The delegate implementation for the [`WrFsgsbaseDelegate`] type.
///
/// # Safety
///
/// * The `CR4.FSGSBASE` enablement bit must be set (*1*).
/// * The [`Fsgsbase`] architectural feature must available.
// SAFETY: The trampoline is only used after the FSGSBASE architectural
// feature has been enabled, which satisfies the delegate's instruction
// availability requirement.
unsafe impl Delegated for WrFsgsbaseDelegate {
    type Input = CsEntry<FsGsBase>;
    type Output = ffi::c_void;

    fn implementation(_: Self::Input) -> Self::Output {
        unreachable!()
    }

    /// The specialisation to write `fsbase` and `gsbase` through the
    /// hotpatch subsystem.
    ///
    /// These cannot be implemented in bare Rust as the compiler can use
    /// registers that are not yet saved.
    #[unsafe(naked)]
    extern "sysv64" fn trampoline(_: Self::Input) -> Self::Output {
        // SAFETY:
        // * The `CR4.FSGSBASE` enablement bit is set (*1*).
        // * The [`Fsgsbase`] architectural feature is available.
        arch::naked_asm!(
            "wrfsbaseq %rdi",
            "wrgsbaseq %rsi",
            // NOTE: Zero out `%rax`, even if we are returning a `ffi::c_void`.
            "xorl %eax, %eax",
            "retq",
            options(att_syntax)
        )
    }
}

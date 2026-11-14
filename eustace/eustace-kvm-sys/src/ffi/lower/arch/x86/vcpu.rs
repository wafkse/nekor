//! KVM vCPU register setup for x86 and x86-64 execution.

use std::{
    io,
    os::fd::{AsRawFd, BorrowedFd},
};

use crate::ffi::binding::bridge;

/// Instruction-pointer offset within the real-mode code segment.
pub type InstructionPointer = bridge::nekor_kvm_real_mode_instruction_pointer_t;

/// Configure the selected vCPU for a real-mode instruction pointer.
///
/// # Safety
///
/// For soundness purposes, the following must be satisfied:
///
/// * The provided file descriptor must identify a stopped x86 KVM virtual processor.
#[inline]
pub unsafe fn entry(vcpu: BorrowedFd<'_>, instruction_pointer: InstructionPointer) -> io::Result<()> {
    // SAFETY:
    //
    // * The caller has asserted that the provided file descriptor identifies a stopped x86 KVM
    //   virtual processor.
    let target_status =
        unsafe { bridge::nekor_kvm_vcpu_real_mode_entry_configure(vcpu.as_raw_fd(), instruction_pointer) };

    match target_status {
        bridge::NEKOR_KVM_STATUS_OK => Ok(()),
        target_errno @ bridge::nekor_kvm_status_t::MIN..bridge::NEKOR_KVM_STATUS_OK => {
            // NOTE: No such `errno` is `i32::MIN`.
            Err(io::Error::from_raw_os_error(target_errno.abs()))
        },
        // NOTE: The C helper contract excludes positive status values.
        _ => unreachable!(),
    }
}

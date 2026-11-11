//! Create and run KVM virtual processors.

use core::mem::MaybeUninit;
use std::{
    io,
    os::fd::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd},
};

use crate::ffi::binding::bridge;

/// Numeric vCPU identity within its KVM virtual machine.
pub type Id = bridge::nekor_kvm_vcpu_id_t;

/// Create a KVM vCPU descriptor.
///
/// # Safety
///
/// For soundness purposes, the following must be satisfied:
///
/// * The provided file descriptor must identify a KVM virtual machine.
#[inline]
pub unsafe fn create(vm: BorrowedFd<'_>, vcpu_id: Id) -> io::Result<OwnedFd> {
    let mut descriptor = MaybeUninit::<bridge::nekor_kvm_descriptor_t>::uninit();

    // SAFETY:
    //
    // * The caller has asserted that the provided file descriptor identifies a KVM virtual machine.
    // * The descriptor output storage remains writable for the duration of the call.
    let target_status = unsafe { bridge::nekor_kvm_vcpu_create(vm.as_raw_fd(), vcpu_id, descriptor.as_mut_ptr()) };

    match target_status {
        bridge::NEKOR_KVM_STATUS_OK => {
            // SAFETY:
            //
            // * The C helper initializes the descriptor before reporting success.
            let descriptor = unsafe { descriptor.assume_init() };

            match descriptor {
                0..=bridge::nekor_kvm_descriptor_t::MAX => {
                    // SAFETY:
                    //
                    // * The successful C helper transfers ownership of this nonnegative descriptor.
                    Ok(unsafe { OwnedFd::from_raw_fd(descriptor) })
                },
                _ => Err(io::Error::from(io::ErrorKind::InvalidData)),
            }
        },
        target_errno @ bridge::nekor_kvm_status_t::MIN..bridge::NEKOR_KVM_STATUS_OK => {
            Err(io::Error::from_raw_os_error(target_errno.abs()))
        },
        // NOTE: The C helper contract excludes positive status values.
        _ => unreachable!(),
    }
}

/// Resume the selected vCPU using its live, exclusively borrowed run mapping.
///
/// # Safety
///
/// For soundness purposes, the following must be satisfied:
///
/// * The provided file descriptor must identify a KVM virtual processor.
/// * The provided byte region must cover that virtual processor's shared run mapping.
/// * The run mapping must remain exclusively accessible throughout execution.
#[inline]
pub unsafe fn run(vcpu: BorrowedFd<'_>, run_mapping: &mut [u8]) -> io::Result<()> {
    let run_mapping_size = run_mapping
        .len()
        .try_into()
        .or(Err(io::Error::from(io::ErrorKind::InvalidInput)))?;

    // SAFETY:
    //
    // * The caller has asserted that the provided file descriptor identifies a KVM virtual
    //   processor.
    // * Its shared run mapping remains valid and exclusively accessible throughout execution.
    let target_status =
        unsafe { bridge::nekor_kvm_vcpu_run(vcpu.as_raw_fd(), run_mapping.as_mut_ptr().cast(), run_mapping_size) };

    match target_status {
        bridge::NEKOR_KVM_STATUS_OK => Ok(()),
        target_errno @ bridge::nekor_kvm_status_t::MIN..bridge::NEKOR_KVM_STATUS_OK => {
            Err(io::Error::from_raw_os_error(target_errno.abs()))
        },
        // NOTE: The C helper contract excludes positive status values.
        _ => unreachable!(),
    }
}

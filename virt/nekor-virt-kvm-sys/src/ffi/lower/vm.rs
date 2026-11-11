//! Create KVM virtual machines and register their guest memory.

use core::mem::MaybeUninit;
use std::{
    io,
    os::fd::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd},
};

use crate::ffi::binding::bridge;

/// Architecture-dependent KVM virtual-machine type.
pub type Type = bridge::nekor_kvm_vm_type_t;

pub mod memory;

/// Create a KVM virtual-machine descriptor.
///
/// # Safety
///
/// For soundness purposes, the following must be satisfied:
///
/// * The provided file descriptor must identify an open KVM device.
#[inline]
pub unsafe fn create(device: BorrowedFd<'_>, vm_type: Type) -> io::Result<OwnedFd> {
    let mut descriptor = MaybeUninit::<bridge::nekor_kvm_descriptor_t>::uninit();

    // SAFETY:
    //
    // * The caller has asserted that the provided file descriptor identifies an open KVM device.
    // * The descriptor output storage remains writable for the duration of the call.
    let target_status = unsafe { bridge::nekor_kvm_vm_create(device.as_raw_fd(), vm_type, descriptor.as_mut_ptr()) };

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

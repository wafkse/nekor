//! Mapping requirements for KVM vCPU run images.

use core::mem::MaybeUninit;
use std::{
    io,
    os::fd::{AsRawFd, BorrowedFd},
};

use crate::ffi::binding::bridge;

/// Read the mapping extent required for a KVM vCPU run image.
///
/// # Safety
///
/// For soundness purposes, the following must be satisfied:
///
/// * The provided file descriptor must identify an open KVM device.
#[inline]
pub unsafe fn size(device: BorrowedFd<'_>) -> io::Result<usize> {
    let mut mapping_size = MaybeUninit::<bridge::nekor_kvm_storage_size_t>::uninit();

    // SAFETY:
    //
    // * The caller has asserted that the provided file descriptor identifies an open KVM device.
    // * The mapping extent output storage remains writable for the duration of the call.
    let target_status =
        unsafe { bridge::nekor_kvm_device_run_mapping_size_read(device.as_raw_fd(), mapping_size.as_mut_ptr()) };

    match target_status {
        bridge::NEKOR_KVM_STATUS_OK => {
            // SAFETY:
            //
            // * The C helper initializes this output before reporting success.
            let mapping_size = unsafe { mapping_size.assume_init() };

            usize::try_from(mapping_size).or(Err(io::Error::from(io::ErrorKind::InvalidData)))
        },
        target_errno @ bridge::nekor_kvm_status_t::MIN..bridge::NEKOR_KVM_STATUS_OK => {
            Err(io::Error::from_raw_os_error(target_errno.abs()))
        },
        // NOTE: The C helper contract excludes positive status values.
        _ => unreachable!(),
    }
}

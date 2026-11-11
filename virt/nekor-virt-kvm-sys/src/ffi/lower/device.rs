//! Open KVM devices and query their API version.

use core::{ffi::CStr, mem::MaybeUninit};
use std::{
    io,
    os::fd::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd},
};

use crate::ffi::binding::bridge;

/// API version reported by the KVM device.
pub type Version = bridge::nekor_kvm_api_version_t;

pub mod run;

/// Open the selected KVM device.
#[inline]
pub fn open(device_path: &CStr) -> io::Result<OwnedFd> {
    let mut descriptor = MaybeUninit::<bridge::nekor_kvm_descriptor_t>::uninit();

    // SAFETY:
    //
    // * The path is nul-terminated and the descriptor output storage is writable.
    let target_status = unsafe { bridge::nekor_kvm_device_open(device_path.as_ptr(), descriptor.as_mut_ptr()) };

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

/// Read the KVM API version.
///
/// # Safety
///
/// For soundness purposes, the following must be satisfied:
///
/// * The provided file descriptor must identify an open KVM device.
#[inline]
pub unsafe fn version(device: BorrowedFd<'_>) -> io::Result<Version> {
    let mut api_version = MaybeUninit::<Version>::uninit();

    // SAFETY:
    //
    // * The caller has asserted that the provided file descriptor identifies an open KVM device.
    // * The API version output storage remains writable for the duration of the call.
    let target_status =
        unsafe { bridge::nekor_kvm_device_api_version_read(device.as_raw_fd(), api_version.as_mut_ptr()) };

    match target_status {
        bridge::NEKOR_KVM_STATUS_OK => {
            // SAFETY:
            //
            // * The C helper initializes this output before reporting success.
            let api_version = unsafe { api_version.assume_init() };

            Ok(api_version)
        },
        target_errno @ bridge::nekor_kvm_status_t::MIN..bridge::NEKOR_KVM_STATUS_OK => {
            Err(io::Error::from_raw_os_error(target_errno.abs()))
        },
        // NOTE: The C helper contract excludes positive status values.
        _ => unreachable!(),
    }
}

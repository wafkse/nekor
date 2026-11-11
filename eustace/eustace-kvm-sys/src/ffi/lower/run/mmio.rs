//! Checked MMIO access to the active KVM run exit.

use core::mem::MaybeUninit;
use std::io::{Error, ErrorKind, Result};

use crate::ffi::binding::bridge;

/// Direction of the active MMIO request.
pub type Direction = bridge::nekor_kvm_mmio_direction_t;

/// Guest address and response size of an MMIO read.
pub type Read = bridge::nekor_kvm_mmio_read_exit_t;

/// Guest address and payload of an MMIO write.
pub type Write = bridge::nekor_kvm_mmio_write_exit_t;

/// Classify the direction of the active MMIO exit.
#[inline]
pub fn direction(mapping: &[u8]) -> Result<Direction> {
    let run_size = mapping.len().try_into().or(Err(Error::from(ErrorKind::InvalidInput)))?;

    let mut direction = MaybeUninit::<Direction>::uninit();

    // SAFETY:
    //
    // * The slice is live for the call and the direction output storage is writable.
    let target_status =
        unsafe { bridge::nekor_kvm_run_mmio_exit_classify(mapping.as_ptr().cast(), run_size, direction.as_mut_ptr()) };

    match target_status {
        bridge::NEKOR_KVM_STATUS_OK => {
            // SAFETY:
            //
            // * The C function initializes the direction before returning success.
            Ok(unsafe { direction.assume_init() })
        },
        target_errno @ bridge::nekor_kvm_status_t::MIN..bridge::NEKOR_KVM_STATUS_OK => {
            Err(Error::from_raw_os_error(target_errno.abs()))
        },
        // NOTE: The C helper contract excludes positive status values.
        _ => unreachable!(),
    }
}

/// Copy the address and response size of an active MMIO read.
#[inline]
pub fn read(mapping: &[u8]) -> Result<Read> {
    let run_size = mapping.len().try_into().or(Err(Error::from(ErrorKind::InvalidInput)))?;

    let mut mmio_read_exit = MaybeUninit::<Read>::uninit();

    // SAFETY:
    //
    // * The slice is live for the call and the output record is writable.
    let target_status = unsafe {
        bridge::nekor_kvm_run_mmio_read_exit_read(mapping.as_ptr().cast(), run_size, mmio_read_exit.as_mut_ptr())
    };

    match target_status {
        bridge::NEKOR_KVM_STATUS_OK => {
            // SAFETY:
            //
            // * The C function initializes every field before returning success.
            Ok(unsafe { mmio_read_exit.assume_init() })
        },
        target_errno @ bridge::nekor_kvm_status_t::MIN..bridge::NEKOR_KVM_STATUS_OK => {
            Err(Error::from_raw_os_error(target_errno.abs()))
        },
        // NOTE: The C helper contract excludes positive status values.
        _ => unreachable!(),
    }
}

/// Copy the address and guest-written bytes of an active MMIO write.
#[inline]
pub fn write(mapping: &[u8]) -> Result<Write> {
    let run_size = mapping.len().try_into().or(Err(Error::from(ErrorKind::InvalidInput)))?;

    let mut mmio_write_exit = MaybeUninit::<Write>::uninit();

    // SAFETY:
    //
    // * The slice is live for the call and the output record is writable.
    let target_status = unsafe {
        bridge::nekor_kvm_run_mmio_write_exit_read(mapping.as_ptr().cast(), run_size, mmio_write_exit.as_mut_ptr())
    };

    match target_status {
        bridge::NEKOR_KVM_STATUS_OK => {
            // SAFETY:
            //
            // * The C function initializes every field before returning success.
            Ok(unsafe { mmio_write_exit.assume_init() })
        },
        target_errno @ bridge::nekor_kvm_status_t::MIN..bridge::NEKOR_KVM_STATUS_OK => {
            Err(Error::from_raw_os_error(target_errno.abs()))
        },
        // NOTE: The C helper contract excludes positive status values.
        _ => unreachable!(),
    }
}

/// Write userspace response bytes into an active MMIO read arm.
#[inline]
pub fn respond(mapping: &mut [u8], data: &[u8]) -> Result<()> {
    let run_size = mapping.len().try_into().or(Err(Error::from(ErrorKind::InvalidInput)))?;
    let data_size = data.len().try_into().or(Err(Error::from(ErrorKind::InvalidInput)))?;

    // SAFETY:
    //
    // * Both slices are live, and the mapping has exclusive access for the complete call.
    let target_status = unsafe {
        bridge::nekor_kvm_run_mmio_read_response_write(mapping.as_mut_ptr().cast(), run_size, data.as_ptr(), data_size)
    };

    match target_status {
        bridge::NEKOR_KVM_STATUS_OK => Ok(()),
        target_errno @ bridge::nekor_kvm_status_t::MIN..bridge::NEKOR_KVM_STATUS_OK => {
            Err(Error::from_raw_os_error(target_errno.abs()))
        },
        // NOTE: The C helper contract excludes positive status values.
        _ => unreachable!(),
    }
}

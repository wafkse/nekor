//! Checked port-I/O access to the active KVM run exit.

use core::mem::MaybeUninit;
use std::io::{Error, ErrorKind, Result};

use crate::ffi::binding::bridge;

/// Port, direction, width, and repetition count of a port-I/O exit.
pub type Exit = bridge::nekor_kvm_io_exit_t;

/// Copy the port-I/O metadata from the active run arm.
#[inline]
pub fn exit(mapping: &[u8]) -> Result<Exit> {
    let run_size = mapping.len().try_into().or(Err(Error::from(ErrorKind::InvalidInput)))?;

    let mut io_exit = MaybeUninit::<Exit>::uninit();

    // SAFETY:
    //
    // * The slice is live for the call and the exit record output storage is writable.
    let target_status =
        unsafe { bridge::nekor_kvm_run_io_exit_read(mapping.as_ptr().cast(), run_size, io_exit.as_mut_ptr()) };

    match target_status {
        bridge::NEKOR_KVM_STATUS_OK => {
            // SAFETY:
            //
            // * The C function initializes every field before returning success.
            Ok(unsafe { io_exit.assume_init() })
        },
        target_errno @ bridge::nekor_kvm_status_t::MIN..bridge::NEKOR_KVM_STATUS_OK => {
            Err(Error::from_raw_os_error(target_errno.abs()))
        },
        // NOTE: The C helper contract excludes positive status values.
        _ => unreachable!(),
    }
}

/// Copy guest port-output bytes into caller-owned storage and return the copied byte count.
#[inline]
pub fn read(mapping: &[u8], data: &mut [u8]) -> Result<usize> {
    let run_size = mapping.len().try_into().or(Err(Error::from(ErrorKind::InvalidInput)))?;
    let data_capacity = data.len().try_into().or(Err(Error::from(ErrorKind::InvalidInput)))?;

    let mut data_size = MaybeUninit::<bridge::nekor_kvm_storage_size_t>::uninit();

    // SAFETY:
    //
    // * Both slices and the output extent remain valid for the complete call.
    let target_status = unsafe {
        bridge::nekor_kvm_run_io_output_read(
            mapping.as_ptr().cast(),
            run_size,
            data.as_mut_ptr(),
            data_capacity,
            data_size.as_mut_ptr(),
        )
    };

    match target_status {
        bridge::NEKOR_KVM_STATUS_OK => {
            // SAFETY:
            //
            // * The C function initializes the byte count before returning success.
            let data_size = unsafe { data_size.assume_init() };

            usize::try_from(data_size).or(Err(Error::from(ErrorKind::InvalidInput)))
        },
        target_errno @ bridge::nekor_kvm_status_t::MIN..bridge::NEKOR_KVM_STATUS_OK => {
            Err(Error::from_raw_os_error(target_errno.abs()))
        },
        // NOTE: The C helper contract excludes positive status values.
        _ => unreachable!(),
    }
}

/// Copy userspace-supplied bytes into an active port-input arm.
#[inline]
pub fn write(mapping: &mut [u8], data: &[u8]) -> Result<()> {
    let run_size = mapping.len().try_into().or(Err(Error::from(ErrorKind::InvalidInput)))?;
    let data_size = data.len().try_into().or(Err(Error::from(ErrorKind::InvalidInput)))?;

    // SAFETY:
    //
    // * Both slices are live, and the mapping has exclusive access for the complete call.
    let target_status = unsafe {
        bridge::nekor_kvm_run_io_input_write(mapping.as_mut_ptr().cast(), run_size, data.as_ptr(), data_size)
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

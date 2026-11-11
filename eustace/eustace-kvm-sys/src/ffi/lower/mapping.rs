//! Allocate guest RAM, map vCPU run images, and copy mapped bytes.

use core::{mem::MaybeUninit, num::NonZeroUsize, ptr::NonNull};
use std::{
    io,
    os::fd::{AsRawFd, BorrowedFd},
};

use crate::ffi::binding::bridge;

/// Create a private anonymous mapping for guest RAM.
#[inline]
pub fn anonymous(size: NonZeroUsize) -> io::Result<NonNull<u8>> {
    let mut mapping = MaybeUninit::<*mut core::ffi::c_void>::uninit();

    let size = size
        .get()
        .try_into()
        .or(Err(io::Error::from(io::ErrorKind::InvalidInput)))?;

    // SAFETY:
    //
    // * The extent is nonzero and the mapping address output storage is writable.
    let target_status = unsafe { bridge::nekor_kvm_mapping_anonymous_create(size, mapping.as_mut_ptr()) };

    match target_status {
        bridge::NEKOR_KVM_STATUS_OK => {
            // SAFETY:
            //
            // * The C helper initializes the mapping address before reporting success.
            let mapping = unsafe { mapping.assume_init() };

            NonNull::new(mapping.cast()).ok_or_else(|| io::Error::from(io::ErrorKind::InvalidData))
        },
        target_errno @ bridge::nekor_kvm_status_t::MIN..bridge::NEKOR_KVM_STATUS_OK => {
            Err(io::Error::from_raw_os_error(target_errno.abs()))
        },
        // NOTE: The C helper contract excludes positive status values.
        _ => unreachable!(),
    }
}

/// Create a shared mapping of the selected vCPU's run image.
///
/// # Safety
///
/// For soundness purposes, the following must be satisfied:
///
/// * The provided file descriptor must identify the KVM virtual processor whose run mapping is
///   being acquired.
#[inline]
pub unsafe fn shared(vcpu: BorrowedFd<'_>, size: NonZeroUsize) -> io::Result<NonNull<u8>> {
    let mut mapping = MaybeUninit::<*mut core::ffi::c_void>::uninit();

    let size = size
        .get()
        .try_into()
        .or(Err(io::Error::from(io::ErrorKind::InvalidInput)))?;

    // SAFETY:
    //
    // * The caller has asserted that the provided file descriptor identifies the selected KVM
    //   virtual processor.
    // * The byte extent is nonzero, and the mapping address output storage remains writable for the
    //   call.
    let target_status =
        unsafe { bridge::nekor_kvm_mapping_shared_create(vcpu.as_raw_fd(), size, mapping.as_mut_ptr()) };

    match target_status {
        bridge::NEKOR_KVM_STATUS_OK => {
            // SAFETY:
            //
            // * The C helper initializes the mapping address before reporting success.
            let mapping = unsafe { mapping.assume_init() };

            NonNull::new(mapping.cast()).ok_or_else(|| io::Error::from(io::ErrorKind::InvalidData))
        },
        target_errno @ bridge::nekor_kvm_status_t::MIN..bridge::NEKOR_KVM_STATUS_OK => {
            Err(io::Error::from_raw_os_error(target_errno.abs()))
        },
        // NOTE: The C helper contract excludes positive status values.
        _ => unreachable!(),
    }
}

/// Release a complete host mapping.
///
/// # Safety
///
/// For soundness purposes, the following must be satisfied:
///
/// * The provided address must identify a live mapping owned by the caller and returned by this
///   bridge.
/// * The provided byte extent must equal the original mapping extent.
/// * No reference into the mapping may remain live after this call.
#[inline]
pub unsafe fn release(mapping: NonNull<u8>, mapping_size: NonZeroUsize) {
    let mapping_size = mapping_size.get() as bridge::nekor_kvm_storage_size_t;

    // SAFETY:
    //
    // * The caller supplies the live mapping and its original extent.
    unsafe { bridge::nekor_kvm_mapping_release(mapping.as_ptr().cast(), mapping_size) };
}

/// Copy bytes from one mapped region into caller-owned storage.
#[inline]
pub fn read(mapping: &[u8], offset: usize, destination: &mut [u8]) -> io::Result<()> {
    let mapping_size = mapping
        .len()
        .try_into()
        .or(Err(io::Error::from(io::ErrorKind::InvalidInput)))?;
    let mapping_offset = offset
        .try_into()
        .or(Err(io::Error::from(io::ErrorKind::InvalidInput)))?;
    let data_size = destination
        .len()
        .try_into()
        .or(Err(io::Error::from(io::ErrorKind::InvalidInput)))?;

    // SAFETY:
    //
    // * The source mapping and destination storage remain valid for the call. C checks the byte
    //   ranges.
    let target_status = unsafe {
        bridge::nekor_kvm_mapping_copy_to(
            mapping.as_ptr().cast(),
            mapping_size,
            mapping_offset,
            destination.as_mut_ptr(),
            data_size,
        )
    };

    match target_status {
        bridge::NEKOR_KVM_STATUS_OK => Ok(()),
        target_errno @ bridge::nekor_kvm_status_t::MIN..bridge::NEKOR_KVM_STATUS_OK => {
            Err(io::Error::from_raw_os_error(target_errno.abs()))
        },
        // NOTE: The C helper contract excludes positive status values.
        _ => unreachable!(),
    }
}

/// Copy caller-owned bytes into one mapped region.
#[inline]
pub fn write(mapping: &mut [u8], offset: usize, source: &[u8]) -> io::Result<()> {
    let mapping_size = mapping
        .len()
        .try_into()
        .or(Err(io::Error::from(io::ErrorKind::InvalidInput)))?;
    let mapping_offset = offset
        .try_into()
        .or(Err(io::Error::from(io::ErrorKind::InvalidInput)))?;
    let data_size = source
        .len()
        .try_into()
        .or(Err(io::Error::from(io::ErrorKind::InvalidInput)))?;

    // SAFETY:
    //
    // * The mapping is exclusively borrowed, source is readable, and C checks ranges.
    let target_status = unsafe {
        bridge::nekor_kvm_mapping_copy_from(
            mapping.as_mut_ptr().cast(),
            mapping_size,
            mapping_offset,
            source.as_ptr(),
            data_size,
        )
    };

    match target_status {
        bridge::NEKOR_KVM_STATUS_OK => Ok(()),
        target_errno @ bridge::nekor_kvm_status_t::MIN..bridge::NEKOR_KVM_STATUS_OK => {
            Err(io::Error::from_raw_os_error(target_errno.abs()))
        },
        // NOTE: The C helper contract excludes positive status values.
        _ => unreachable!(),
    }
}

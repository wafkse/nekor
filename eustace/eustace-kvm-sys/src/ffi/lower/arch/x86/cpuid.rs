//! Query supported CPUID entries and install virtual-processor CPUID configurations.

use core::mem::MaybeUninit;
use std::{
    io,
    os::fd::{AsRawFd, BorrowedFd},
};

use crate::ffi::binding::bridge;

/// A CPUID query and its returned register values.
pub type Entry = bridge::nekor_kvm_cpuid_entry_t;

/// Leaf, subleaf, and attributes of a CPUID query.
pub type Query = bridge::nekor_kvm_cpuid_query_t;

/// Register values returned for a CPUID query.
pub type Registers = bridge::nekor_kvm_cpuid_registers_t;

/// Populate the supported CPUID entry list and return its initialized prefix length.
///
/// Insufficient output capacity returns `E2BIG`. Failure leaves the entry list unchanged.
///
/// # Safety
///
/// For soundness purposes, the following must be satisfied:
///
/// * The provided file descriptor must identify a KVM device.
#[inline]
pub unsafe fn supported(device: BorrowedFd<'_>, entry_list: &mut [MaybeUninit<Entry>]) -> io::Result<usize> {
    let entry_capacity = entry_list
        .len()
        .try_into()
        .or(Err(io::Error::from(io::ErrorKind::InvalidInput)))?;

    let mut entry_count = MaybeUninit::<bridge::nekor_kvm_storage_size_t>::uninit();

    // SAFETY:
    //
    // * The caller supplies a KVM device. The slice bounds writable entry storage, and the count
    //   output remains live for the call. C writes entries only after validating the result.
    let target_status = unsafe {
        bridge::nekor_kvm_cpuid_supported(
            device.as_raw_fd(),
            entry_list.as_mut_ptr().cast(),
            entry_capacity,
            entry_count.as_mut_ptr(),
        )
    };

    match target_status {
        bridge::NEKOR_KVM_STATUS_OK => {
            // SAFETY:
            //
            // * The C helper initializes the count before reporting success.
            let entry_count = unsafe { entry_count.assume_init() };

            let entry_count = usize::try_from(entry_count).or(Err(io::Error::from(io::ErrorKind::InvalidData)))?;

            match entry_list.get(..entry_count) {
                Some(_) => Ok(entry_count),
                None => Err(io::Error::from(io::ErrorKind::InvalidData)),
            }
        },
        target_errno @ bridge::nekor_kvm_status_t::MIN..bridge::NEKOR_KVM_STATUS_OK => {
            Err(io::Error::from_raw_os_error(target_errno.abs()))
        },
        // NOTE: The C helper contract excludes positive status values.
        _ => unreachable!(),
    }
}

/// Install the entry list as the selected vCPU's CPUID configuration.
///
/// No pointer into the caller's entry list is retained after the call.
///
/// # Safety
///
/// For soundness purposes, the following must be satisfied:
///
/// * The provided file descriptor must identify a stopped KVM virtual processor.
#[inline]
pub unsafe fn install(vcpu: BorrowedFd<'_>, entry_list: &[Entry]) -> io::Result<()> {
    let entry_count = entry_list
        .len()
        .try_into()
        .or(Err(io::Error::from(io::ErrorKind::InvalidInput)))?;

    // SAFETY:
    //
    // * The caller supplies a stopped KVM vCPU, and the slice bounds the readable entry list.
    let target_status = unsafe { bridge::nekor_kvm_cpuid_install(vcpu.as_raw_fd(), entry_list.as_ptr(), entry_count) };

    match target_status {
        bridge::NEKOR_KVM_STATUS_OK => Ok(()),
        target_errno @ bridge::nekor_kvm_status_t::MIN..bridge::NEKOR_KVM_STATUS_OK => {
            Err(io::Error::from_raw_os_error(target_errno.abs()))
        },
        // NOTE: The C helper contract excludes positive status values.
        _ => unreachable!(),
    }
}

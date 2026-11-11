//! Guest memory registration through the KVM VM descriptor.

use core::ffi::c_void;
use std::{
    io,
    os::fd::{AsRawFd, BorrowedFd},
};

use crate::ffi::binding::bridge;

/// KVM memory-slot identifier, including address-space selection bits.
pub type Slot = bridge::nekor_kvm_memory_slot_t;

/// Guest physical address of a registered memory region.
pub type Gpa = bridge::nekor_kvm_guest_physical_address_t;

/// Register the entire host byte region as a KVM guest memory slot.
///
/// # Safety
///
/// For soundness purposes, the following must be satisfied:
///
/// * The provided file descriptor must identify a KVM virtual machine.
/// * The backing memory must remain mapped and inaccessible to conflicting Rust borrows until the
///   VM stops using the memory slot.
#[inline]
pub unsafe fn register(vm: BorrowedFd<'_>, slot: Slot, guest_address: Gpa, host_mapping: &[u8]) -> io::Result<()> {
    let memory_size = host_mapping
        .len()
        .try_into()
        .or(Err(io::Error::from(io::ErrorKind::InvalidInput)))?;

    // SAFETY:
    //
    // * The caller has asserted that the provided file descriptor identifies a KVM virtual machine.
    // * The caller guarantees that the backing memory remains mapped without conflicting Rust
    //   borrows until the virtual machine stops using the slot.
    let target_status = unsafe {
        bridge::nekor_kvm_guest_memory_register(
            vm.as_raw_fd(),
            slot,
            guest_address,
            memory_size,
            host_mapping.as_ptr().cast::<c_void>(),
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

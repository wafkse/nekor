//! Host page-size queries for KVM memory mappings.

use core::{mem::MaybeUninit, num::NonZeroUsize};
use std::io;

use crate::ffi::binding::bridge;

/// Read the host page size used to align guest memory regions.
#[inline]
pub fn page_size() -> io::Result<NonZeroUsize> {
    let mut page_size = MaybeUninit::<bridge::nekor_kvm_storage_size_t>::uninit();

    // SAFETY:
    //
    // * `page_size` is writable output storage.
    let target_status = unsafe { bridge::nekor_kvm_host_page_size_read(page_size.as_mut_ptr()) };

    match target_status {
        bridge::NEKOR_KVM_STATUS_OK => {
            // SAFETY:
            //
            // * The C helper initializes the page size before reporting success.
            let page_size = unsafe { page_size.assume_init() };

            let page_size = usize::try_from(page_size).or(Err(io::Error::from(io::ErrorKind::InvalidData)))?;

            NonZeroUsize::new(page_size).ok_or_else(|| io::Error::from(io::ErrorKind::InvalidData))
        },
        target_errno @ bridge::nekor_kvm_status_t::MIN..bridge::NEKOR_KVM_STATUS_OK => {
            Err(io::Error::from_raw_os_error(target_errno.abs()))
        },
        // NOTE: The C helper contract excludes positive status values.
        _ => unreachable!(),
    }
}

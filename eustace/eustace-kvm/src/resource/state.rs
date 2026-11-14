//! State and context record models for the KVM-based backend.

use alloc::ffi::CString;
#[cfg(any(feature = "stable", feature = "lts"))]
use alloc::vec::Vec;
#[cfg(any(feature = "stable", feature = "lts"))]
use core::num::NonZeroUsize;
#[cfg(any(feature = "stable", feature = "lts"))]
use std::io::ErrorKind;
use std::{
    os::{
        fd::{AsFd, OwnedFd},
        unix::ffi::OsStrExt,
    },
    path::Path,
};

use eustace_hypervisor::baseline::Backend;
use eustace_model::known::X86_64;

#[cfg(not(any(feature = "stable", feature = "lts")))]
use crate::error::Result;
#[cfg(any(feature = "stable", feature = "lts"))]
use crate::{
    arch::x86::cpuid::CpuidEntry,
    error::{KvmError, Result},
    sys::ffi::{binding::kvm::preferred as kvm, lower::arch::x86::cpuid},
};
use crate::{error::KvmError, sys::ffi::lower::device};
#[cfg(any(feature = "stable", feature = "lts"))]

/// An owned descriptor for the system-wide KVM interface.
///
/// # On KVM API Versioning
///
/// The selected binding feature pins the KVM API version expected by this descriptor.
///
/// With a KVM binding feature enabled, opening validates that API version and obtains the host page
/// size and vCPU run-map extent. It does not probe optional KVM capabilities.
// NOTE(invariant): With a KVM binding feature enabled, the descriptor reports the pinned API
// version and valid mapping sizes.
pub struct Kvm {
    /// Open KVM device descriptor owned by this handle.
    fd: OwnedFd,

    /// Run mapping size reported by KVM.
    #[cfg(any(feature = "stable", feature = "lts"))]
    run_mapping_size: NonZeroUsize,

    /// Host page size used to align guest RAM extents.
    #[cfg(any(feature = "stable", feature = "lts"))]
    page_size: NonZeroUsize,
}

impl Kvm {
    /// The canonical location of the KVM character device.
    pub const DEVICE_PATH: &str = "/dev/kvm";

    /// Open a file descriptor to the system-wide KVM character device.
    #[inline]
    pub fn open() -> Result<Self> {
        Self::open_at(Path::new(Self::DEVICE_PATH))
    }

    /// Open and validate the KVM character device at the specified path.
    #[inline]
    pub fn open_at(target_path: &Path) -> Result<Self> {
        let device_path = CString::new(target_path.as_os_str().as_bytes()).or(Err(KvmError::InvalidDevicePath))?;
        let fd = device::open(&device_path)?;

        #[cfg(any(feature = "stable", feature = "lts"))]
        {
            // SAFETY:
            //
            // * This resource owns the matching KVM descriptor and its live backing storage.
            let api_version = unsafe { device::version(fd.as_fd()) }?;

            if api_version != kvm::KVM_API_VERSION.cast_signed() {
                return Err(KvmError::UnexpectedApiVersion(api_version));
            }

            // SAFETY:
            //
            // * Opening this handle established the KVM device descriptor contract.
            let run_size = unsafe { device::run::size(fd.as_fd()) }?;
            let run_mapping_size = NonZeroUsize::new(run_size).ok_or(KvmError::InvalidRunMappingSize)?;
            let page_size = host::pagesize()?;

            if run_mapping_size.get() < page_size.get()
                || run_mapping_size.get() > isize::MAX as usize
                || !run_mapping_size.get().is_multiple_of(page_size.get())
            {
                return Err(KvmError::InvalidRunMappingSize);
            }

            Ok(Self {
                fd,
                run_mapping_size,
                page_size,
            })
        }

        #[cfg(not(any(feature = "stable", feature = "lts")))]
        Ok(Self { fd })
    }

    /// Return the vCPU run-mapping extent reported by KVM.
    #[cfg(any(feature = "stable", feature = "lts"))]
    #[inline]
    #[must_use]
    pub const fn run_mapping_size(&self) -> NonZeroUsize {
        let Self { run_mapping_size, .. } = self;

        *run_mapping_size
    }

    /// Return the host page size used to align guest memory mappings.
    #[cfg(any(feature = "stable", feature = "lts"))]
    #[inline]
    #[must_use]
    pub const fn page_size(&self) -> NonZeroUsize {
        let Self { page_size, .. } = self;

        *page_size
    }

    /// Return the CPUID entry list supported by this KVM device.
    ///
    /// Queries begin with capacity for 64 entries and permit at most eight attempts. An `E2BIG`
    /// response doubles the output capacity. Success returns the initialized entries. Allocation
    /// and kernel failures are returned to the caller.
    #[cfg(any(feature = "stable", feature = "lts"))]
    #[inline]
    pub fn cpuid(&self) -> Result<Vec<CpuidEntry>> {
        let Self { fd, .. } = self;

        let mut entry_capacity = 64_usize;
        let mut attempt = usize::MIN;

        loop {
            let mut bridge_entry_list = Vec::<cpuid::Entry>::new();

            bridge_entry_list
                .try_reserve_exact(entry_capacity)
                .or(Err(KvmError::AllocationFailure))?;

            // SAFETY:
            //
            // * This handle owns a validated KVM device. The spare capacity is writable output
            //   storage, and its length bounds the list passed to C.
            let outcome = unsafe { cpuid::supported(fd.as_fd(), bridge_entry_list.spare_capacity_mut()) };

            match outcome {
                Ok(entry_count) => {
                    let mut entry_list = Vec::new();

                    entry_list
                        .try_reserve_exact(entry_count)
                        .or(Err(KvmError::AllocationFailure))?;

                    for entry in bridge_entry_list.spare_capacity_mut().iter().take(entry_count) {
                        // SAFETY:
                        //
                        // * Success initializes exactly this bounded prefix. Bridge entries contain
                        //   plain values and no owned resources requiring destruction.
                        let entry = unsafe { entry.assume_init_read() };

                        entry_list.push(CpuidEntry::lift(entry));
                    }

                    return Ok(entry_list);
                },
                Err(error) => match (error.kind(), attempt) {
                    (ErrorKind::ArgumentListTooLong, usize::MIN..7) => {
                        entry_capacity = entry_capacity.checked_mul(2).ok_or(KvmError::SizeOverflow)?;
                        attempt += 1;
                    },
                    _ => return Err(error.into()),
                },
            }
        }
    }
}

impl Backend<X86_64> for Kvm {}

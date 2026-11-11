//! Registered guest RAM and bounded host access while its vCPU is stopped.

use core::{convert::TryFrom, num::NonZeroUsize, ptr::NonNull, slice};
use std::os::fd::{AsFd, OwnedFd};

use nekor_virt_address::gpa::Gpa;
use nekor_virt_memory::region::GuestMemoryRegion;

use crate::{
    error::{KvmError, Result},
    sys::ffi::lower::{mapping, vm},
};

/// A host mapping registered as one KVM guest memory slot.
// NOTE(invariant): The registered extent remains mapped until its VM descriptor closes.
pub struct RegisteredRam {
    /// Guest address extent registered with KVM.
    region: GuestMemoryRegion,

    /// Base address of the host mapping backing the guest physical extent.
    mapping: NonNull<u8>,

    /// Exact nonzero extent of the host mapping.
    mapping_size: NonZeroUsize,
}

impl RegisteredRam {
    /// Validate a guest region against the host page size and return its allocation length.
    pub fn extent(region: GuestMemoryRegion, page_size: NonZeroUsize) -> Result<NonZeroUsize> {
        let page_size_u64 = u64::try_from(page_size.get()).or(Err(KvmError::SizeOverflow))?;

        if !region.start().value().is_multiple_of(page_size_u64) || !region.size().value().is_multiple_of(page_size_u64)
        {
            return Err(KvmError::InvalidMemoryRegion);
        }

        let byte_length = usize::try_from(region.size().value()).or(Err(KvmError::SizeOverflow))?;

        if byte_length > isize::MAX as usize {
            return Err(KvmError::SizeOverflow);
        }

        let byte_length = NonZeroUsize::new(byte_length).ok_or(KvmError::InvalidMemoryRegion)?;

        Ok(byte_length)
    }

    /// Map and register one contiguous guest RAM extent.
    pub fn register(
        fd: &OwnedFd,
        slot: vm::memory::Slot,
        region: GuestMemoryRegion,
        byte_length: NonZeroUsize,
    ) -> Result<Self> {
        let mapping = mapping::anonymous(byte_length)?;

        // SAFETY:
        //
        // * The C bridge returned a live mapping covering `byte_length` bytes.
        let host_mapping = unsafe { slice::from_raw_parts(mapping.as_ptr(), byte_length.get()) };

        let value = region.start().value();

        let guest_address = vm::memory::Gpa { value };

        // SAFETY:
        //
        // * The VM is live and this owner keeps the mapping live until the VM closes.
        let registration = unsafe { vm::memory::register(fd.as_fd(), slot, guest_address, host_mapping) };

        if let Err(target_error) = registration {
            // SAFETY:
            //
            // * This function still owns the exact mapping returned above.
            unsafe { mapping::release(mapping, byte_length) };

            return Err(target_error.into());
        }

        Ok(Self {
            region,
            mapping,
            mapping_size: byte_length,
        })
    }

    /// Return whether this region overlaps the requested guest address extent.
    #[inline]
    pub const fn overlaps(&self, region: GuestMemoryRegion) -> bool {
        let Self {
            region: current_region, ..
        } = self;

        current_region.overlaps(region)
    }

    /// Return whether this region contains the complete requested address range.
    #[inline]
    pub fn contains_range(&self, guest_start: Gpa, byte_length: usize) -> Result<bool> {
        let Self { region, .. } = self;

        let requested_length = u64::try_from(byte_length).or(Err(KvmError::SizeOverflow))?;

        Ok(region.offset_of(guest_start, requested_length).is_some())
    }

    /// Return whether this region contains the specified guest byte.
    #[inline]
    pub const fn contains_address(&self, guest_address: Gpa) -> bool {
        let Self { region, .. } = self;

        region.contains(guest_address)
    }

    /// Copy a guest memory range into a host-owned destination.
    pub fn read(&self, guest_start: Gpa, destination: &mut [u8]) -> Result<()> {
        let Self {
            region,
            mapping,
            mapping_size,
        } = self;

        let offset = Self::offset(guest_start, destination.len(), *region)?;
        // SAFETY:
        //
        // * The owned mapping remains live for its exact stored extent.
        let host_mapping = unsafe { slice::from_raw_parts(mapping.as_ptr(), mapping_size.get()) };

        Ok(mapping::read(host_mapping, offset, destination)?)
    }

    /// Copy a host source into a guest memory range.
    pub fn write(&mut self, guest_start: Gpa, source: &[u8]) -> Result<()> {
        let Self {
            region,
            mapping,
            mapping_size,
        } = self;

        let offset = Self::offset(guest_start, source.len(), *region)?;
        // SAFETY:
        //
        // * The owned mapping remains live for its exact stored extent and is exclusively borrowed.
        let host_mapping = unsafe { slice::from_raw_parts_mut(mapping.as_ptr(), mapping_size.get()) };

        Ok(mapping::write(host_mapping, offset, source)?)
    }

    /// Convert a guest address range into a checked host mapping offset.
    fn offset(guest_start: Gpa, byte_length: usize, region: GuestMemoryRegion) -> Result<usize> {
        let requested_length = u64::try_from(byte_length).or(Err(KvmError::SizeOverflow))?;
        let offset = region
            .offset_of(guest_start, requested_length)
            .ok_or(KvmError::GuestMemoryUnmapped)?;

        usize::try_from(offset).or(Err(KvmError::SizeOverflow))
    }
}

impl Drop for RegisteredRam {
    #[inline]
    fn drop(&mut self) {
        let Self {
            mapping, mapping_size, ..
        } = self;

        // SAFETY:
        //
        // * This value uniquely owns the live mapping and records the byte extent used to create
        //   it.
        unsafe { mapping::release(*mapping, *mapping_size) };
    }
}

//! KVM virtual-machine ownership and guest RAM access.

use core::{convert::TryFrom, num::NonZeroUsize};
use std::os::fd::{AsFd, OwnedFd};

use eustace_address::gpa::Gpa;
use eustace_memory::region::GuestMemoryRegion;

use super::{memory::RegisteredRam, vcpu::Vcpu};
use crate::{
    error::{KvmError, Result},
    sys::ffi::lower::{mapping, vcpu, vm},
};

/// An owned KVM virtual machine with its registered guest RAM.
// NOTE(invariant): The KVM descriptor closes before its registered host RAM mappings are unmapped.
pub struct Machine {
    /// KVM VM descriptor. It precedes `memory_regions` so that KVM drops its references first.
    fd: OwnedFd,

    /// Host mappings registered as the VM's guest physical memory.
    memory_regions: Vec<RegisteredRam>,

    /// Extent required for each vCPU run mapping.
    run_mapping_size: NonZeroUsize,

    /// Whether KVM has accepted creation of this VM's single vCPU.
    vcpu_created: bool,

    /// Host page size used for RAM mapping and guest physical alignment.
    page_size: NonZeroUsize,
}

impl Machine {
    /// Register zero-filled guest RAM described by a guest address region.
    ///
    /// KVM requires the region start and size to align to the host page size. Each call adds one
    /// contiguous region. Regions cannot overlap and remain registered until this VM is dropped.
    /// The MVP uses monotonically assigned KVM memory slots.
    #[inline]
    pub fn register_ram(&mut self, region: GuestMemoryRegion) -> Result<()> {
        let Self {
            fd,
            memory_regions,
            page_size,
            ..
        } = self;

        let byte_length = RegisteredRam::extent(region, *page_size)?;

        if memory_regions
            .iter()
            .any(|current_region| current_region.overlaps(region))
        {
            return Err(KvmError::OverlappingMemoryRegion);
        }

        let slot = u16::try_from(memory_regions.len()).or(Err(KvmError::InvalidMemoryRegion))?;

        let slot = vm::memory::Slot::from(slot);

        memory_regions.try_reserve(1).or(Err(KvmError::AllocationFailure))?;

        let memory_region = RegisteredRam::register(fd, slot, region, byte_length)?;

        memory_regions.push(memory_region);

        Ok(())
    }

    /// Copy bytes from a registered guest physical address range.
    #[inline]
    pub fn read_memory(&self, guest_start: Gpa, destination: &mut [u8]) -> Result<()> {
        let Self { memory_regions, .. } = self;

        for memory_region in memory_regions {
            if memory_region.contains_range(guest_start, destination.len())? {
                return memory_region.read(guest_start, destination);
            }
        }

        Err(KvmError::GuestMemoryUnmapped)
    }

    /// Copy bytes into a registered guest physical address range.
    #[inline]
    pub fn write_memory(&mut self, guest_start: Gpa, source: &[u8]) -> Result<()> {
        let Self { memory_regions, .. } = self;

        for memory_region in memory_regions {
            if memory_region.contains_range(guest_start, source.len())? {
                return memory_region.write(guest_start, source);
            }
        }

        Err(KvmError::GuestMemoryUnmapped)
    }

    /// Create the MVP's single vCPU and exclusively borrow this VM until it is dropped.
    #[inline]
    pub fn create_vcpu(&mut self) -> Result<Vcpu<'_>> {
        let Self {
            fd,
            run_mapping_size,
            vcpu_created,
            ..
        } = self;

        if *vcpu_created {
            return Err(KvmError::VcpuAlreadyCreated);
        }

        let run_mapping_size = *run_mapping_size;
        // SAFETY:
        //
        // * This resource owns the matching KVM descriptor and its live backing storage.
        let vcpu_fd = unsafe { vcpu::create(fd.as_fd(), 0) }?;
        // SAFETY:
        //
        // * This resource owns the matching KVM descriptor and its live backing storage.
        let run_mapping = unsafe { mapping::shared(vcpu_fd.as_fd(), run_mapping_size) }?;

        *vcpu_created = true;

        // SAFETY:
        //
        // * The C helpers created this vCPU and its matching complete run mapping.
        Ok(unsafe { Vcpu::new(self, vcpu_fd, run_mapping, run_mapping_size) })
    }

    /// Create a VM record from a descriptor and the host mapping parameters validated by `Kvm`.
    ///
    /// # Safety
    ///
    /// For soundness purposes, the following must be satisfied:
    ///
    /// * The caller must uniquely own the provided KVM virtual machine descriptor.
    /// * The run mapping extent and page size must come from the validated KVM device used to
    ///   create the virtual machine.
    /// * The run mapping extent must fit a Rust slice.
    #[inline]
    pub const unsafe fn new(fd: OwnedFd, run_mapping_size: NonZeroUsize, page_size: NonZeroUsize) -> Self {
        Self {
            fd,
            memory_regions: Vec::new(),
            run_mapping_size,
            vcpu_created: false,
            page_size,
        }
    }

    /// Return whether the VM has a registered byte at the guest physical address.
    #[inline]
    pub fn contains_guest_address(&self, guest_address: Gpa) -> bool {
        let Self { memory_regions, .. } = self;

        memory_regions
            .iter()
            .any(|region| region.contains_address(guest_address))
    }
}

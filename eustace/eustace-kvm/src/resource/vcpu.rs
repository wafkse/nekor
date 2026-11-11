//! Exclusive vCPU control, real-mode entry setup, and exit settlement.

use alloc::vec::Vec;
use core::{convert::TryFrom, num::NonZeroUsize, ptr::NonNull, slice};
use std::os::fd::{AsFd, OwnedFd};

use eustace_address::gpa::Gpa;
use eustace_arch::x86::entry::RealModeEntryPoint;
use eustace_ioport::access::{IoDirection, IoPortAccess};
use eustace_mmio::access::MmioAccess;

use super::{machine::Machine, run::RunImage};
use crate::{
    arch::x86::cpuid::CpuidEntry,
    error::{KvmError, Result},
    run::RunExit,
    sys::ffi::lower::{arch::x86, mapping, vcpu},
};

/// A vCPU whose exclusive borrow keeps its VM and RAM alive while it runs.
// NOTE(invariant): The C-created run image is unmapped before its vCPU descriptor closes, and the
// VM stays exclusively borrowed.
pub struct Vcpu<'machine> {
    /// Base address of the shared KVM run image owned by this vCPU.
    run_mapping: NonNull<u8>,

    /// Exact byte extent of the shared run image.
    run_mapping_size: NonZeroUsize,

    /// KVM vCPU descriptor associated with the shared run mapping.
    fd: OwnedFd,

    /// Exclusively borrowed machine retaining the registered guest RAM mappings.
    machine: &'machine mut Machine,

    /// Whether a valid real-mode entry point has been installed.
    entry_configured: bool,

    /// Guest response required before another run ioctl may be issued.
    pending_response: Option<PendingResponse>,

    /// Most recent exit returned to the caller.
    last_exit: Option<RunExit>,
}

/// Response state that must be settled before the next KVM run ioctl.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingResponse {
    /// Port-input bytes requested by the guest.
    IoInput {
        /// Number of bytes requested by the guest operation.
        byte_count: usize,
    },

    /// MMIO-read bytes requested by the guest.
    MmioRead {
        /// Number of bytes requested by the guest operation.
        byte_count: usize,
    },
}

impl<'machine> Vcpu<'machine> {
    /// Install the supplied CPUID entry list while this vCPU is stopped.
    ///
    /// An outstanding guest response must be settled before configuration changes. The caller
    /// continues to own the entry list.
    #[inline]
    pub fn cpuid(&mut self, entry_list: &[CpuidEntry]) -> Result<()> {
        let Self {
            fd, pending_response, ..
        } = self;

        match pending_response {
            None => {
                let mut bridge_entry_list = Vec::new();

                bridge_entry_list
                    .try_reserve_exact(entry_list.len())
                    .or(Err(KvmError::AllocationFailure))?;

                for entry in entry_list {
                    bridge_entry_list.push(entry.raw());
                }

                // SAFETY:
                //
                // * This resource owns a stopped KVM vCPU. C borrows the bridge list only during
                //   this call and releases its native image before returning.
                unsafe { x86::cpuid::install(fd.as_fd(), &bridge_entry_list) }?;

                Ok(())
            },
            Some(_) => Err(KvmError::UnsettledExitResponse),
        }
    }

    /// Copy bytes from registered guest RAM while the vCPU is stopped.
    #[inline]
    pub fn read_memory(&self, guest_start: Gpa, destination: &mut [u8]) -> Result<()> {
        let Self { machine, .. } = self;

        machine.read_memory(guest_start, destination)
    }

    /// Copy bytes into registered guest RAM while the vCPU is stopped.
    #[inline]
    pub fn write_memory(&mut self, guest_start: Gpa, source: &[u8]) -> Result<()> {
        let Self { machine, .. } = self;

        machine.write_memory(guest_start, source)
    }

    /// Configure a real-mode entry point in the first 64 KiB of mapped guest RAM.
    ///
    /// This sets the code segment to a flat real-mode segment and updates the instruction pointer.
    /// The vCPU must be stopped. Callers can install guest bytes through [`Machine::write_memory`]
    /// before creating the vCPU or through [`Vcpu::write_memory`] while it is stopped.
    #[inline]
    pub fn set_real_mode_entry_point(&mut self, entry_point: RealModeEntryPoint) -> Result<()> {
        let Self {
            fd,
            machine,
            entry_configured,
            pending_response,
            ..
        } = self;

        if pending_response.is_some() {
            return Err(KvmError::UnsettledExitResponse);
        }

        let guest_address = Gpa::new(u64::from(entry_point.value()));

        if !machine.contains_guest_address(guest_address) {
            return Err(KvmError::GuestMemoryUnmapped);
        }

        // SAFETY:
        //
        // * This resource owns the matching KVM descriptor and its live backing storage.
        unsafe { x86::vcpu::entry(fd.as_fd(), entry_point.value()) }?;

        *entry_configured = true;

        Ok(())
    }

    /// Run the configured guest until KVM returns an exit or an operating-system error occurs.
    #[inline]
    pub fn run(&mut self) -> Result<RunExit> {
        let Self {
            fd,
            run_mapping,
            run_mapping_size,
            entry_configured,
            pending_response,
            last_exit,
            ..
        } = self;

        if !*entry_configured {
            return Err(KvmError::VcpuNotConfigured);
        }

        if pending_response.is_some() {
            return Err(KvmError::UnsettledExitResponse);
        }

        *last_exit = None;

        let run_mapping = mapping_bytes(run_mapping, *run_mapping_size);

        // SAFETY:
        //
        // * This resource owns the matching KVM descriptor and its live backing storage.
        unsafe { vcpu::run(fd.as_fd(), run_mapping) }?;

        let run_image = RunImage::new(run_mapping)?;
        let exit = run_image.exit()?;

        *pending_response = Self::response_for(exit)?;
        *last_exit = Some(exit);

        Ok(exit)
    }

    /// Copy guest port-output bytes from the most recent output exit.
    #[inline]
    pub fn io_output_read(&mut self, destination: &mut [u8]) -> Result<usize> {
        let Self {
            run_mapping,
            run_mapping_size,
            last_exit,
            ..
        } = self;

        match last_exit {
            Some(RunExit::Io(IoPortAccess {
                direction: IoDirection::Out,
                ..
            })) => (),
            _ => return Err(KvmError::WrongExitKind),
        }

        let run_image = RunImage::new(mapping_bytes(run_mapping, *run_mapping_size))?;

        run_image.io_output_read(destination)
    }

    /// Supply the exact byte sequence requested by the most recent port-input exit.
    #[inline]
    pub fn io_input_write(&mut self, source: &[u8]) -> Result<()> {
        let Self {
            run_mapping,
            run_mapping_size,
            pending_response,
            ..
        } = self;

        let Some(PendingResponse::IoInput { byte_count }) = *pending_response else {
            return Err(KvmError::WrongExitKind);
        };

        if source.len() != byte_count {
            return Err(KvmError::WrongResponseSize {
                expected: byte_count,
                actual: source.len(),
            });
        }

        let run_mapping = mapping_bytes(run_mapping, *run_mapping_size);
        let mut run_image = RunImage::new(run_mapping)?;

        run_image.io_input_write(source)?;
        *pending_response = None;

        Ok(())
    }

    /// Supply the exact byte sequence requested by the most recent MMIO-read exit.
    #[inline]
    pub fn mmio_read_response_write(&mut self, source: &[u8]) -> Result<()> {
        let Self {
            run_mapping,
            run_mapping_size,
            pending_response,
            ..
        } = self;

        let Some(PendingResponse::MmioRead { byte_count }) = *pending_response else {
            return Err(KvmError::WrongExitKind);
        };

        if source.len() != byte_count {
            return Err(KvmError::WrongResponseSize {
                expected: byte_count,
                actual: source.len(),
            });
        }

        let run_mapping = mapping_bytes(run_mapping, *run_mapping_size);
        let mut run_image = RunImage::new(run_mapping)?;

        run_image.mmio_read_response_write(source)?;
        *pending_response = None;

        Ok(())
    }

    /// Return the latest exit, if this vCPU has returned from a run ioctl.
    #[inline]
    #[must_use]
    pub const fn last_exit(&self) -> Option<RunExit> {
        let Self { last_exit, .. } = self;

        *last_exit
    }

    /// Build the response state required by an exit, if it asks userspace for data.
    fn response_for(exit: RunExit) -> Result<Option<PendingResponse>> {
        match exit {
            RunExit::Io(IoPortAccess {
                direction: IoDirection::In,
                element_size,
                element_count,
                ..
            }) => {
                let element_count = usize::try_from(element_count.value()).or(Err(KvmError::SizeOverflow))?;
                let byte_count = element_count
                    .checked_mul(element_size.byte_count())
                    .ok_or(KvmError::SizeOverflow)?;

                Ok(Some(PendingResponse::IoInput { byte_count }))
            },
            RunExit::Mmio(MmioAccess::Read(read_exit)) => Ok(Some(PendingResponse::MmioRead {
                byte_count: usize::from(read_exit.byte_count.value()),
            })),
            RunExit::Io(_) | RunExit::Mmio(MmioAccess::Write(_)) | RunExit::Halt | RunExit::Other(_) => Ok(None),
        }
    }

    /// Construct a vCPU around its KVM descriptor and mapped run image.
    ///
    /// # Safety
    ///
    /// For soundness purposes, the following must be satisfied:
    ///
    /// * The provided file descriptor must uniquely own a KVM virtual processor belonging to the
    ///   borrowed virtual machine.
    /// * The provided address must transfer unique ownership of that virtual processor's complete
    ///   writable shared run mapping.
    /// * The provided byte extent must describe the complete mapping and fit a Rust slice.
    #[inline]
    pub const unsafe fn new(
        machine: &'machine mut Machine,
        fd: OwnedFd,
        run_mapping: NonNull<u8>,
        run_mapping_size: NonZeroUsize,
    ) -> Self {
        Self {
            run_mapping,
            run_mapping_size,
            fd,
            machine,
            entry_configured: false,
            pending_response: None,
            last_exit: None,
        }
    }
}

impl Drop for Vcpu<'_> {
    #[inline]
    fn drop(&mut self) {
        let Self {
            run_mapping,
            run_mapping_size,
            ..
        } = self;

        // SAFETY:
        //
        // * This vCPU uniquely owns the C-created run mapping and its original extent.
        unsafe { mapping::release(*run_mapping, *run_mapping_size) };
    }
}

/// Borrow the complete mapping extent owned by a KVM resource.
#[inline]
const fn mapping_bytes(mapping: &mut NonNull<u8>, mapping_size: NonZeroUsize) -> &mut [u8] {
    // SAFETY:
    //
    // * Callers pass the live pointer and original nonzero extent returned by the C mapper.
    unsafe { slice::from_raw_parts_mut(mapping.as_ptr(), mapping_size.get()) }
}

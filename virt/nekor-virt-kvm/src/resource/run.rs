//! Checked access to a mapped KVM run image.
//!
//! Exit inspection validates the active operation's fields and data extent. Input and MMIO-read
//! responses must be written before the next KVM run operation.

use nekor_virt_address::gpa::Gpa;
use nekor_virt_ioport::{
    access::{IoDirection, IoElementCount, IoElementSize, IoPortAccess},
    port::IoPort,
};
use nekor_virt_mmio::access::{MmioAccess, MmioByteCount, MmioRead, MmioWrite};

use crate::{
    error::{KvmError, Result},
    run::{ExitReason, RunExit},
    sys::ffi::{binding::bridge, lower::run as abi},
};

/// An exclusive byte view over a KVM run mapping.
// NOTE(invariant): `bytes` is large and aligned enough for the C-validated run image.
pub struct RunImage<'mapping> {
    /// Exclusively borrowed host mapping validated for the target KVM run layout.
    mapping: &'mapping mut [u8],
}

impl<'mapping> RunImage<'mapping> {
    /// Validate a byte region for use as a KVM run image.
    ///
    /// Pass the vCPU run mapping to obtain meaningful KVM exit values. This checks only the
    /// mapping size and alignment. It does not inspect the image until [`RunImage::exit`].
    ///
    /// The byte region must remain the exclusive view of the run mapping until this value is
    /// dropped. Construct the region only after KVM has returned from its run ioctl, and do not
    /// issue another run ioctl while this view is live.
    #[inline]
    pub fn new(mapping: &'mapping mut [u8]) -> Result<Self> {
        abi::validate(mapping)?;

        Ok(Self { mapping })
    }

    /// Classify the active exit and lift its selected union arm into a typed value.
    #[inline]
    pub fn exit(&self) -> Result<RunExit> {
        let Self { mapping } = self;

        let abi::Outcome {
            kind: exit_kind,
            reason: exit_reason,
        } = abi::exit(mapping)?;

        match exit_kind {
            bridge::NEKOR_KVM_RUN_EXIT_KIND_HALT => Ok(RunExit::Halt),
            bridge::NEKOR_KVM_RUN_EXIT_KIND_OTHER => Ok(RunExit::Other(ExitReason::new(exit_reason))),
            bridge::NEKOR_KVM_RUN_EXIT_KIND_IO => self.io_exit(),
            bridge::NEKOR_KVM_RUN_EXIT_KIND_MMIO => self.mmio_exit(),
            target_value => Err(KvmError::UnexpectedExitKind(target_value)),
        }
    }

    /// Copy guest port-output bytes into a userspace-owned slice.
    #[inline]
    pub fn io_output_read(&self, data: &mut [u8]) -> Result<usize> {
        let Self { mapping } = self;

        Ok(abi::io::read(mapping, data)?)
    }

    /// Copy userspace-supplied bytes into a guest port-input operation.
    #[inline]
    pub fn io_input_write(&mut self, data: &[u8]) -> Result<()> {
        let Self { mapping } = self;

        Ok(abi::io::write(mapping, data)?)
    }

    /// Copy userspace response bytes into an active guest MMIO read.
    ///
    /// The response byte count must equal the [`MmioRead::byte_count`] reported by
    /// [`RunImage::exit`]. Complete this operation before the next run ioctl uses the mapping.
    #[inline]
    pub fn mmio_read_response_write(&mut self, data: &[u8]) -> Result<()> {
        let Self { mapping } = self;

        Ok(abi::mmio::respond(mapping, data)?)
    }

    /// Lift the port-I/O arm into its semantic record.
    fn io_exit(&self) -> Result<RunExit> {
        let Self { mapping } = self;

        let io_exit = abi::io::exit(mapping)?;
        let direction = match io_exit.direction {
            bridge::NEKOR_KVM_IO_DIRECTION_IN => IoDirection::In,
            bridge::NEKOR_KVM_IO_DIRECTION_OUT => IoDirection::Out,
            _ => return Err(KvmError::Malformed),
        };
        let element_size = match io_exit.element_size {
            bridge::NEKOR_KVM_IO_ELEMENT_SIZE_BYTE => IoElementSize::Byte,
            bridge::NEKOR_KVM_IO_ELEMENT_SIZE_WORD => IoElementSize::Word,
            bridge::NEKOR_KVM_IO_ELEMENT_SIZE_DWORD => IoElementSize::Dword,
            _ => return Err(KvmError::Malformed),
        };
        let port = IoPort::new(io_exit.port.value);
        let element_count = IoElementCount::new(u64::from(io_exit.element_count)).ok_or(KvmError::Malformed)?;

        Ok(RunExit::Io(IoPortAccess {
            direction,
            element_size,
            port,
            element_count,
        }))
    }

    /// Lift the direction-specific MMIO arm into a semantic record.
    fn mmio_exit(&self) -> Result<RunExit> {
        let Self { mapping } = self;

        let direction = abi::mmio::direction(mapping)?;

        match direction {
            bridge::NEKOR_KVM_MMIO_DIRECTION_READ => self.mmio_read_exit(),
            bridge::NEKOR_KVM_MMIO_DIRECTION_WRITE => self.mmio_write_exit(),
            _ => Err(KvmError::Malformed),
        }
    }

    /// Lift the guest MMIO read request into a semantic record.
    fn mmio_read_exit(&self) -> Result<RunExit> {
        let Self { mapping } = self;

        let mmio_read_exit = abi::mmio::read(mapping)?;
        let address = Gpa::new(mmio_read_exit.address.value);
        let byte_count = MmioByteCount::new(mmio_read_exit.byte_count).ok_or(KvmError::Malformed)?;

        Ok(RunExit::Mmio(MmioAccess::Read(MmioRead { address, byte_count })))
    }

    /// Lift the guest MMIO write payload into a semantic record.
    fn mmio_write_exit(&self) -> Result<RunExit> {
        let Self { mapping } = self;

        let mmio_write_exit = abi::mmio::write(mapping)?;
        let address = Gpa::new(mmio_write_exit.address.value);
        let data = mmio_write_exit.data;
        let byte_count = MmioByteCount::new(mmio_write_exit.byte_count).ok_or(KvmError::Malformed)?;

        Ok(RunExit::Mmio(MmioAccess::Write(MmioWrite {
            address,
            data,
            byte_count,
        })))
    }
}

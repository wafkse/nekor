//! Failures returned by the checked KVM interfaces.

use std::io;

use fack::prelude::Error as FackError;

/// Failure surfaced by the typed KVM backend.
#[derive(Debug, FackError)]
#[error(inline)]
pub enum KvmError {
    /// The operating-system path contains an interior nul byte.
    #[error("the KVM device path contains a nul byte")]
    InvalidDevicePath,

    /// An operating-system operation failed.
    #[error("{0}")]
    #[error(from)]
    Io(io::Error),

    /// The host KVM interface reports an API version outside the pinned UAPI contract.
    #[error("KVM API version `{0}` does not match the pinned interface")]
    UnexpectedApiVersion(i32),

    /// The KVM run mapping size is not a positive value representable by this process.
    #[error("KVM returned an invalid vCPU run-mapping size")]
    InvalidRunMappingSize,

    /// The host did not report a usable system page size.
    #[error("the host did not report a usable system page size")]
    PageSizeUnavailable,

    /// Local arithmetic exceeded the representable byte extent.
    #[error("a KVM storage size exceeded the representable byte extent")]
    SizeOverflow,

    /// A returned KVM record contains an invalid field value.
    #[error("the selected KVM record contains malformed state")]
    Malformed,

    /// The requested CPUID storage could not be allocated.
    #[error("the requested CPUID storage could not be allocated")]
    AllocationFailure,

    /// A guest RAM region is unaligned or its KVM slot is outside the supported range.
    #[error("the guest RAM region is unaligned or its KVM slot is outside the supported range")]
    InvalidMemoryRegion,

    /// A new guest memory extent overlaps an existing registered extent.
    #[error("the guest memory extent overlaps an existing region")]
    OverlappingMemoryRegion,

    /// A requested guest address range is not backed by registered guest memory.
    #[error("the guest address range is not backed by registered memory")]
    GuestMemoryUnmapped,

    /// A vCPU cannot run until its real-mode entry point has been configured.
    #[error("the vCPU real-mode entry point has not been configured")]
    VcpuNotConfigured,

    /// The MVP VM already created its single vCPU.
    #[error("the VM already created its vCPU")]
    VcpuAlreadyCreated,

    /// A response-required guest exit must be completed before the vCPU can run again.
    #[error("the previous guest exit still requires a userspace response")]
    UnsettledExitResponse,

    /// The requested response method does not match the current exit.
    #[error("the requested response does not match the current guest exit")]
    WrongExitKind,

    /// The response length does not match the current guest request.
    #[error("the guest exit requires `{expected}` response bytes but received `{actual}`")]
    WrongResponseSize {
        /// Number of response bytes required by the guest exit.
        expected: usize,

        /// Number of response bytes provided by the caller.
        actual: usize,
    },

    /// Exit inspection returned an unknown classification.
    #[error("the C bridge returned unknown exit classification `{0}`")]
    UnexpectedExitKind(u32),
}

/// Result returned by the checked KVM interfaces.
pub type Result<T> = core::result::Result<T, KvmError>;

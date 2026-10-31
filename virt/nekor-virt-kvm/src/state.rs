//! State and context record models for the KVM-based backend.

use std::{fs::OpenOptions, io, os::fd::OwnedFd, path::Path};

/// An owned KVM file descriptor.
///
/// # On KVM API Versioning
///
/// As the KVM API version code (`KVM_API_VERSION`) has been effectively frozen to `12` since
/// the day the author of this code was born, this is intended to support solely that API version.
///
/// Note that, however, this does not check for the existence of any extensions that may be deemed
/// ubiquitous.
#[repr(transparent)]
// NOTE(invariant): OwnedFd must be a KVM-backed file descriptor.
pub struct Kvm(OwnedFd);

impl Kvm {
    /// The canonical location of the KVM character device.
    pub const DEVICE_PATH: &str = "/dev/kvm";

    /// Open a file descriptor to the system-wide KVM character device.
    #[inline]
    pub fn open() -> Result<Self, io::Error> {
        // SAFETY: The canonical default path is considered to always belong to the KVM subsystem.
        unsafe { Self::open_at(Path::new(Self::DEVICE_PATH)) }
    }

    /// Open a file descriptor to the system-wide KVM character device at the specified path.
    ///
    /// # Safety
    ///
    /// The provided path must be to a KVM-managed character device.
    ///
    /// This represents contract breakage, not memory unsafety. On a system where the the path may
    /// not represent a KVM-managed character device, this type will present itself with broken
    /// invariants.
    #[inline]
    pub unsafe fn open_at(target_path: &Path) -> Result<Self, io::Error> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(target_path)
            .map(OwnedFd::from)
            .map(Self)
    }
}

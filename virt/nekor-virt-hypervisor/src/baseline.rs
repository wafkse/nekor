//! The baseline module for the `nekor-virt-hypervisor` crate.
//!
//! This is used for baseline traits and other items that are required for
//! hypervisor-backend-agnostic work.

use nekor_virt_model::baseline::Arch;

/// A representation of a particular hypervisor backend.
///
/// For example, Linux commonly uses `KVM`, while Windows provides the `Windows Hypervisor Platform`
/// for userspace-managed virtual machines.
pub trait Backend<A>
where
    A: Arch,
{
}

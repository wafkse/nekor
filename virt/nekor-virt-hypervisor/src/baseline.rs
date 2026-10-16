//! The baseline module for the `nekor-virt-hypervisor` crate.
//!
//! This is used for baseline traits and other items that are required for
//! hypervisor-backend-agnostic work.

use nekor_virt_model::baseline::Arch;

/// A representation of a particular hypervisor backend.
///
/// For example, on Linux, `KVM` is generally used. On Windows, the `WHP` (abbreviated from `Windows
/// Hypervisor Example`) permits a usermode-managed virtual-machine.
pub trait Backend<A>
where
    A: Arch,
{
}

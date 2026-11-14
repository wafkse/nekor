# eustace-kvm

KVM system-device access and typed CPUID and guest-exit values for Eustace.

## Use

Open the system KVM device with [`resource::state::Kvm::open`] or
[`resource::state::Kvm::open_at`]. With a `stable` or `lts` feature, the handle validates the pinned
API version, run-mapping extent, and host page size. It also exposes supported CPUID entries through
[`resource::state::Kvm::cpuid`].

Use [`sys::ffi::lower`](../eustace-kvm-sys/README.md) for virtual-machine, virtual-processor, guest
memory, and run-image operations. The caller manages those descriptors and mappings and upholds the
documented safety requirements of unsafe operations.

The crate implements `Backend<X86_64>` and exposes `RunExit` for guest-exit values.

## Boundaries

Linux KVM declarations and host operations belong to
[`eustace-kvm-sys`](../eustace-kvm-sys/README.md). Address, I/O, and MMIO values belong to their
domain crates.

## Status

The default `stable` feature selects the stable kernel pin. Use `--no-default-features --features lts`
to select the LTS pin.

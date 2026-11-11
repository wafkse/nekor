# eustace-kvm-sys

Linux KVM declarations and checked operations for device descriptors, guest memory, and virtual
processors. Resource ownership and execution are managed by
[`eustace-kvm`](../eustace-kvm/README.md).

## Use

Import the selected declarations and bridge ABI from their distinct modules.

```rust
use eustace_kvm_sys::ffi::binding::{bridge, kvm::preferred};
```

The declarations follow the Linux KVM API. See the
[Linux KVM API documentation](https://docs.kernel.org/virt/kvm/api.html) for ioctl arguments and
capability requirements.

## Kernel selection

| Cargo feature | Module | Linux pin |
| --- | --- | --- |
| `stable` (default) | `stable` | `v7.2.12` |
| `lts` | `lts` | `v6.18.58` |

For LTS only, set `default-features = false` and `features = ["lts"]`. If both features are enabled,
`preferred` selects `stable`. Use `stable` or `lts` directly when the selection must be explicit.

## Regenerate

Run the recipes from this crate's directory. The target triple defaults to `CARGO_BUILD_TARGET`, or
the host triple when it is unset.

| Command | Action |
| --- | --- |
| `just checkout <kernel_channel> [target_triple]` | Prepare the pinned kernel headers. |
| `just generate <kernel_channel> [target_triple]` | Regenerate the selected binding pair. |
| `just generate-all [target_triple]` | Regenerate both kernel versions. |
| `just build <kernel_channel> [target_triple]` | Build with the selected kernel version. |

```sh
just generate stable
just generate-all aarch64-unknown-linux-gnu
```

For kernel updates, verify the upstream tag and commit in
[checkout/version.config](checkout/version.config). The [checkout guide](checkout/README.md) covers
header preparation and caches.

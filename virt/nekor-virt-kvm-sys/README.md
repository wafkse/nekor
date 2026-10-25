# nekor-virt-kvm-sys

Raw Linux KVM bindings for code that talks to `/dev/kvm`. This `no_std` crate
provides ioctl request numbers, constants, and C-layout structures with their
upstream names. File descriptors, memory mappings, and ioctl calls belong to the
consumer.

## Use

For a sibling crate under `virt/`, add this dependency.

```toml
[dependencies]
nekor-virt-kvm-sys = { path = "../nekor-virt-kvm-sys" }
```

Import `preferred` to use the selected kernel interface.

```rust
use nekor_virt_kvm_sys::preferred as kvm;

let request = kvm::KVM_GET_API_VERSION;
let expected_version = kvm::KVM_API_VERSION;

assert_eq!(expected_version, 12);
```

`request` is the ioctl number to pass to your syscall interface. Structures such
as `kvm::kvm_run` and `kvm::kvm_userspace_memory_region` describe the corresponding
kernel ABI. See the [Linux KVM API documentation](https://docs.kernel.org/virt/kvm/api.html)
for descriptor types, argument requirements, and capability checks.

## Kernel selection

| Cargo feature | Module | Linux pin |
| --- | --- | --- |
| `stable` (default) | `stable` | `v7.2.12` |
| `lts` | `lts` | `v6.18.58` |

For LTS only, set `default-features = false` and `features = ["lts"]` on the
dependency. Both features may be enabled together. `preferred` selects `stable`
whenever it is enabled, otherwise `lts`. Use `nekor_virt_kvm_sys::stable` or
`nekor_virt_kvm_sys::lts` when the choice must be explicit.

These pins belong to crate version `0.1.0` and remain fixed for that release.
They select the available definitions. Check runtime support with
[`KVM_CHECK_EXTENSION`](https://docs.kernel.org/virt/kvm/api.html#kvm-check-extension)
before using an optional KVM capability.

## Target support

Bindings are included for `x86_64-unknown-linux-gnu` and
`aarch64-unknown-linux-gnu`, for both kernel versions. Cargo selects the binding
for its exact target triple. Generate bindings for another target before building
for it.

Generate for the target that will run the KVM client. Cross-compilation also
requires that target's Rust library, or nightly `build-std` support.

Normal Cargo builds use the included bindings. They require neither a kernel
checkout nor libclang.

## Regenerate

Use the repository toolchain and run these commands from this crate's directory.
Install `just`, Git, GNU Make, a C compiler, rsync, jq, libclang, and rustfmt.

| Command | Action |
| --- | --- |
| `just checkout <channel> [target]` | Prepare the pinned kernel headers. |
| `just generate <channel> [target]` | Prepare headers and regenerate the selected binding. |
| `just generate-all [target]` | Regenerate both kernel versions. |
| `just build <channel> [target]` | Prepare the selected binding and build the crate. |

`<channel>` is `lts` or `stable`. The target defaults to `CARGO_BUILD_TARGET`,
then the host target if that variable is unset.

```sh
just generate stable
just generate-all aarch64-unknown-linux-gnu
just build lts
```

Keep the Cargo lockfile and use the same libclang and rustfmt versions when
comparing output. The `internal-generate-binding` feature is reserved for these
recipes.

For an intentional kernel update, add the new crate-version mapping to
[checkout/version.config](checkout/version.config), verify its tags and commit
hashes against the Linux stable repository, and regenerate each supported target.
Keep released mappings unchanged. The [checkout guide](checkout/README.md)
covers the source and header caches.

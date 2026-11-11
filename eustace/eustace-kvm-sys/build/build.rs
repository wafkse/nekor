//! Build the native bridge and, when requested, regenerate bindings from exported headers.
//!
//! Kernel sources and headers are prepared before Cargo runs this script.

/// Binding generation enabled after the selected Linux userspace headers have been exported.
#[cfg(feature = "internal-generate-binding")]
mod binding;

extern crate alloc;

#[cfg(any(feature = "stable", feature = "lts"))]
mod native;

/// Build scripts watched for changes by Cargo.
const BUILD_RELATIVE_DIRECTORY: &str = "build";

/// Native bridge source and C header trees watched for changes by Cargo.
const C_RELATIVE_DIRECTORY: &str = "c";

/// Kernel release pins used to prepare the exported headers.
const KERNEL_VERSION_RELATIVE_FILE: &str = "checkout/version.config";

/// Public headers shared by native compilation and binding generation.
#[cfg(any(feature = "stable", feature = "lts", feature = "internal-generate-binding"))]
const C_INCLUDE_RELATIVE_DIRECTORY: &str = "c/include";

/// C sources compiled into the bridge archive.
#[cfg(any(feature = "stable", feature = "lts"))]
const NATIVE_SOURCE_RELATIVE_DIRECTORY: &str = "c/src";

/// Exported kernel headers grouped by target and release channel.
#[cfg(any(feature = "stable", feature = "lts", feature = "internal-generate-binding"))]
const KERNEL_HEADER_RELATIVE_DIRECTORY: &str = "checkout/header";

/// Include directory beneath each target and channel's header export.
#[cfg(any(feature = "stable", feature = "lts", feature = "internal-generate-binding"))]
const KERNEL_INCLUDE_RELATIVE_DIRECTORY: &str = "include";

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let target_triple = std::env::var("TARGET")?;

    println!("cargo:rustc-env=NEKOR_KVM_TARGET={target_triple}");
    println!("cargo:rerun-if-changed={BUILD_RELATIVE_DIRECTORY}");
    println!("cargo:rerun-if-changed={C_RELATIVE_DIRECTORY}");
    println!("cargo:rerun-if-changed={KERNEL_VERSION_RELATIVE_FILE}");
    println!("cargo:rerun-if-env-changed=NEKOR_KVM_CHANNEL");
    println!("cargo:rerun-if-env-changed=NEKOR_KVM_TAG");
    println!("cargo:rerun-if-env-changed=NEKOR_KVM_COMMIT");
    println!("cargo:rerun-if-env-changed=NEKOR_KVM_TARGET");

    #[cfg(feature = "internal-generate-binding")]
    binding::generate()?;

    #[cfg(any(feature = "stable", feature = "lts"))]
    native::compile(&target_triple)?;

    Ok(())
}

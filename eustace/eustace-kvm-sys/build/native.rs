//! Compile the C implementation of the KVM bridge.
//!
//! Native sources use the selected channel's exported UAPI headers. Cargo tracks
//! both the include tree and the canonical headers reached through its symlinks.

use std::{env, ffi::OsStr, io, path::PathBuf};

use walkdir::WalkDir;

/// KVM UAPI header required within the exported kernel include directory.
const KVM_HEADER_RELATIVE_FILE: &str = "linux/kvm.h";

/// Build the native archive using the same kernel channel as `preferred`.
///
/// The exported headers must already exist for the Cargo target.
///
/// Header preparation remains an explicit checkout step. Builds use the exported
/// headers without fetching kernel sources.
///
/// # Failure
///
/// Missing exports, unreadable source trees, and C compiler failures stop the build.
pub fn compile(target_triple: &str) -> Result<(), Box<dyn core::error::Error>> {
    let kernel_channel = if cfg!(feature = "stable") { "stable" } else { "lts" };

    let crate_directory = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);

    let include_directory = crate_directory.join(crate::C_INCLUDE_RELATIVE_DIRECTORY);
    let source_directory = crate_directory.join(crate::NATIVE_SOURCE_RELATIVE_DIRECTORY);

    let kernel_include_directory = crate_directory
        .join(crate::KERNEL_HEADER_RELATIVE_DIRECTORY)
        .join(target_triple)
        .join(kernel_channel)
        .join(crate::KERNEL_INCLUDE_RELATIVE_DIRECTORY);

    if !kernel_include_directory.join(KVM_HEADER_RELATIVE_FILE).is_file() {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "missing pinned KVM headers at {}. Run `just checkout {kernel_channel} {target_triple}` in the crate directory",
                kernel_include_directory.display()
            ),
        ))?;
    }

    println!("cargo:rerun-if-changed={}", kernel_include_directory.display());

    let mut build_context = cc::Build::new();

    build_context
        .std("gnu11")
        .include(&include_directory)
        .include(kernel_include_directory);

    for target_value in WalkDir::new(source_directory).sort_by_file_name() {
        let target_value = target_value?;
        let file_type = target_value.file_type();

        match target_value.path().extension().and_then(OsStr::to_str) {
            Some("c") if file_type.is_file() || file_type.is_symlink() => {
                build_context.file(target_value.path());
            },
            _ => {},
        }
    }

    // NOTE: Track canonical headers as well as the include tree so changes behind
    // directory symlinks invalidate the native archive.
    for target_value in WalkDir::new(include_directory).follow_links(true) {
        let target_value = target_value?;
        let target_path = target_value.path();

        match target_path.extension().and_then(OsStr::to_str) {
            Some("H" | "h") if target_value.file_type().is_file() => {
                println!("cargo:rerun-if-changed={}", target_path.canonicalize()?.display());
            },
            _ => {},
        }
    }

    build_context.try_compile(&env::var("CARGO_PKG_NAME")?)?;

    Ok(())
}

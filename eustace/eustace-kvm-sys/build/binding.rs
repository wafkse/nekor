//! Generate target-specific KVM declarations and verify the adapted C layouts.
//!
//! Linux declarations retain their provenance and layout reference. The native bridge
//! has a separate binding so its ABI remains independent of the adapted UAPI views.

use std::{
    env, fs,
    io::{self, Write},
    path::PathBuf,
    process::Command,
};

mod adapt;
mod bitfield;
mod error;

/// Unmodified Bindgen output retained for provenance.
const DEFAULT_BINDING_RELATIVE_FILE: &str = "binding-default.rs";

/// Original declarations without generated bitfield implementations, used for layout comparison.
const LAYOUT_BINDING_RELATIVE_FILE: &str = "binding-layout.rs";

/// Binding output with the local flexible array member representation.
const PROCESSED_BINDING_RELATIVE_FILE: &str = "binding-processed.rs";

/// Binding output for the Nekor-owned C bridge ABI.
const BRIDGE_BINDING_RELATIVE_FILE: &str = "binding-bridge.rs";

/// Generate separate Linux KVM and Nekor bridge bindings for the selected pin and target.
///
/// The build environment selects the kernel pin, target, and exported header directory.
///
/// # Failure
///
/// Returns an error for missing or invalid environment settings, parsing failures,
/// or failed filesystem operations.
pub fn generate() -> Result<(), Box<dyn core::error::Error>> {
    let kernel_channel = env::var("NEKOR_KVM_CHANNEL")?;

    match kernel_channel.as_str() {
        "lts" | "stable" => {},
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "NEKOR_KVM_CHANNEL must be lts or stable",
        ))?,
    }

    let kernel_tag = env::var("NEKOR_KVM_TAG")?;
    let kernel_commit = env::var("NEKOR_KVM_COMMIT")?;
    let crate_version = env::var("CARGO_PKG_VERSION")?;
    let target_triple = env::var("NEKOR_KVM_TARGET")?;

    let (target_pointer_width, target_endian) = target_configuration(&target_triple)?;

    let kernel_include_directory = PathBuf::from(crate::KERNEL_HEADER_RELATIVE_DIRECTORY)
        .join(&target_triple)
        .join(&kernel_channel)
        .join(crate::KERNEL_INCLUDE_RELATIVE_DIRECTORY);
    let kvm_header_path = kernel_include_directory.join("linux/kvm.h");
    let bridge_header_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?)
        .join(crate::C_INCLUDE_RELATIVE_DIRECTORY)
        .join("nekor-kvm.h");
    let target_output_directory = PathBuf::from(env::var("OUT_DIR")?).join(&target_triple);
    let include_directory = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?).join(crate::C_INCLUDE_RELATIVE_DIRECTORY);

    fs::create_dir_all(&target_output_directory)?;

    println!("cargo:rerun-if-changed={}", kernel_include_directory.display());

    let kvm_binding = bindgen_builder(
        &target_triple,
        &include_directory,
        &kernel_include_directory,
        &target_output_directory,
    )
    .allowlist_type("kvm_.*")
    .allowlist_var("KVM_.*")
    .header(kvm_header_path.to_string_lossy())
    .generate()?;
    let default_binding_ast = syn::parse_file(&kvm_binding.to_string())?;
    let bridge_binding = bindgen_builder(
        &target_triple,
        &include_directory,
        &kernel_include_directory,
        &target_output_directory,
    )
    .header(bridge_header_path.to_string_lossy())
    .prepend_enum_name(false)
    .allowlist_type("nekor_kvm_.*")
    .allowlist_var("NEKOR_KVM_.*")
    .allowlist_function("nekor_kvm_.*")
    .generate()?;
    let bridge_binding_ast = syn::parse_file(&bridge_binding.to_string())?;
    let bridge_binding_source = prettyplease::unparse(&bridge_binding_ast);
    let bitfield_adaptation =
        bitfield::Adapter::new(&default_binding_ast, target_pointer_width, target_endian).apply()?;
    let adapted_binding = adapt::Adapter::new(&default_binding_ast).apply(
        bitfield_adaptation.binding_ast,
        &bitfield_adaptation.removed_record_names,
    )?;
    let processed_binding_source = prettyplease::unparse(&adapted_binding.processed_binding_ast);
    let layout_binding_source = prettyplease::unparse(&adapted_binding.layout_binding_ast);
    let default_binding_source = prettyplease::unparse(&default_binding_ast);
    let clang_version = bindgen::clang_version().full;
    let generation_metadata = format_args!(
        include_str!("binding/binding.head.template"),
        kernel_tag = kernel_tag,
        kernel_commit = kernel_commit,
        target_triple = target_triple,
        crate_version = crate_version,
        clang_version = clang_version,
    );
    let channel_output_directory = target_output_directory.join(&kernel_channel);
    let processed_binding_path = channel_output_directory.join(PROCESSED_BINDING_RELATIVE_FILE);
    let layout_binding_path = channel_output_directory.join(LAYOUT_BINDING_RELATIVE_FILE);
    let default_binding_path = channel_output_directory.join(DEFAULT_BINDING_RELATIVE_FILE);
    let bridge_binding_path = channel_output_directory.join(BRIDGE_BINDING_RELATIVE_FILE);

    fs::create_dir_all(&channel_output_directory)?;

    let mut processed_binding_file = fs::File::create(processed_binding_path)?;

    write!(
        processed_binding_file,
        "{generation_metadata}{processed_binding_source}"
    )?;

    let mut layout_binding_file = fs::File::create(layout_binding_path)?;

    write!(layout_binding_file, "{generation_metadata}{layout_binding_source}")?;

    let mut default_binding_file = fs::File::create(default_binding_path)?;

    write!(default_binding_file, "{generation_metadata}{default_binding_source}")?;

    let mut bridge_binding_file = fs::File::create(bridge_binding_path)?;

    write!(bridge_binding_file, "{generation_metadata}{bridge_binding_source}")?;

    Ok(())
}

/// Configure Bindgen for a selected target and the two header roots used by this crate.
fn bindgen_builder(
    target_triple: &str,
    include_directory: &std::path::Path,
    kernel_include_directory: &std::path::Path,
    target_output_directory: &std::path::Path,
) -> bindgen::Builder {
    bindgen::Builder::default()
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .clang_macro_fallback()
        .clang_macro_fallback_build_dir(target_output_directory)
        .clang_arg(format!("--target={target_triple}"))
        .clang_arg("-nostdinc")
        .clang_arg("-D__BINDGEN__")
        .clang_arg(format!("-I{}", include_directory.display()))
        .clang_arg(format!("-I{}", kernel_include_directory.display()))
        .use_core()
        .rust_edition(bindgen::RustEdition::Edition2024)
        .wrap_unsafe_ops(true)
        .generate_comments(true)
        .layout_tests(true)
        .derive_debug(false)
        .formatter(bindgen::Formatter::None)
}

/// Read target ABI facts from rustc for the selected binding target.
fn target_configuration(target_triple: &str) -> Result<(u32, bitfield::Endian), Box<dyn core::error::Error>> {
    let rustc_path = env::var("RUSTC")?;
    let target_configuration = Command::new(rustc_path)
        .args(["--print", "cfg", "--target", target_triple])
        .output()?;

    match target_configuration.status.success() {
        true => {},
        false => {
            let error_message = String::from_utf8_lossy(&target_configuration.stderr).into_owned();

            Err(io::Error::new(io::ErrorKind::InvalidInput, error_message))?;
        },
    }

    let target_configuration = String::from_utf8(target_configuration.stdout)?;
    let target_pointer_width = target_configuration
        .lines()
        .find_map(|line| line.strip_prefix("target_pointer_width=\""))
        .and_then(|value| value.strip_suffix('"'))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "rustc did not report the selected target pointer width",
            )
        })?
        .parse::<u32>()?;
    let target_endian = target_configuration
        .lines()
        .find_map(|line| line.strip_prefix("target_endian=\""))
        .and_then(|value| value.strip_suffix('"'))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "rustc did not report the selected target byte order",
            )
        })?;
    let target_endian = match target_endian {
        "little" => bitfield::Endian::Little,
        "big" => bitfield::Endian::Big,
        endian => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("unsupported target byte order `{endian}`"),
        ))?,
    };

    Ok((target_pointer_width, target_endian))
}

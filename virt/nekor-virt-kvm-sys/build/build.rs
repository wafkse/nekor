/// Binding generation enabled after the selected Linux userspace headers have been exported.
#[cfg(feature = "internal-generate-binding")]
mod binding;

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let target_triple = std::env::var("TARGET")?;

    println!("cargo:rustc-env=NEKOR_KVM_TARGET={target_triple}");
    println!("cargo:rerun-if-changed=build");
    println!("cargo:rerun-if-changed=checkout/version.config");
    println!("cargo:rerun-if-env-changed=NEKOR_KVM_CHANNEL");
    println!("cargo:rerun-if-env-changed=NEKOR_KVM_TAG");
    println!("cargo:rerun-if-env-changed=NEKOR_KVM_COMMIT");
    println!("cargo:rerun-if-env-changed=NEKOR_KVM_TARGET");

    #[cfg(feature = "internal-generate-binding")]
    binding::generate()?;

    Ok(())
}

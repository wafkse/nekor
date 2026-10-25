/// Optional generation from explicitly prepared Linux userspace headers.
#[cfg(feature = "internal-generate-binding")]
mod binding {
    include!("binding.rs");
}

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let target = std::env::var("TARGET")?;

    println!("cargo:rustc-env=NEKOR_KVM_TARGET={target}");
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

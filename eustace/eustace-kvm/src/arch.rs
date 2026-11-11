//! KVM operations and values grouped by guest architecture.

#[cfg(all(
    any(feature = "stable", feature = "lts"),
    any(target_arch = "x86", target_arch = "x86_64"),
))]
pub mod x86;

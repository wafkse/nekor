#![allow(
    explicit_restricted_visibility,
    no_numeric_field_access,
    no_self_field_access,
    private_single_call_fn,
    self_pattern_rest,
    statement_group_spacing,
    struct_invariant_note,
    reason = "The generated interfaces preserve upstream declarations and generator syntax."
)]
#![expect(nonstandard_style, clippy::all, clippy::pedantic, clippy::restriction)]

//! Generated declarations for the pinned Linux KVM ABI and Nekor C bridge.
//!
//! Public users access these declarations through [`crate::ffi::binding`]. Lint exemptions apply
//! only to this generated subtree.

/// Raw KVM UAPI from the pinned Linux LTS release.
#[cfg(feature = "lts")]
pub mod lts;

/// Raw KVM UAPI from the pinned Linux stable release.
#[cfg(feature = "stable")]
pub mod stable;

pub mod bridge {
    //! Generated declarations for the Nekor C bridge ABI.

    cfg_select! {
        all(feature = "stable", feature = "lts") => {
            include!(concat!(
                "generated/stable/",
                env!("NEKOR_KVM_TARGET"),
                "/binding-bridge.rs"
            ));
        },
        feature = "stable" => {
            include!(concat!(
                "generated/stable/",
                env!("NEKOR_KVM_TARGET"),
                "/binding-bridge.rs"
            ));
        },
        feature = "lts" => {
            include!(concat!(
                "generated/lts/",
                env!("NEKOR_KVM_TARGET"),
                "/binding-bridge.rs"
            ));
        },
    }
}

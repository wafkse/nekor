#![doc(hidden)]
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

//! Pinned Linux KVM declarations selected by the crate's Cargo features.
//!
//! The interface modules are re-exported at the crate root. Lint exemptions
//! apply only to this generated subtree.

/// Raw KVM UAPI from the pinned Linux LTS release.
#[cfg(feature = "lts")]
pub mod lts;

/// Raw KVM UAPI from the pinned Linux stable release.
#[cfg(feature = "stable")]
pub mod stable;

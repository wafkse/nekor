//! Procedural derive support for `nekor-project`.
//!
//! Application crates should depend on the `nekor-project` facade rather than
//! importing this crate directly. The facade re-exports the [`Project`] derive
//! and provides the runtime traits referenced by generated code.
//!
//! # Accepted input
//!
//! The derive accepts structs and enums with named, unnamed, empty, or unit
//! fields. It preserves source visibility, generic parameters, generic
//! defaults, bounds, and where clauses in generated projection types.
//!
//! Unions are rejected because the macro cannot safely determine which field is
//! active. Packed representations are rejected because projected references
//! require aligned fields.
//!
//! # Generated output
//!
//! A source type named `Value` receives immutable and mutable projection types
//! named `ValueProjection` and `ValueProjectionMut`. The derive also emits an
//! implementation of the facade `Project` trait.
//!
//! Fields marked with `#[project(pin)]` use `core::pin::Pin` in both projection
//! types. Other fields use ordinary references. Enum projection types preserve
//! variant names and field shapes without copying explicit source
//! discriminants.
//!
//! The derive emits structural `Unpin` logic and coherence blockers for `Drop`,
//! `Deref`, and `DerefMut` when pinned fields exist. A matching type-level
//! `#[project(unsafe = Trait)]` clause removes only the named enforcement rule.
//!
//! # Helper attributes
//!
//! `#[project(pin)]` is a field option. It marks a field as structurally
//! pinned.
//!
//! `#[project(unsafe = Unpin)]` is a type option. It omits the generated
//! structural `Unpin` implementation.
//!
//! `#[project(unsafe = Drop)]` is a type option. It omits the generated `Drop`
//! coherence blocker.
//!
//! `#[project(unsafe = Deref)]` is a type option. It omits the generated
//! `Deref` coherence blocker.
//!
//! `#[project(unsafe = DerefMut)]` is a type option. It omits the generated
//! `DerefMut` coherence blocker.
//!
//! Every unsafe clause transfers responsibility for the matching invariant to
//! the annotated type. See the facade crate documentation for complete safety
//! contracts and examples.

use nekor_project_core::ProjectItem;
use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

/// Derive immutable and mutable pin projections.
///
/// The macro parses the input into a validated `ProjectItem`, then expands the
/// projection types, trait implementation, structural `Unpin` logic, and trait
/// blockers.
///
/// # Errors
///
/// Expansion produces a compile error for unsupported input forms, malformed
/// helper attributes, duplicate unsafe clauses, misplaced options, unknown
/// unsafe targets, and packed representations.
#[proc_macro_derive(Project, attributes(project))]
pub fn project_derive(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);

    match ProjectItem::input(derive_input).and_then(ProjectItem::expand) {
        Ok(target_tokens) => TokenStream::from(target_tokens),
        Err(target_error) => TokenStream::from(target_error.into_compile_error()),
    }
}

//! Parsing for `project` helper attributes.
//!
//! The parser recognizes one option per helper attribute.
//!
//! ```text
//! #[project(pin)]
//! #[project(unsafe = Unpin)]
//! #[project(unsafe = Drop)]
//! #[project(unsafe = Deref)]
//! #[project(unsafe = DerefMut)]
//! ```
//!
//! `pin` belongs on a field. Every `unsafe = Trait` clause belongs on the
//! source struct or enum. The item and field models enforce placement after
//! this module parses the syntax.
//!
//! Parsing is strict. Unknown names, unknown unsafe targets, trailing tokens,
//! and malformed assignments return `syn::Error`. Callers never receive a
//! partially parsed option.

use alloc::string::ToString;
use alloc::vec::Vec;

use syn::{Attribute, Ident, Token, parse::Nothing, parse::Parse};

/// One parsed `project` helper attribute.
///
/// This type represents syntax only. Placement and duplicate validation happen
/// when a complete source item or field is constructed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectAttribute {
    /// A type-level opt-out from one generated safety enforcement rule.
    ///
    /// The contained [`UnsafeClauseTarget`] identifies the exact rule omitted
    /// by expansion. Other enforcement remains active.
    UnsafeClause(UnsafeClauseTarget),

    /// Mark a field as structurally pinned.
    ///
    /// Immutable expansion uses `Pin<&T>`. Mutable expansion uses
    /// `Pin<&mut T>`. This option does not affect any other field.
    Pin,
}

impl ProjectAttribute {
    /// Parse every `project` helper attribute in an attribute list.
    ///
    /// Non-project attributes are ignored and remain the responsibility of the
    /// source item. Project attributes preserve source order in the returned
    /// list.
    ///
    /// # Errors
    ///
    /// Returns an error when a project helper attribute is malformed, contains
    /// trailing tokens, or names an unsupported unsafe target.
    pub fn list(attribute_list: &[Attribute]) -> syn::Result<Vec<Self>> {
        attribute_list
            .iter()
            .filter(|attribute| attribute.path().is_ident("project"))
            .map(Attribute::parse_args)
            .collect()
    }
}

impl Parse for ProjectAttribute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(Token![unsafe]) {
            let _: Token![unsafe] = input.parse()?;
            let _: Token![=] = input.parse()?;
            let trait_name: Ident = input.parse()?;
            let _: Nothing = input.parse()?;

            let clause_target = match trait_name.to_string().as_str() {
                "Unpin" => UnsafeClauseTarget::Unpin,
                "Deref" => UnsafeClauseTarget::Deref,
                "DerefMut" => UnsafeClauseTarget::DerefMut,
                "Drop" => UnsafeClauseTarget::Drop,
                _ => {
                    return Err(syn::Error::new_spanned(
                        trait_name,
                        "invalid trait name for unsafe clause",
                    ));
                }
            };

            return Ok(Self::UnsafeClause(clause_target));
        }

        let annotate_ident: Ident = input.parse()?;

        if annotate_ident == "pin" {
            let _: Nothing = input.parse()?;

            Ok(Self::Pin)
        } else {
            Err(syn::Error::new_spanned(
                annotate_ident,
                "invalid project attribute annotation",
            ))
        }
    }
}

/// The enforcement rule disabled by a type-level unsafe clause.
///
/// Every variant represents a separate proof obligation. Selecting one variant
/// does not imply or enable any other variant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnsafeClauseTarget {
    /// Omit the generated structural [`Unpin`] implementation.
    ///
    /// Normal auto-trait behavior and user-written implementations determine
    /// whether the source type implements [`Unpin`]. The type author must prove
    /// that every move permitted by that result preserves all pinned fields.
    ///
    /// This option does not change field projection types and does not remove
    /// the `Drop`, `Deref`, or `DerefMut` blockers.
    Unpin,

    /// Omit the generated [`Drop`] coherence blocker.
    ///
    /// The type author must prove that destructor code never moves a pinned
    /// field and never replaces an active enum variant. Destructor logic that
    /// needs field access should forward through `nekor_project::PinnedDrop`.
    ///
    /// This option does not generate a destructor. It only permits the source
    /// type to provide one.
    Drop,

    /// Omit the generated [`core::ops::Deref`] coherence blocker.
    ///
    /// The type author must prove that shared dereferencing cannot expose a
    /// path that moves pinned data. Interior mutability in the dereference
    /// target is part of this obligation.
    ///
    /// This option does not permit [`core::ops::DerefMut`]. That trait needs
    /// its own clause.
    Deref,

    /// Omit the generated [`core::ops::DerefMut`] coherence blocker.
    ///
    /// The type author must prove that mutable dereferencing cannot expose an
    /// ordinary mutable reference from which a pinned field can be replaced,
    /// swapped, or moved.
    ///
    /// This option does not permit [`core::ops::Deref`]. A type implementing
    /// both traits must select both clauses.
    DerefMut,
}

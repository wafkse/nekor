//! Field shape and structural pinning metadata.
//!
//! This module converts `syn::Fields` into a representation tailored to
//! projection expansion. It preserves named, unnamed, empty, and unit shapes.
//! Each field records its source visibility, source type, stable member name,
//! and whether `#[project(pin)]` selected structural pinning.
//!
//! Type-level unsafe clauses are rejected here when they appear on fields. A
//! field may contain at most one `project(pin)` attribute. Invalid syntax is
//! returned as `syn::Error` before item expansion begins.

use alloc::format;
use alloc::vec::Vec;

use syn::{Field, Fields, FieldsNamed, FieldsUnnamed, Ident, Index, Type, Visibility};

use crate::attribute::{ProjectAttribute, UnsafeClauseTarget};

/// A validated source field shape.
///
/// Expansion uses the variant to preserve source syntax. Named fields become a
/// projection structure or enum variant with named fields. Unnamed fields keep
/// positional order. Unit inputs remain unit projections.
///
/// Empty named and empty unnamed field sets remain distinct from unit inputs.
// NOTE(invariant): Every field in `Named` has an identifier. Every field in
// `Unnamed` has its original positional index. `ProjectFields::new` establishes
// this relationship and no mutation API can change it.
pub enum ProjectFields {
    /// Named fields.
    Named(ProjectFieldsNamed),

    /// Unnamed fields.
    Unnamed(ProjectFieldsUnnamed),

    /// A unit structure or variant.
    Unit,
}

impl ProjectFields {
    /// Transform source fields into validated project fields.
    ///
    /// Field order and field shape are preserved exactly. Named fields receive
    /// identifier members. Unnamed fields receive zero-based positional
    /// members.
    ///
    /// # Errors
    ///
    /// Returns an error when a field contains a malformed helper attribute or
    /// an attribute that is invalid on a field.
    pub fn new(fields: Fields) -> syn::Result<Self> {
        match fields {
            Fields::Named(FieldsNamed { named, .. }) => named
                .into_iter()
                .enumerate()
                .map(ProjectField::new)
                .collect::<syn::Result<Vec<_>>>()
                .map(ProjectFieldsNamed::new)
                .map(Self::Named),
            Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => unnamed
                .into_iter()
                .enumerate()
                .map(ProjectField::new)
                .collect::<syn::Result<Vec<_>>>()
                .map(ProjectFieldsUnnamed::new)
                .map(Self::Unnamed),
            Fields::Unit => Ok(Self::Unit),
        }
    }

    /// Return the fields in source order.
    #[must_use]
    pub fn field_list(&self) -> &[ProjectField] {
        match self {
            Self::Named(fields) => fields.field_list(),
            Self::Unnamed(fields) => fields.field_list(),
            Self::Unit => &[],
        }
    }

    /// Return whether this is a named field set.
    #[must_use]
    pub const fn is_named(&self) -> bool {
        matches!(self, Self::Named(..))
    }

    /// Return whether this is an unnamed field set.
    #[must_use]
    pub const fn is_unnamed(&self) -> bool {
        matches!(self, Self::Unnamed(..))
    }
}

/// A source-order collection of named project fields.
///
/// Every contained field has an [`IdentOrIndex::Ident`] name.
pub struct ProjectFieldsNamed {
    /// Fields in source order.
    field_list: Vec<ProjectField>,
}

impl ProjectFieldsNamed {
    /// Construct named project fields.
    #[must_use]
    pub const fn new(field_list: Vec<ProjectField>) -> Self {
        Self { field_list }
    }

    /// Return the fields in source order.
    #[must_use]
    pub fn field_list(&self) -> &[ProjectField] {
        let Self { field_list } = self;

        field_list
    }
}

/// A source-order collection of unnamed project fields.
///
/// Every contained field has an [`IdentOrIndex::Index`] name that matches its
/// position in this collection.
pub struct ProjectFieldsUnnamed {
    /// Fields in source order.
    field_list: Vec<ProjectField>,
}

impl ProjectFieldsUnnamed {
    /// Construct unnamed project fields.
    #[must_use]
    pub const fn new(field_list: Vec<ProjectField>) -> Self {
        Self { field_list }
    }

    /// Return the fields in source order.
    #[must_use]
    pub fn field_list(&self) -> &[ProjectField] {
        let Self { field_list } = self;

        field_list
    }
}

/// One validated field in a projected source item.
///
/// The private representation prevents later stages from changing the source
/// type, visibility, member identity, or structural pinning decision.
pub struct ProjectField {
    /// Source field visibility.
    visibility: Visibility,

    /// Source field name or positional index.
    name: IdentOrIndex,

    /// Source field type.
    ty: Type,

    /// Whether this field is structurally pinned.
    pinned: bool,
}

impl ProjectField {
    /// Transform one source field into a validated project field.
    ///
    /// Named fields retain their source identifier. Unnamed fields use the
    /// supplied source index. `project(pin)` becomes the stored pinning policy.
    ///
    /// # Errors
    ///
    /// Returns an error when the field has malformed, duplicate, or misplaced
    /// helper attributes.
    pub fn new((field_index, field): (usize, Field)) -> syn::Result<Self> {
        let Field {
            attrs: attribute_list,
            vis: field_visibility,
            ident: field_name,
            ty: field_type,
            ..
        } = field;

        let project_attribute_list = ProjectAttribute::list(&attribute_list)?;
        let mut field_pinned = false;

        for project_attribute in project_attribute_list {
            match project_attribute {
                ProjectAttribute::Pin if field_pinned => {
                    return Err(syn::Error::new_spanned(
                        field_type,
                        "duplicate project pin attribute",
                    ));
                }
                ProjectAttribute::Pin => field_pinned = true,
                ProjectAttribute::UnsafeClause(target) => {
                    let target_name = match target {
                        UnsafeClauseTarget::Unpin => "Unpin",
                        UnsafeClauseTarget::Drop => "Drop",
                        UnsafeClauseTarget::Deref => "Deref",
                        UnsafeClauseTarget::DerefMut => "DerefMut",
                    };

                    return Err(syn::Error::new_spanned(
                        field_type,
                        format!("unsafe {target_name} clauses are only valid on a type"),
                    ));
                }
            }
        }

        let field_name = field_name.map_or_else(
            || IdentOrIndex::Index(Index::from(field_index)),
            IdentOrIndex::Ident,
        );

        Ok(Self {
            visibility: field_visibility,
            name: field_name,
            ty: field_type,
            pinned: field_pinned,
        })
    }

    /// Return the source field visibility.
    #[must_use]
    pub const fn visibility(&self) -> &Visibility {
        let Self { visibility, .. } = self;

        visibility
    }

    /// Return the source field name or positional index.
    #[must_use]
    pub const fn name(&self) -> &IdentOrIndex {
        let Self { name, .. } = self;

        name
    }

    /// Return the source field type.
    #[must_use]
    pub const fn ty(&self) -> &Type {
        let Self { ty, .. } = self;

        ty
    }

    /// Return whether this field is structurally pinned.
    ///
    /// A true result means immutable expansion uses `Pin<&T>`, mutable
    /// expansion uses `Pin<&mut T>`, and generated `Unpin` depends on this
    /// field type.
    #[must_use]
    pub const fn pinned(&self) -> bool {
        let &Self { pinned, .. } = self;

        pinned
    }
}

/// Stable source identity for a named or positional field.
///
/// The distinction lets expansion use identifiers for named patterns and
/// numeric source order for tuple patterns without converting either concept to
/// an untyped string.
pub enum IdentOrIndex {
    /// A named field identifier.
    Ident(Ident),

    /// An unnamed field index.
    Index(Index),
}

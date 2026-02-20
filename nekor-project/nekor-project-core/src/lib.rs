#![cfg_attr(not(any(test, miri)), no_std)]
#![forbid(
    clippy::all,
    clippy::perf,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::panic,
    clippy::pedantic,
    rustdoc::all
)]
//! Core macro crate for `nekor-project`.

extern crate alloc;

use alloc::string::ToString;
use alloc::vec::Vec;

use proc_macro2::TokenStream;

use syn::{
    Attribute, Data, DataEnum, DataStruct, DeriveInput, Error, Expr, Field, FieldMutability,
    Fields, FieldsNamed, FieldsUnnamed, Generics, Ident, Token, Type, Variant, Visibility,
    parse::{Nothing, Parse},
    token::{self, Brace},
};

/// A language item that has been pin-projected.
pub enum ProjectItem {
    /// A projected structure with potentially structurally-pinned fields.
    Struct(ProjectStruct),

    /// A projected enumeration with potentially structurally-pinned fields.
    Enum(ProjectEnum),
}

impl ProjectItem {
    /// Expand the target pin-project item.
    #[inline]
    pub fn expand(self) -> syn::Result<TokenStream> {
        match self {
            ProjectItem::Struct(project_struct) => project_struct.expand(),
            ProjectItem::Enum(project_enum) => project_enum.expand(),
        }
    }
}

impl ProjectItem {
    pub fn input(derive_input: DeriveInput) -> syn::Result<Self> {
        let DeriveInput { ident, data, .. } = derive_input;

        match data {
            Data::Struct(struct_data) => {
                let DeriveInput {
                    attrs: attribute_list,
                    vis: struct_visibility,
                    generics: struct_generics,
                    ..
                } = derive_input;

                let struct_ident = ident;

                let attribute_list = attribute_list
                    .into_iter()
                    .map(ProjectAttribute::new)
                    .flatten()
                    .collect::<Vec<ProjectAttribute>>();

                let DataStruct {
                    struct_token,
                    fields,
                    semi_token,
                } = struct_data;

                let field_list = ProjectFields::new(fields);

                let struct_data = ProjectStructData {
                    struct_token,
                    field_list,
                    semi_token,
                };

                Ok(Self::Struct(ProjectStruct {
                    attribute_list,
                    struct_visibility,
                    struct_ident,
                    struct_generics,
                    struct_data,
                }))
            }
            Data::Enum(enum_data) => {
                let DeriveInput {
                    attrs: attribute_list,
                    vis: enum_visibility,
                    generics: enum_generics,
                    ..
                } = derive_input;

                let enum_ident = ident;

                let attribute_list = attribute_list
                    .into_iter()
                    .map(ProjectAttribute::new)
                    .flatten()
                    .collect::<Vec<ProjectAttribute>>();

                let DataEnum {
                    enum_token,
                    brace_token,
                    variants,
                } = enum_data;

                let variant_list = variants.into_iter().map(ProjectEnumVariant::new).collect();

                let enum_data = ProjectEnumData {
                    enum_token,
                    brace_token,
                    variant_list,
                };

                Ok(Self::Enum(ProjectEnum {
                    attribute_list,
                    enum_visibility,
                    enum_ident,
                    enum_generics,
                    enum_data,
                }))
            }
            Data::Union(..) => Err(Error::new_spanned(ident, "a union cannot be pin-projected")),
        }
    }
}

/// A `project` helper attribute.
///
/// This enumeration contains all the possible project attributes.
pub enum ProjectAttribute {
    /// A `project(unsafe = Trait)` project attribute.
    UnsafeClause(UnsafeClauseTarget),

    /// A `project(pin)` project attribute.
    Pin,
}

impl ProjectAttribute {
    /// Attempt to parse a project attribute from an attribute.
    pub fn new(attr: Attribute) -> Option<Self> {
        if attr.meta.path().is_ident("project") {
            attr.parse_args().ok()
        } else {
            None
        }
    }
}

impl Parse for ProjectAttribute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let annotate_ident: Ident = input.parse()?;

        match annotate_ident.to_string().as_str() {
            "unsafe" => {
                let _: Token![=] = input.parse()?;

                let trait_name: Ident = input.parse()?;

                let _: Nothing = input.parse()?;

                let clause_target = match trait_name.to_string().as_str() {
                    "Unpin" => Ok(UnsafeClauseTarget::Unpin),
                    "Deref" => Ok(UnsafeClauseTarget::Deref),
                    "DerefMut" => Ok(UnsafeClauseTarget::DerefMut),
                    "Drop" => Ok(UnsafeClauseTarget::Drop),
                    _ => Err(syn::Error::new_spanned(
                        trait_name,
                        "invalid trait name for unsafe clause",
                    )),
                }?;

                Ok(Self::UnsafeClause(clause_target))
            }
            "pin" => {
                let _: Nothing = input.parse()?;

                Ok(Self::Pin)
            }
            _ => Err(syn::Error::new_spanned(
                annotate_ident,
                "invalid project attribute annotation",
            )),
        }
    }
}

/// The target of an unsafe clause.
///
/// Determines whether to implement *blocker* impossibly-bounded
/// implementations of the trait.
pub enum UnsafeClauseTarget {
    /// The [`Unpin`] trait.
    ///
    /// # Soundness
    ///
    /// By default, an implementation block for the `Unpin` trait is performed
    /// on the annotated item.
    ///
    /// This default implementation only explicitly implements the `Unpin` trait
    /// if all structurally-pinned fields are also implementors of the trait.
    ///
    /// A `project(unsafe = Unpin)` clause permits *not including* such
    /// implementation block for a particular type, letting the `Unpin`
    /// auto-trait semantics take over for the annotated type.
    ///
    /// This is *unsafe* since a structurally-pinned field can be `Unpin`, which
    /// makes pinning the containing structure futile.
    Unpin,

    /// The [`Drop`] trait.
    ///
    /// # Soundness
    ///
    /// A badly-implemented `Drop` implementation can move out from the bare
    /// mutable reference to the pinned structure, effectively being able to
    /// break pinning guarantees due to unfettered access to all
    /// structurally-pinned fields via a mutable borrow.
    Drop,

    /// The [`Deref`] trait.
    ///
    /// # Soundness
    ///
    /// Similarly to soundness implications of the [`Self::DerefMut`] variant,
    /// this can break pinning guarantees if the structurally-pinned field is
    /// embedded within a construct that can permit *interior mutability*
    /// (a.k.a., be non-`Freeze`).
    ///
    /// This potentially allows that a badly-implemented `Deref` implementation
    /// is able to retrieve a mutable reference to structurally-pinned data and
    /// effectively break any pinning guarantees.
    Deref,

    /// The [`DerefMut`] trait.
    ///
    /// # Soundness
    ///
    /// A badly-implemented `DerefMut` implementation can potentially
    /// dereference to a structurally-pinned field, thus exposing a bare mutable
    /// reference (`&mut StructuallyPinnedTy`), which can be moved out from,
    /// breaking pinning guarantees.
    DerefMut,
}

/// A pin-projected structure.
pub struct ProjectStruct {
    /// The project-specific attributes of the structure.
    attribute_list: Vec<ProjectAttribute>,

    /// The visibility of the type.
    struct_visibility: Visibility,

    /// The identifier of the structure.
    struct_ident: Ident,

    /// The generic type parameter list, lifetime, and generic bounds of the
    /// structure.
    struct_generics: Generics,

    /// The data intrinsic to the structure.
    struct_data: ProjectStructData,
}

impl ProjectStruct {
    /// Expand this pin-project structure.
    pub fn expand(self) -> syn::Result<TokenStream> {
        let Self {
            attribute_list,
            struct_visibility,
            struct_ident,
            struct_generics,
            struct_data,
        } = self;

        // TODO: Finish expansion of this.

        Ok(TokenStream::new())
    }
}

/// A pin-projected enumeration.
pub struct ProjectEnum {
    /// The project-specific attributes of the enumeration.
    attribute_list: Vec<ProjectAttribute>,

    /// The visibility of the type.
    enum_visibility: Visibility,

    /// The identifier of the structure.
    enum_ident: Ident,

    /// The generic type parameter list, lifetime, and generic bounds of the
    /// structure.
    enum_generics: Generics,

    /// The generic type parameter list, lifetime, and generic bounds of the
    /// structure.
    enum_data: ProjectEnumData,
}

impl ProjectEnum {
    /// Expand this pin-project enumeration.
    pub fn expand(self) -> syn::Result<TokenStream> {
        Ok(TokenStream::new())
    }
}

/// The data respective to a pin-projected enumeration.
pub struct ProjectEnumData {
    /// The `enum` token.
    enum_token: Token![enum],

    /// The `{}` token pair.
    brace_token: Brace,

    /// The project-specific variants of the enumeration.
    variant_list: Vec<ProjectEnumVariant>,
}

pub struct ProjectEnumVariant {
    /// The project-specific attribtues of the enumeration variant.
    attribute_list: Vec<ProjectAttribute>,

    /// The identifier of the enumeration variant.
    variant_ident: Ident,

    /// The field list of the enumeration variant.
    field_list: ProjectFields,

    /// The explicit discriminant of the enumeration variant.
    variant_discriminant: Option<(Token![=], Expr)>,
}

impl ProjectEnumVariant {
    pub fn new(target_variant: Variant) -> Self {
        let Variant {
            attrs: attribute_list,
            ident: variant_ident,
            fields: field_list,
            discriminant: variant_discriminant,
        } = target_variant;

        let attribute_list = attribute_list
            .into_iter()
            .map(ProjectAttribute::new)
            .flatten()
            .collect::<Vec<ProjectAttribute>>();

        let field_list = ProjectFields::new(field_list);

        Self {
            attribute_list,
            variant_ident,
            field_list,
            variant_discriminant,
        }
    }
}

/// Struct data relevant to pin-projection.
pub struct ProjectStructData {
    /// The `struct` token.
    struct_token: Token![struct],

    /// The fields of the struct.
    field_list: ProjectFields,

    /// The semicolon token of the struct.
    semi_token: Option<Token![;]>,
}

/// A set of fields.
pub enum ProjectFields {
    /// Named fields.
    Named(ProjectFieldsNamed),

    /// Unnamed fields.
    Unnamed(ProjectFieldsUnnamed),

    /// Unit struct or variant.
    Unit,
}

impl ProjectFields {
    /// Transform a set of fields to a set of project-fields.
    pub fn new(fields: Fields) -> Self {
        match fields {
            Fields::Named(FieldsNamed { brace_token, named }) => {
                let field_list = named
                    .into_iter()
                    .enumerate()
                    .map(ProjectField::new)
                    .collect();

                ProjectFields::Named(ProjectFieldsNamed {
                    brace_token,
                    field_list,
                })
            }
            Fields::Unnamed(FieldsUnnamed {
                paren_token,
                unnamed,
            }) => {
                let field_list = unnamed
                    .into_iter()
                    .enumerate()
                    .map(ProjectField::new)
                    .collect();

                ProjectFields::Unnamed(ProjectFieldsUnnamed {
                    paren_token,
                    field_list,
                })
            }
            Fields::Unit => ProjectFields::Unit,
        }
    }
}

/// A set of named fields.
pub struct ProjectFieldsNamed {
    /// The brace token.
    brace_token: token::Brace,

    /// The field list.
    field_list: Vec<ProjectField>,
}

/// A set of unnamed fields.
pub struct ProjectFieldsUnnamed {
    /// The parenthesis token.
    paren_token: token::Paren,

    /// The field list.
    field_list: Vec<ProjectField>,
}

/// A singular field of a pin-projected structure or enumeration.
pub struct ProjectField {
    /// The project-specific attribtues of the enumeration variant.
    attribute_list: Vec<ProjectAttribute>,

    /// The visbility of the field.
    field_visibility: Visibility,

    /// The mutability of the field.
    field_mutability: FieldMutability,

    /// The identifier or numeric indice of the field.
    field_name: IdentOrIndex,

    /// The `:` token.
    colon_token: Option<Token![:]>,

    /// The type of the field.
    field_type: Type,
}

impl ProjectField {
    /// Transform a field to a project-field.
    pub fn new((field_indice, field_struct): (usize, Field)) -> Self {
        let Field {
            attrs: attribute_list,
            vis: field_visibility,
            mutability: field_mutability,
            ident: field_name,
            colon_token,
            ty: field_type,
        } = field_struct;

        let attribute_list = attribute_list
            .into_iter()
            .map(ProjectAttribute::new)
            .flatten()
            .collect();

        let field_name = if let Some(field_ident) = field_name {
            IdentOrIndex::Ident(field_ident)
        } else {
            IdentOrIndex::Index(field_indice)
        };

        ProjectField {
            attribute_list,
            field_visibility,
            field_mutability,
            field_name,
            colon_token,
            field_type,
        }
    }
}

/// An union between an identifier or a numeric indice.
pub enum IdentOrIndex {
    /// The identifier of the entity.
    Ident(Ident),

    /// The local index of the entity.
    Index(usize),
}

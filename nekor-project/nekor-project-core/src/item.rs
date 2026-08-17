//! Complete source item validation and projection expansion.
//!
//! [`ProjectItem`] owns the transition from a `syn::DeriveInput` to generated
//! tokens. Parsing validates representation attributes and helper placement,
//! then constructs a structure-specific or enum-specific model. Expansion
//! consumes that model so validated configuration cannot be reused or mutated
//! after code generation.
//!
//! A source type named `Value` generates `ValueProjection` and
//! `ValueProjectionMut`. Projection types preserve source visibility, field
//! shape, generic parameters, generic defaults, bounds, and where clauses. Enum
//! projections preserve variant names and omit explicit source discriminants.
//!
//! Generated target code implements `nekor_project::Project`, uses only `core`,
//! and performs no allocation. This implementation crate may use `alloc` while
//! constructing host-side syntax and token streams.

use alloc::format;
use alloc::vec::Vec;

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{
    Attribute, Data, DataEnum, DataStruct, DeriveInput, Error, Expr, GenericParam, Generics, Ident,
    Lifetime, LifetimeParam, Meta, Token, Visibility, parse_quote, punctuated::Punctuated,
};

use crate::attribute::{ProjectAttribute, UnsafeClauseTarget};
use crate::field::{IdentOrIndex, ProjectField, ProjectFields};

/// A validated struct or enum awaiting projection expansion.
///
/// Construction is available only through [`ProjectItem::input`]. This keeps
/// unsupported unions, packed fields, malformed helper attributes, duplicate
/// clauses, and invalid option placement out of the expansion stage.
pub enum ProjectItem {
    /// A projected structure with potentially structurally-pinned fields.
    Struct(ProjectStruct),

    /// A projected enumeration with potentially structurally-pinned fields.
    Enum(ProjectEnum),
}

impl ProjectItem {
    /// Parse a derive input into a validated project item.
    ///
    /// Parsing preserves the complete generic signature and source visibility.
    /// It validates all type and field helper attributes before returning.
    ///
    /// # Errors
    ///
    /// Returns an error for unions, packed representations, malformed helper
    /// attributes, and invalid helper attribute placement.
    pub fn input(derive_input: DeriveInput) -> syn::Result<Self> {
        let DeriveInput {
            attrs: attribute_list,
            vis: item_visibility,
            ident: item_ident,
            generics: item_generics,
            data,
        } = derive_input;

        Self::reject_packed(&attribute_list)?;
        let project_attribute_list = Self::type_attributes(&attribute_list)?;

        match data {
            Data::Struct(struct_data) => {
                let DataStruct { fields, .. } = struct_data;
                let struct_data = ProjectStructData {
                    field_list: ProjectFields::new(fields)?,
                };

                Ok(Self::Struct(ProjectStruct {
                    attribute_list: project_attribute_list,
                    struct_visibility: item_visibility,
                    struct_ident: item_ident,
                    struct_generics: item_generics,
                    struct_data,
                }))
            }
            Data::Enum(enum_data) => {
                let DataEnum { variants, .. } = enum_data;
                let variant_list = variants
                    .into_iter()
                    .map(ProjectEnumVariant::new)
                    .collect::<syn::Result<Vec<_>>>()?;
                let enum_data = ProjectEnumData { variant_list };

                Ok(Self::Enum(ProjectEnum {
                    attribute_list: project_attribute_list,
                    enum_visibility: item_visibility,
                    enum_ident: item_ident,
                    enum_generics: item_generics,
                    enum_data,
                }))
            }
            Data::Union(..) => Err(Error::new_spanned(
                item_ident,
                "a union cannot be pin-projected",
            )),
        }
    }

    /// Expand projection types and supporting implementations.
    ///
    /// Expansion emits immutable and mutable projection types, an unsafe
    /// `nekor_project::Project` implementation, structural `Unpin` logic, and
    /// any enabled trait blockers.
    ///
    /// # Errors
    ///
    /// Returns an error if the validated field shape cannot be represented by
    /// the corresponding generated projection shape.
    pub fn expand(self) -> syn::Result<TokenStream> {
        match self {
            Self::Struct(project_struct) => project_struct.expand(),
            Self::Enum(project_enum) => project_enum.expand(),
        }
    }

    /// Parse and validate type-level project helper attributes.
    fn type_attributes(attribute_list: &[Attribute]) -> syn::Result<Vec<ProjectAttribute>> {
        let project_attribute_list = ProjectAttribute::list(attribute_list)?;
        let mut unsafe_target_list = Vec::new();

        for project_attribute in &project_attribute_list {
            match project_attribute {
                ProjectAttribute::Pin => {
                    return Err(Error::new_spanned(
                        attribute_list
                            .iter()
                            .find(|attribute| attribute.path().is_ident("project")),
                        "project pin attributes are only valid on fields",
                    ));
                }
                ProjectAttribute::UnsafeClause(target) if unsafe_target_list.contains(target) => {
                    return Err(Error::new_spanned(
                        attribute_list
                            .iter()
                            .find(|attribute| attribute.path().is_ident("project")),
                        "duplicate project unsafe clause",
                    ));
                }
                ProjectAttribute::UnsafeClause(target) => unsafe_target_list.push(*target),
            }
        }

        Ok(project_attribute_list)
    }

    /// Reject packed representations because their fields may be unaligned.
    fn reject_packed(attribute_list: &[Attribute]) -> syn::Result<()> {
        for attribute in attribute_list
            .iter()
            .filter(|attribute| attribute.path().is_ident("repr"))
        {
            let representation_list =
                attribute.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;

            if let Some(representation) = representation_list
                .iter()
                .find(|representation| representation.path().is_ident("packed"))
            {
                return Err(Error::new_spanned(
                    representation,
                    "pin projection does not support packed representations",
                ));
            }
        }

        Ok(())
    }
}

/// A validated structure projection model.
///
/// The model owns source visibility, identity, generics, type options, and the
/// exact field shape consumed by expansion.
// NOTE(invariant): `struct_data` was parsed from the same derive input as the
// stored identity and generics. All helper attributes were validated before
// construction and the private fields prevent replacement afterward.
pub struct ProjectStruct {
    /// Type-level project helper attributes.
    attribute_list: Vec<ProjectAttribute>,

    /// Source structure visibility.
    struct_visibility: Visibility,

    /// Source structure identifier.
    struct_ident: Ident,

    /// Source structure generics.
    struct_generics: Generics,

    /// Source structure field data.
    struct_data: ProjectStructData,
}

impl ProjectStruct {
    /// Expand this pin-projected structure.
    ///
    /// # Errors
    ///
    /// Returns an error if a named source field does not contain an identifier.
    pub fn expand(self) -> syn::Result<TokenStream> {
        let Self {
            attribute_list,
            struct_visibility,
            struct_ident,
            struct_generics,
            struct_data,
        } = self;
        let ProjectStructData { field_list } = struct_data;

        Expansion::new(
            attribute_list,
            struct_visibility,
            struct_ident,
            struct_generics,
            ExpansionData::Struct(field_list),
        )
        .expand()
    }
}

/// Validated structure data relevant to projection expansion.
///
/// The contained [`ProjectFields`] distinguishes named, unnamed, empty, and
/// unit structures.
pub struct ProjectStructData {
    /// Source structure fields.
    field_list: ProjectFields,
}

/// A validated enumeration projection model.
///
/// Expansion preserves each variant name and field shape. Source discriminants
/// remain metadata only and are not assigned to projection variants.
// NOTE(invariant): `enum_data` belongs to the stored source identity and
// generic signature. Variant helper placement and every field option were
// validated before construction.
pub struct ProjectEnum {
    /// Type-level project helper attributes.
    attribute_list: Vec<ProjectAttribute>,

    /// Source enumeration visibility.
    enum_visibility: Visibility,

    /// Source enumeration identifier.
    enum_ident: Ident,

    /// Source enumeration generics.
    enum_generics: Generics,

    /// Source enumeration variant data.
    enum_data: ProjectEnumData,
}

impl ProjectEnum {
    /// Expand this pin-projected enumeration.
    ///
    /// # Errors
    ///
    /// Returns an error if a named variant field does not contain an
    /// identifier.
    pub fn expand(self) -> syn::Result<TokenStream> {
        let Self {
            attribute_list,
            enum_visibility,
            enum_ident,
            enum_generics,
            enum_data,
        } = self;
        let ProjectEnumData { variant_list } = enum_data;

        Expansion::new(
            attribute_list,
            enum_visibility,
            enum_ident,
            enum_generics,
            ExpansionData::Enum(variant_list),
        )
        .expand()
    }
}

/// Validated enumeration data relevant to projection expansion.
///
/// Variant order matches source declaration order.
pub struct ProjectEnumData {
    /// Source variants in declaration order.
    variant_list: Vec<ProjectEnumVariant>,
}

/// One validated enumeration variant.
///
/// The source discriminant is retained only so generated documentation can
/// explain that projection enums do not copy it.
pub struct ProjectEnumVariant {
    /// Source variant identifier.
    variant_ident: Ident,

    /// Source variant fields.
    field_list: ProjectFields,

    /// Source explicit discriminant.
    variant_discriminant: Option<(Token![=], Expr)>,
}

impl ProjectEnumVariant {
    /// Transform one source variant into a validated project variant.
    ///
    /// Field order, field shape, and any explicit discriminant are preserved in
    /// the model. The discriminant is not emitted on the projection variant.
    ///
    /// # Errors
    ///
    /// Returns an error when a project helper attribute is attached to the
    /// variant itself or a field helper attribute is invalid.
    pub fn new(variant: syn::Variant) -> syn::Result<Self> {
        let syn::Variant {
            attrs: attribute_list,
            ident: variant_ident,
            fields,
            discriminant: variant_discriminant,
        } = variant;

        if let Some(attribute) = attribute_list
            .iter()
            .find(|attribute| attribute.path().is_ident("project"))
        {
            return Err(Error::new_spanned(
                attribute,
                "project helper attributes are not valid on enum variants",
            ));
        }

        Ok(Self {
            variant_ident,
            field_list: ProjectFields::new(fields)?,
            variant_discriminant,
        })
    }
}

/// Validated data used by a single expansion.
enum ExpansionData {
    /// Structure fields.
    Struct(ProjectFields),

    /// Enumeration variants.
    Enum(Vec<ProjectEnumVariant>),
}

impl ExpansionData {
    /// Return every field in source declaration order.
    fn field_list(&self) -> Vec<&ProjectField> {
        match self {
            Self::Struct(field_list) => field_list.field_list().iter().collect(),
            Self::Enum(variant_list) => variant_list
                .iter()
                .flat_map(|variant| variant.field_list.field_list())
                .collect(),
        }
    }
}

/// Complete context for one derive expansion.
struct Expansion {
    /// Type-level project helper attributes.
    attribute_list: Vec<ProjectAttribute>,

    /// Visibility shared by generated projection types.
    item_visibility: Visibility,

    /// Source type identifier.
    item_ident: Ident,

    /// Source type generics.
    item_generics: Generics,

    /// Source shape and fields.
    item_data: ExpansionData,
}

impl Expansion {
    /// Construct an expansion context.
    const fn new(
        attribute_list: Vec<ProjectAttribute>,
        item_visibility: Visibility,
        item_ident: Ident,
        item_generics: Generics,
        item_data: ExpansionData,
    ) -> Self {
        Self {
            attribute_list,
            item_visibility,
            item_ident,
            item_generics,
            item_data,
        }
    }

    /// Expand projection types, the `Project` implementation, and safety rules.
    fn expand(self) -> syn::Result<TokenStream> {
        let Self {
            ref item_ident,
            ref item_generics,
            ..
        } = self;
        let projection_ident = format_ident!("{}Projection", item_ident);
        let projection_mut_ident = format_ident!("{}ProjectionMut", item_ident);
        let projection_lifetime = Self::projection_lifetime(item_generics);

        let projection = self.projection_type(&projection_ident, &projection_lifetime, false)?;
        let projection_mut =
            self.projection_type(&projection_mut_ident, &projection_lifetime, true)?;
        let project_impl = self.project_impl(
            &projection_ident,
            &projection_mut_ident,
            &projection_lifetime,
        )?;
        let unpin_impl = self.unpin_impl(&projection_lifetime);
        let blocker_list = self.blocker_list();

        Ok(quote! {
            #projection
            #projection_mut
            #project_impl
            #unpin_impl
            #(#blocker_list)*
        })
    }

    /// Expand one immutable or mutable projection type.
    fn projection_type(
        &self,
        projection_ident: &Ident,
        projection_lifetime: &Lifetime,
        mutable: bool,
    ) -> syn::Result<TokenStream> {
        let Self {
            item_visibility,
            item_ident,
            item_generics,
            item_data,
            ..
        } = self;
        let has_fields = !item_data.field_list().is_empty();
        let projection_generics =
            Self::projection_generics(item_generics, item_ident, projection_lifetime, has_fields);
        let generic_declaration = Self::generic_declaration(&projection_generics);
        let where_clause = &projection_generics.where_clause;
        let projection_doc = if mutable {
            format!("Mutable pin projection of [`{item_ident}`].")
        } else {
            format!("Immutable pin projection of [`{item_ident}`].")
        };

        match item_data {
            ExpansionData::Struct(field_list) => match field_list {
                ProjectFields::Named(..) => {
                    let field_definition_list = field_list
                        .field_list()
                        .iter()
                        .map(|field| {
                            Self::named_field_definition(field, projection_lifetime, mutable)
                        })
                        .collect::<syn::Result<Vec<_>>>()?;

                    Ok(quote! {
                        #[doc = #projection_doc]
                        #item_visibility struct #projection_ident #generic_declaration #where_clause {
                            #(#field_definition_list)*
                        }
                    })
                }
                ProjectFields::Unnamed(..) => {
                    let field_definition_list = field_list
                        .field_list()
                        .iter()
                        .map(|field| {
                            Self::unnamed_field_definition(field, projection_lifetime, mutable)
                        })
                        .collect::<Vec<_>>();

                    Ok(quote! {
                        #[doc = #projection_doc]
                        #item_visibility struct #projection_ident #generic_declaration (
                            #(#field_definition_list)*
                        ) #where_clause;
                    })
                }
                ProjectFields::Unit => Ok(quote! {
                    #[doc = #projection_doc]
                    #item_visibility struct #projection_ident #generic_declaration #where_clause;
                }),
            },
            ExpansionData::Enum(variant_list) => {
                let variant_definition_list = variant_list
                    .iter()
                    .map(|variant| Self::variant_definition(variant, projection_lifetime, mutable))
                    .collect::<syn::Result<Vec<_>>>()?;

                Ok(quote! {
                    #[doc = #projection_doc]
                    #item_visibility enum #projection_ident #generic_declaration #where_clause {
                        #(#variant_definition_list)*
                    }
                })
            }
        }
    }

    /// Expand the unsafe `Project` implementation.
    fn project_impl(
        &self,
        projection_ident: &Ident,
        projection_mut_ident: &Ident,
        projection_lifetime: &Lifetime,
    ) -> syn::Result<TokenStream> {
        let Self {
            item_ident,
            item_generics,
            item_data,
            ..
        } = self;
        let (impl_generics, type_generics, where_clause) = item_generics.split_for_impl();
        let projection_type = self.projection_type_use(projection_ident, projection_lifetime);
        let projection_mut_type =
            self.projection_type_use(projection_mut_ident, projection_lifetime);
        let immutable_body = match item_data {
            ExpansionData::Struct(field_list) => {
                Self::struct_project_body(field_list, projection_ident, false)?
            }
            ExpansionData::Enum(variant_list) => {
                Self::enum_project_body(variant_list, projection_ident, false)?
            }
        };
        let mutable_body = match item_data {
            ExpansionData::Struct(field_list) => {
                Self::struct_project_body(field_list, projection_mut_ident, true)?
            }
            ExpansionData::Enum(variant_list) => {
                Self::enum_project_body(variant_list, projection_mut_ident, true)?
            }
        };

        Ok(quote! {
            unsafe impl #impl_generics ::nekor_project::Project for #item_ident #type_generics #where_clause {
                type Projection<#projection_lifetime> = #projection_type
                where
                    Self: #projection_lifetime;

                type ProjectionMut<#projection_lifetime> = #projection_mut_type
                where
                    Self: #projection_lifetime;

                #[inline]
                fn project<#projection_lifetime>(
                    self: ::core::pin::Pin<&#projection_lifetime Self>,
                ) -> Self::Projection<#projection_lifetime> {
                    #immutable_body
                }

                #[inline]
                fn project_mut<#projection_lifetime>(
                    self: ::core::pin::Pin<&#projection_lifetime mut Self>,
                ) -> Self::ProjectionMut<#projection_lifetime> {
                    #mutable_body
                }
            }
        })
    }

    /// Expand the structural `Unpin` implementation when it is not opted out.
    fn unpin_impl(&self, projection_lifetime: &Lifetime) -> TokenStream {
        let Self {
            attribute_list,
            item_ident,
            item_generics,
            item_data,
            ..
        } = self;

        if attribute_list.contains(&ProjectAttribute::UnsafeClause(UnsafeClauseTarget::Unpin)) {
            return TokenStream::new();
        }

        let pinned_field_list = item_data
            .field_list()
            .into_iter()
            .filter(|field| field.pinned())
            .collect::<Vec<_>>();

        if pinned_field_list.is_empty() {
            let (impl_generics, type_generics, where_clause) = item_generics.split_for_impl();

            return quote! {
                impl #impl_generics ::core::marker::Unpin for #item_ident #type_generics #where_clause {}
            };
        }

        let helper_ident = format_ident!("__NekorProjectUnpin{item_ident}");
        let mut helper_generics = item_generics.clone();
        let source_parameter_list = core::mem::take(&mut helper_generics.params);
        helper_generics
            .params
            .push(GenericParam::Lifetime(LifetimeParam::new(
                projection_lifetime.clone(),
            )));
        helper_generics.params.extend(source_parameter_list);

        let helper_generic_declaration = Self::generic_declaration(&helper_generics);
        let helper_where_clause = &helper_generics.where_clause;
        let helper_type = self.projection_type_use(&helper_ident, projection_lifetime);
        let (_, source_type_generics, _) = item_generics.split_for_impl();
        let pinned_marker_list = pinned_field_list
            .iter()
            .enumerate()
            .map(|(index, field)| {
                let marker_ident = format_ident!("pinned_field_{index}");
                let field_type = field.ty();

                quote! {
                    /// Carries the `Unpin` state of one structurally-pinned field.
                    #marker_ident: ::core::marker::PhantomData<#field_type>,
                }
            })
            .collect::<Vec<_>>();
        let mut unpin_generics = item_generics.clone();
        unpin_generics
            .make_where_clause()
            .predicates
            .push(parse_quote!(for<#projection_lifetime> #helper_type: ::core::marker::Unpin));
        let (impl_generics, type_generics, where_clause) = unpin_generics.split_for_impl();

        quote! {
            const _: () = {
                /// Carries only the structural pinning state used for `Unpin`.
                struct #helper_ident #helper_generic_declaration #helper_where_clause {
                    /// Uses every source generic without inheriting its auto traits.
                    source_generics: ::core::marker::PhantomData<fn() -> #item_ident #source_type_generics>,

                    /// Makes the helper condition dependent for concrete fields.
                    projection_lifetime: ::core::marker::PhantomData<&#projection_lifetime ()>,

                    #(#pinned_marker_list)*
                }

                impl #impl_generics ::core::marker::Unpin for #item_ident #type_generics #where_clause {}
            };
        }
    }

    /// Expand coherence blockers for unsafe traits that were not opted out.
    fn blocker_list(&self) -> Vec<TokenStream> {
        let Self {
            attribute_list,
            item_data,
            ..
        } = self;
        let has_pinned_fields = item_data.field_list().into_iter().any(ProjectField::pinned);

        if !has_pinned_fields {
            return Vec::new();
        }

        [
            (UnsafeClauseTarget::Drop, quote!(::core::ops::Drop), "Drop"),
            (
                UnsafeClauseTarget::Deref,
                quote!(::core::ops::Deref),
                "Deref",
            ),
            (
                UnsafeClauseTarget::DerefMut,
                quote!(::core::ops::DerefMut),
                "DerefMut",
            ),
        ]
        .into_iter()
        .filter(|(target, ..)| !attribute_list.contains(&ProjectAttribute::UnsafeClause(*target)))
        .map(|(_, trait_path, trait_name)| self.blocker(&trait_path, trait_name))
        .collect()
    }

    /// Expand one coherence blocker.
    fn blocker(&self, trait_path: &TokenStream, trait_name: &str) -> TokenStream {
        let Self {
            item_ident,
            item_generics,
            ..
        } = self;
        let (impl_generics, type_generics, where_clause) = item_generics.split_for_impl();
        let blocker_ident = format_ident!("__NekorProjectMustNotImpl{trait_name}");

        quote! {
            const _: () = {
                trait #blocker_ident {}

                impl<T: #trait_path> #blocker_ident for T {}

                impl #impl_generics #blocker_ident for #item_ident #type_generics #where_clause {}
            };
        }
    }

    /// Expand a named projection field definition.
    fn named_field_definition(
        field: &ProjectField,
        projection_lifetime: &Lifetime,
        mutable: bool,
    ) -> syn::Result<TokenStream> {
        let field_ident = Self::named_ident(field)?;
        let field_visibility = field.visibility();
        let field_type = Self::field_type(field, projection_lifetime, mutable);
        let field_doc = format!("Projection of `{field_ident}`.");

        Ok(quote! {
            #[doc = #field_doc]
            #field_visibility #field_ident: #field_type,
        })
    }

    /// Expand an unnamed projection field definition.
    fn unnamed_field_definition(
        field: &ProjectField,
        projection_lifetime: &Lifetime,
        mutable: bool,
    ) -> TokenStream {
        let field_visibility = field.visibility();
        let field_type = Self::field_type(field, projection_lifetime, mutable);

        quote! {
            /// Projection of the corresponding positional source field.
            #field_visibility #field_type,
        }
    }

    /// Expand one projection enumeration variant.
    fn variant_definition(
        variant: &ProjectEnumVariant,
        projection_lifetime: &Lifetime,
        mutable: bool,
    ) -> syn::Result<TokenStream> {
        let ProjectEnumVariant {
            variant_ident,
            field_list,
            variant_discriminant,
        } = variant;
        let variant_doc = if variant_discriminant.is_some() {
            format!("Projection of `{variant_ident}` without its source discriminant.")
        } else {
            format!("Projection of `{variant_ident}`.")
        };

        match field_list {
            ProjectFields::Named(..) => {
                let field_definition_list = field_list
                    .field_list()
                    .iter()
                    .map(|field| {
                        let field_ident = Self::named_ident(field)?;
                        let field_type = Self::field_type(field, projection_lifetime, mutable);
                        let field_doc = format!("Projection of `{field_ident}`.");

                        Ok(quote! {
                            #[doc = #field_doc]
                            #field_ident: #field_type,
                        })
                    })
                    .collect::<syn::Result<Vec<_>>>()?;

                Ok(quote! {
                    #[doc = #variant_doc]
                    #variant_ident {
                        #(#field_definition_list)*
                    },
                })
            }
            ProjectFields::Unnamed(..) => {
                let field_type_list = field_list
                    .field_list()
                    .iter()
                    .enumerate()
                    .map(|(index, field)| {
                        let field_type = Self::field_type(field, projection_lifetime, mutable);
                        let field_doc = format!("Projection of positional field {index}.");

                        quote! {
                            #[doc = #field_doc]
                            #field_type
                        }
                    })
                    .collect::<Vec<_>>();

                Ok(quote! {
                    #[doc = #variant_doc]
                    #variant_ident(#(#field_type_list),*),
                })
            }
            ProjectFields::Unit => Ok(quote! {
                #[doc = #variant_doc]
                #variant_ident,
            }),
        }
    }

    /// Expand one structure projection body.
    fn struct_project_body(
        field_list: &ProjectFields,
        projection_ident: &Ident,
        mutable: bool,
    ) -> syn::Result<TokenStream> {
        // NOTE(invariant): A mutable projection consumes the unique pinned
        // borrow and immediately splits it into disjoint field borrows. The
        // generated code never moves the source structure or any field.
        let target_value = if mutable {
            quote! {
                unsafe { ::core::pin::Pin::get_unchecked_mut(self) }
            }
        } else {
            quote! {
                ::core::pin::Pin::get_ref(self)
            }
        };

        match field_list {
            ProjectFields::Named(..) => {
                let field_ident_list = field_list
                    .field_list()
                    .iter()
                    .map(Self::named_ident)
                    .collect::<syn::Result<Vec<_>>>()?;
                let projection_list = field_list
                    .field_list()
                    .iter()
                    .zip(&field_ident_list)
                    .map(|(field, field_ident)| {
                        let projection = Self::field_projection(field, quote!(#field_ident));
                        quote!(#field_ident: #projection)
                    })
                    .collect::<Vec<_>>();

                Ok(quote! {
                    let Self { #(#field_ident_list),* } = #target_value;
                    #projection_ident { #(#projection_list),* }
                })
            }
            ProjectFields::Unnamed(..) => {
                let binding_list = (0..field_list.field_list().len())
                    .map(|index| format_ident!("target_field_{index}"))
                    .collect::<Vec<_>>();
                let projection_list = field_list
                    .field_list()
                    .iter()
                    .zip(&binding_list)
                    .map(|(field, binding)| Self::field_projection(field, quote!(#binding)))
                    .collect::<Vec<_>>();

                Ok(quote! {
                    let Self(#(#binding_list),*) = #target_value;
                    #projection_ident(#(#projection_list),*)
                })
            }
            ProjectFields::Unit => Ok(quote!(#projection_ident)),
        }
    }

    /// Expand one enumeration projection body.
    fn enum_project_body(
        variant_list: &[ProjectEnumVariant],
        projection_ident: &Ident,
        mutable: bool,
    ) -> syn::Result<TokenStream> {
        // NOTE(invariant): A mutable projection consumes the unique pinned
        // borrow and matches without replacing the discriminant. Every match
        // arm creates disjoint borrows of the active variant fields.
        let target_value = if mutable {
            quote! {
                unsafe { ::core::pin::Pin::get_unchecked_mut(self) }
            }
        } else {
            quote! {
                ::core::pin::Pin::get_ref(self)
            }
        };
        let arm_list = variant_list
            .iter()
            .map(|variant| Self::variant_arm(variant, projection_ident))
            .collect::<syn::Result<Vec<_>>>()?;

        Ok(quote! {
            match #target_value {
                #(#arm_list)*
            }
        })
    }

    /// Expand one enumeration projection match arm.
    fn variant_arm(
        variant: &ProjectEnumVariant,
        projection_ident: &Ident,
    ) -> syn::Result<TokenStream> {
        let ProjectEnumVariant {
            variant_ident,
            field_list,
            ..
        } = variant;

        match field_list {
            ProjectFields::Named(..) => {
                let field_ident_list = field_list
                    .field_list()
                    .iter()
                    .map(Self::named_ident)
                    .collect::<syn::Result<Vec<_>>>()?;
                let projection_list = field_list
                    .field_list()
                    .iter()
                    .zip(&field_ident_list)
                    .map(|(field, field_ident)| {
                        let projection = Self::field_projection(field, quote!(#field_ident));
                        quote!(#field_ident: #projection)
                    })
                    .collect::<Vec<_>>();

                Ok(quote! {
                    Self::#variant_ident { #(#field_ident_list),* } =>
                        #projection_ident::#variant_ident { #(#projection_list),* },
                })
            }
            ProjectFields::Unnamed(..) => {
                let binding_list = (0..field_list.field_list().len())
                    .map(|index| format_ident!("target_field_{index}"))
                    .collect::<Vec<_>>();
                let projection_list = field_list
                    .field_list()
                    .iter()
                    .zip(&binding_list)
                    .map(|(field, binding)| Self::field_projection(field, quote!(#binding)))
                    .collect::<Vec<_>>();

                Ok(quote! {
                    Self::#variant_ident(#(#binding_list),*) =>
                        #projection_ident::#variant_ident(#(#projection_list),*),
                })
            }
            ProjectFields::Unit => Ok(quote! {
                Self::#variant_ident => #projection_ident::#variant_ident,
            }),
        }
    }

    /// Return a projected field type.
    fn field_type(
        field: &ProjectField,
        projection_lifetime: &Lifetime,
        mutable: bool,
    ) -> TokenStream {
        let field_type = field.ty();

        match (field.pinned(), mutable) {
            (true, true) => quote!(::core::pin::Pin<&#projection_lifetime mut #field_type>),
            (true, false) => quote!(::core::pin::Pin<&#projection_lifetime #field_type>),
            (false, true) => quote!(&#projection_lifetime mut #field_type),
            (false, false) => quote!(&#projection_lifetime #field_type),
        }
    }

    // NOTE(invariant): The binding borrows a field from a pinned source value.
    // Generated `Unpin` and trait blockers preserve the location of every field
    // marked as structurally pinned for the lifetime of the returned projection.
    /// Return an expression that projects one field binding.
    fn field_projection(field: &ProjectField, binding: TokenStream) -> TokenStream {
        if field.pinned() {
            quote! {
                unsafe { ::core::pin::Pin::new_unchecked(#binding) }
            }
        } else {
            binding
        }
    }

    /// Return a named field identifier.
    fn named_ident(field: &ProjectField) -> syn::Result<&Ident> {
        match field.name() {
            IdentOrIndex::Ident(field_ident) => Ok(field_ident),
            IdentOrIndex::Index(field_index) => Err(Error::new(
                field_index.span,
                "a named field is missing its identifier",
            )),
        }
    }

    /// Select a collision-free generated projection lifetime.
    fn projection_lifetime(generics: &Generics) -> Lifetime {
        let mut index = 0_usize;

        loop {
            let name = if index == 0 {
                "'project".into()
            } else {
                format!("'project_{index}")
            };
            let collides = generics
                .lifetimes()
                .any(|parameter| parameter.lifetime.ident == name.trim_start_matches('\''));

            if !collides {
                return Lifetime::new(&name, Span::call_site());
            }

            index += 1;
        }
    }

    /// Construct generic parameters for a generated projection type.
    fn projection_generics(
        source_generics: &Generics,
        source_ident: &Ident,
        projection_lifetime: &Lifetime,
        has_fields: bool,
    ) -> Generics {
        let mut projection_generics = source_generics.clone();

        if has_fields {
            let source_parameters = core::mem::take(&mut projection_generics.params);
            projection_generics
                .params
                .push(GenericParam::Lifetime(LifetimeParam::new(
                    projection_lifetime.clone(),
                )));
            projection_generics.params.extend(source_parameters);

            let (_, source_type_generics, _) = source_generics.split_for_impl();
            projection_generics
                .make_where_clause()
                .predicates
                .push(parse_quote!(#source_ident #source_type_generics: #projection_lifetime));
        }

        projection_generics
    }

    /// Return a generic parameter declaration without its where clause.
    fn generic_declaration(generics: &Generics) -> TokenStream {
        let parameter_list = &generics.params;

        if parameter_list.is_empty() {
            TokenStream::new()
        } else {
            quote!(<#parameter_list>)
        }
    }

    /// Return a generated projection type use.
    fn projection_type_use(
        &self,
        projection_ident: &Ident,
        projection_lifetime: &Lifetime,
    ) -> TokenStream {
        let Self {
            item_generics,
            item_data,
            ..
        } = self;
        let argument_list = item_generics
            .params
            .iter()
            .map(|parameter| match parameter {
                GenericParam::Lifetime(parameter) => {
                    let lifetime = &parameter.lifetime;
                    quote!(#lifetime)
                }
                GenericParam::Type(parameter) => {
                    let ident = &parameter.ident;
                    quote!(#ident)
                }
                GenericParam::Const(parameter) => {
                    let ident = &parameter.ident;
                    quote!(#ident)
                }
            })
            .collect::<Vec<_>>();

        if !item_data.field_list().is_empty() {
            quote!(#projection_ident<#projection_lifetime #(, #argument_list)*>)
        } else if argument_list.is_empty() {
            quote!(#projection_ident)
        } else {
            quote!(#projection_ident<#(#argument_list),*>)
        }
    }
}

#[cfg(test)]
mod tests {
    //! Parser regression tests for safety-sensitive helper attributes.

    use syn::{DeriveInput, parse_quote};

    use super::ProjectItem;

    /// Assert that an invalid derive input is rejected during parsing.
    fn rejects(input: DeriveInput) {
        assert!(ProjectItem::input(input).is_err());
    }

    #[test]
    fn rejects_union_projection() {
        rejects(parse_quote! {
            union Target {
                value: u32,
            }
        });
    }

    #[test]
    fn rejects_packed_projection() {
        rejects(parse_quote! {
            #[repr(C, packed(2))]
            struct Target {
                value: u32,
            }
        });
    }

    #[test]
    fn rejects_type_level_pin() {
        rejects(parse_quote! {
            #[project(pin)]
            struct Target;
        });
    }

    #[test]
    fn rejects_field_level_unsafe_clause() {
        rejects(parse_quote! {
            struct Target {
                #[project(unsafe = Drop)]
                value: u32,
            }
        });
    }

    #[test]
    fn rejects_variant_level_attribute() {
        rejects(parse_quote! {
            enum Target {
                #[project(pin)]
                Value,
            }
        });
    }

    #[test]
    fn rejects_duplicate_unsafe_clause() {
        rejects(parse_quote! {
            #[project(unsafe = Drop)]
            #[project(unsafe = Drop)]
            struct Target;
        });
    }

    #[test]
    fn rejects_malformed_helper_attribute() {
        rejects(parse_quote! {
            struct Target {
                #[project(pin, pin)]
                value: u32,
            }
        });
    }
}

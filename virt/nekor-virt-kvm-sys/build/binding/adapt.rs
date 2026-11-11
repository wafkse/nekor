//! Substitute the local flexible array representation in Bindgen output.
//!
//! Processed declarations are checked against a sanitized copy of the Bindgen output.

use alloc::collections::BTreeSet;

use syn::{ImplItem, Item, Type, visit_mut::VisitMut};

use super::error::AdaptationError;

/// Bindgen's bitfield-unit implementation contains generated accessors replaced by the adapter.
const BINDGEN_BITFIELD_UNIT_TYPE: &str = "__BindgenBitfieldUnit";

/// Bindgen's name for the helper representing C flexible array members.
///
/// C uses these members for dynamically sized trailing storage in a record.
///
/// Bindgen preserves the member's alignment with a zero-sized generic helper.
const BINDGEN_FLEXIBLE_ARRAY_TYPE: &str = "__IncompleteArrayField";

/// A Syn visitor for the local flexible array member representation.
// NOTE(invariant): None.
pub struct Adapter<'binding> {
    /// Bindgen output used to retain original declaration layouts.
    default_binding_ast: &'binding syn::File,
}

/// The processed binding and a layout-only copy of the original declarations.
pub struct AdaptedBinding {
    /// FAM and bitfield-adapted declarations with layout assertions.
    pub processed_binding_ast: syn::File,

    /// Original types with generated bitfield implementations removed.
    pub layout_binding_ast: syn::File,
}

impl<'binding> Adapter<'binding> {
    /// Borrow the default AST used for layout comparisons.
    #[inline]
    pub const fn new(default_binding_ast: &'binding syn::File) -> Self {
        Self { default_binding_ast }
    }

    /// Produce FAM-adapted declarations and a sanitized layout reference.
    ///
    /// # Failure
    ///
    /// This fails if Bindgen does not emit all flexible-array helper accessors.
    pub fn apply(
        mut self,
        mut processed_binding_ast: syn::File,
        removed_record_names: &BTreeSet<String>,
    ) -> Result<AdaptedBinding, AdaptationError> {
        let Self { default_binding_ast } = self;

        let mut layout_binding_ast = default_binding_ast.clone();

        self.visit_file_mut(&mut processed_binding_ast);

        layout_binding_ast
            .items
            .retain(|item| !Self::bitfield_implementation(item, removed_record_names));
        Self::constify_flexible_array_accessors(&mut layout_binding_ast)?;

        let layout_assertion_list = Self::checks(&processed_binding_ast);
        let layout_binding_relative_file = crate::binding::LAYOUT_BINDING_RELATIVE_FILE;

        processed_binding_ast.items.push(syn::parse_quote! {
            #[expect(dead_code, reason = "The original interface supplies layout evidence only.")]
            mod vanilla {
                include!(#layout_binding_relative_file);
            }
        });
        processed_binding_ast.items.push(syn::parse_quote! {
            const _: () = {
                #(#layout_assertion_list)*
            };
        });

        Ok(AdaptedBinding {
            processed_binding_ast,
            layout_binding_ast,
        })
    }

    /// Make Bindgen's flexible-array helper accessors const in the layout reference.
    fn constify_flexible_array_accessors(layout_binding_ast: &mut syn::File) -> Result<(), AdaptationError> {
        let mut accessor_name_set = BTreeSet::new();

        for binding_item in &mut layout_binding_ast.items {
            let implementation = match binding_item {
                Item::Impl(implementation) => implementation,
                _ => continue,
            };

            match implementation.self_ty.as_ref() {
                Type::Path(implemented_type) => match implemented_type.path.segments.last() {
                    Some(segment) if segment.ident == BINDGEN_FLEXIBLE_ARRAY_TYPE => {},
                    _ => continue,
                },
                _ => continue,
            }

            for implementation_item in &mut implementation.items {
                let ImplItem::Fn(function) = implementation_item else {
                    continue;
                };

                let accessor_name = function.sig.ident.to_string();

                match accessor_name.as_str() {
                    "as_ptr" | "as_mut_ptr" | "as_slice" | "as_mut_slice" => {
                        if !accessor_name_set.insert(accessor_name.clone()) {
                            Err(AdaptationError::Duplicate(format!(
                                "Bindgen generated `__IncompleteArrayField::{accessor_name}` more than once"
                            )))?;
                        }

                        function.sig.constness = Some(syn::parse_quote!(const));
                    },
                    _ => continue,
                }
            }
        }

        let missing_accessor_name_list = ["as_ptr", "as_mut_ptr", "as_slice", "as_mut_slice"]
            .into_iter()
            .filter(|accessor_name| !accessor_name_set.contains(*accessor_name))
            .collect::<Vec<_>>();

        match missing_accessor_name_list.as_slice() {
            [] => Ok(()),
            missing_accessor_name_list => Err(AdaptationError::Missing(format!(
                "Bindgen omitted `__IncompleteArrayField` accessors {}",
                missing_accessor_name_list.join(", ")
            ))),
        }
    }

    /// Remove Bindgen implementations that contain the replaced bitfield accessors.
    fn bitfield_implementation(binding_item: &Item, removed_record_names: &BTreeSet<String>) -> bool {
        match binding_item {
            Item::Impl(implementation) => match implementation.self_ty.as_ref() {
                Type::Path(implemented_type) => match implemented_type.path.segments.last() {
                    Some(type_segment) => {
                        let type_name = type_segment.ident.to_string();

                        type_name == BINDGEN_BITFIELD_UNIT_TYPE || removed_record_names.contains(&type_name)
                    },
                    None => false,
                },
                _ => false,
            },
            _ => false,
        }
    }

    /// Emit comparisons for each nongeneric struct or union in the processed AST.
    fn checks(processed_binding_ast: &syn::File) -> Vec<syn::Stmt> {
        let mut layout_assertion_list = Vec::new();

        for binding_item in &processed_binding_ast.items {
            let (type_identifier, named_field_list) = match binding_item {
                Item::Struct(struct_declaration) if struct_declaration.generics.params.is_empty() => {
                    let named_field_list = match &struct_declaration.fields {
                        syn::Fields::Named(syn::FieldsNamed {
                            named: named_field_list,
                            ..
                        }) => Some(named_field_list),
                        _ => None,
                    };

                    (&struct_declaration.ident, named_field_list)
                },
                Item::Union(union_declaration) if union_declaration.generics.params.is_empty() => {
                    (&union_declaration.ident, Some(&union_declaration.fields.named))
                },
                _ => continue,
            };

            layout_assertion_list.extend(Self::layout(type_identifier));

            for member_declaration in named_field_list.into_iter().flatten() {
                layout_assertion_list.extend(Self::offset(type_identifier, member_declaration));
            }
        }

        layout_assertion_list
    }

    /// Emit size and alignment assertions against the corresponding default type.
    fn layout(type_identifier: &syn::Ident) -> Vec<syn::Stmt> {
        vec![
            syn::parse_quote! {
                assert!(::core::mem::size_of::<#type_identifier>() == ::core::mem::size_of::<vanilla::#type_identifier>());
            },
            syn::parse_quote! {
                assert!(::core::mem::align_of::<#type_identifier>() == ::core::mem::align_of::<vanilla::#type_identifier>());
            },
        ]
    }

    /// Emit a member offset assertion when the declaration has a named field.
    fn offset(type_identifier: &syn::Ident, member_declaration: &syn::Field) -> Option<syn::Stmt> {
        member_declaration.ident.as_ref().map(|member_identifier| {
            syn::parse_quote! {
                assert!(::core::mem::offset_of!(#type_identifier, #member_identifier) == ::core::mem::offset_of!(vanilla::#type_identifier, #member_identifier));
            }
        })
    }

    /// Recognize the flexible array helper struct and its associated implementations.
    fn helper(binding_item: &Item) -> bool {
        let declared_type_identifier = match binding_item {
            Item::Struct(struct_declaration) => Some(&struct_declaration.ident),
            Item::Impl(impl_declaration) => match impl_declaration.self_ty.as_ref() {
                Type::Path(implemented_type) => implemented_type
                    .path
                    .segments
                    .last()
                    .map(|type_segment| &type_segment.ident),
                _ => None,
            },
            _ => None,
        };

        declared_type_identifier.is_some_and(|type_identifier| type_identifier == BINDGEN_FLEXIBLE_ARRAY_TYPE)
    }
}

impl VisitMut for Adapter<'_> {
    fn visit_file_mut(&mut self, processed_binding_ast: &mut syn::File) {
        // NOTE: The default file contains Bindgen's C-layout assertions. The processed
        // file emits comparisons with those declarations after replacing the helper.
        processed_binding_ast.items.retain(|binding_item| match binding_item {
            Item::Const(constant_declaration) if constant_declaration.ident == "_" => false,
            _ => !Self::helper(binding_item),
        });

        syn::visit_mut::visit_file_mut(self, processed_binding_ast);
    }

    fn visit_type_path_mut(&mut self, member_type: &mut syn::TypePath) {
        match member_type.path.segments.last() {
            Some(member_type_segment) if member_type_segment.ident == BINDGEN_FLEXIBLE_ARRAY_TYPE => {
                let element_type_argument_list = &member_type_segment.arguments;

                member_type.path = syn::parse_quote!(crate::ffi::layout::fam::Fam #element_type_argument_list);
            },
            _ => {},
        }

        syn::visit_mut::visit_type_path_mut(self, member_type);
    }
}

//! Substitute the local flexible array representation in Bindgen output.
//!
//! Processed declarations are checked against the default file during compilation.

use syn::{Item, Type, visit_mut::VisitMut};

/// A Syn visitor for the local flexible array member representation.
///
/// The default AST is cloned and retained as the reference for size, alignment,
/// and named member-offset assertions.
// NOTE(invariant): None.
pub struct Adapter<'binding> {
    /// Bindgen output used as the reference for processed declaration layouts.
    default_binding_ast: &'binding syn::File,
}

impl<'binding> Adapter<'binding> {
    /// Borrow the default AST used for layout comparisons.
    #[inline]
    pub const fn new(default_binding_ast: &'binding syn::File) -> Self {
        Self { default_binding_ast }
    }

    /// Produce the processed AST with flexible member substitution and layout assertions.
    pub fn apply(mut self) -> syn::File {
        let Self { default_binding_ast } = self;

        let mut processed_binding_ast = default_binding_ast.clone();

        self.visit_file_mut(&mut processed_binding_ast);

        let layout_assertion_list = Self::checks(&processed_binding_ast);

        processed_binding_ast.items.push(syn::parse_quote! {
            #[expect(dead_code, reason = "The default interface supplies layout evidence only.")]
            mod vanilla {
                include!("binding-default.rs");
            }
        });
        processed_binding_ast.items.push(syn::parse_quote! {
            const _: () = {
                #(#layout_assertion_list)*
            };
        });

        processed_binding_ast
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

        declared_type_identifier.is_some_and(|type_identifier| type_identifier == "__IncompleteArrayField")
    }
}

impl VisitMut for Adapter<'_> {
    fn visit_file_mut(&mut self, processed_binding_ast: &mut syn::File) {
        // NOTE: The default file retains Bindgen's C-layout assertions. The processed
        // file emits comparisons with those declarations after replacing the helper.
        processed_binding_ast.items.retain(|binding_item| match binding_item {
            Item::Const(constant_declaration) if constant_declaration.ident == "_" => false,
            _ => !Self::helper(binding_item),
        });

        syn::visit_mut::visit_file_mut(self, processed_binding_ast);
    }

    fn visit_type_path_mut(&mut self, member_type: &mut syn::TypePath) {
        match member_type.path.segments.last() {
            Some(member_type_segment) if member_type_segment.ident == "__IncompleteArrayField" => {
                let element_type_argument_list = &member_type_segment.arguments;

                member_type.path = syn::parse_quote!(crate::lowlevel::fam::Fam #element_type_argument_list);
            },
            _ => {},
        }

        syn::visit_mut::visit_type_path_mut(self, member_type);
    }
}

//! Metadata trait generator infrastructure.

use proc_macro2::TokenStream;

use quote::quote;

use syn::{
    Ident, Token,
    parse::{Parse, ParseStream},
};

use crate::primitive::Primitive;

/// The input macro structure expected by the `metadata` proc-macro.
#[derive(Clone)]
pub struct Metadata {
    /// A marker token to indicate whether to include a `detail::Sealed` trait
    /// in the output.
    ///
    /// This permits the existence of multiple [`Metadata`] expansions in the
    /// same module.
    become_token: Option<Token![become]>,

    /// The identifier indicating the input type.
    input_type: Primitive,

    /// The identifier indicating the output type.
    output_type: Primitive,

    /// The name of the trait to-be-exposed by the associated macro.
    trait_name: Ident,
}

impl Metadata {
    /// Expand to the actual metadata trait definition.
    ///
    /// # Errors
    ///
    /// This is infallible, but may in turn become fallible in the future.
    pub fn expand(self) -> syn::Result<TokenStream> {
        let Self {
            become_token,
            input_type,
            output_type,
            trait_name,
            ..
        } = self;

        let trait_name_string = trait_name.to_string();
        let trait_name_slice = trait_name_string.as_str();

        let type_name_in = input_type.as_str();
        let type_name_out = output_type.as_str();

        let optional_impl_detail = match become_token {
            Some(..) => {
                let detail_module_doc =
                    format!("Implementation details for the `{trait_name_slice}*`-related traits.");

                let sealed_trait_doc = format!(
                    "A trait to act as a supertrait seal for the `{trait_name_slice}*`-related \
                     traits."
                );

                Some(quote! {
                    #[doc(hidden)]
                    mod detail {
                        #![doc = #detail_module_doc]

                        #![doc = #sealed_trait_doc]
                        pub trait Sealed {}
                    }
                })
            }
            None => None,
        };

        let metadata_trait_doc =
            format!("A metadata item about a {type_name_out}-from-{type_name_in} extraction.");

        let metadata_trait = quote! {
            #optional_impl_detail

            #[doc = #metadata_trait_doc]
            pub trait #trait_name<const N: usize, const M: usize>: detail::Sealed {
                /// The number of bits to be extracted.
                const BITSET_WIDTH: usize;

                /// The bitwise mask composed of all to-be-extracted bits.
                const BITSET_MASK: Self;

                /// The fuse mask for the extract operation.
                const EXTRACT_MASK: Self;

                /// The fuse mask for the merge operation.
                const FUSE_MASK: Self;
            }
        };

        Ok(metadata_trait)
    }
}

impl Metadata {
    /// Determine the input [`Primitive`] in this metadata invocation.
    #[inline]
    #[must_use]
    pub const fn input(&self) -> &Primitive {
        let Self { input_type, .. } = self;

        input_type
    }

    /// Determine the output [`Primitive`] in this metadata invocation.
    #[inline]
    #[must_use]
    pub const fn output(&self) -> &Primitive {
        let Self { output_type, .. } = self;

        output_type
    }

    /// Determine the trait name in this metadata invocation.
    #[inline]
    #[must_use]
    pub const fn name(&self) -> &Ident {
        let Self { trait_name, .. } = self;

        trait_name
    }
}

impl Parse for Metadata {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let become_token = input.parse()?;

        let _: Token![for] = input.parse()?;

        let input_type = input.parse()?;

        let _: Token![->] = input.parse()?;

        let output_type = input.parse()?;

        let _: Token![as] = input.parse()?;

        let trait_name = input.parse()?;

        Ok(Self {
            become_token,
            input_type,
            output_type,
            trait_name,
        })
    }
}

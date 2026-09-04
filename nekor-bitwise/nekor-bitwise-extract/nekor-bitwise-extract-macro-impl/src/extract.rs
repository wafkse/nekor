//! Extract trait implementation macros.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    Ident, LitInt, Path, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::Bracket,
};

use crate::{implement::BitRanges, primitive::Primitive};

/// A forwarded trait implementation specification.
///
/// This structure represents a directive to forward trait implementations
/// from a source trait to a target trait for a specific target type and a
/// list of output types.
///
/// # Syntax
///
/// ```text
/// target_type use source_trait become [output_type, ...]
/// ```
///
/// # Example
///
/// ```text
/// u16 use MetadataU16 become [u8, u16]
/// ```
pub struct Forwarded {
    /// The target type for which to generate the forwarded implementation.
    target_type: Primitive,

    /// The source trait to forward implementations from.
    source_trait: Path,

    /// A comma-separated list of output types to generate implementations for.
    output_type_list: Punctuated<Primitive, Token![,]>,
}

impl Forwarded {
    /// Expand the forwarded trait implementation.
    ///
    /// This generates trait implementations that forward from the source trait
    /// to the target trait for each output type in the list.
    ///
    /// # Errors
    ///
    /// This function is currently infallible, but may become fallible in the
    /// future.
    pub fn expand(self, target_trait: &Path) -> syn::Result<TokenStream> {
        let Self {
            target_type,
            source_trait,
            output_type_list,
            ..
        } = self;

        let target_type_name = target_type.as_str();
        let target_type_ident = target_type.as_ident();

        let mut impls = Vec::new();

        for output_type in output_type_list {
            let output_type_name = output_type.as_str();
            let output_type_ident = output_type.as_ident();

            // Generate implementations for all valid bit ranges
            let ranges = BitRanges::inout(target_type.clone(), output_type.clone());

            for range in ranges {
                let start = *range.start();
                let end = *range.end();

                let start_lit = LitInt::new(&start.to_string(), Span::call_site());
                let end_lit = LitInt::new(&end.to_string(), Span::call_site());

                let impl_doc = format!(
                    "Forward {output_type_name}-from-{target_type_name} extraction for bits \
                     {start}..={end}."
                );

                impls.push(quote! {
                    #[doc = #impl_doc]
                    impl #target_trait<#start_lit, #end_lit> for #target_type_ident {
                        type Output = #output_type_ident;

                        const BITSET_WIDTH: usize = <#target_type_ident as #source_trait<#start_lit, #end_lit, #output_type_ident>>::BITSET_WIDTH;

                        const BITSET_MASK: Self = <#target_type_ident as #source_trait<#start_lit, #end_lit, #output_type_ident>>::BITSET_MASK;

                        const EXTRACT_MASK: Self = <#target_type_ident as #source_trait<#start_lit, #end_lit, #output_type_ident>>::EXTRACT_MASK;

                        const FUSE_MASK: Self = <#target_type_ident as #source_trait<#start_lit, #end_lit, #output_type_ident>>::FUSE_MASK;

                        #[inline]
                        fn extract(&self) -> Self::Output {
                            Extractor::<#start_lit, #end_lit, Self, Self::Output>::extract(self)
                        }

                        #[inline]
                        fn merge(&mut self, target_value: Self::Output) -> Self::Output {
                            Extractor::<#start_lit, #end_lit, Self, Self::Output>::merge(self, target_value)
                        }
                    }
                });
            }
        }

        Ok(quote! {
            #(#impls)*
        })
    }
}

impl Parse for Forwarded {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let target_type = input.parse()?;

        let _: Token![use] = input.parse()?;

        let source_trait = input.parse()?;

        let _: Token![become] = input.parse()?;

        let content;
        let _ = syn::bracketed!(content in input);

        let output_type_list = content.parse_terminated(Primitive::parse, Token![,])?;

        Ok(Self {
            target_type,
            source_trait,
            output_type_list,
        })
    }
}

/// An extract macro invocation.
///
/// This enumeration represents the two modes of the extract macro:
/// splitting work across multiple invocations, or forwarding a single
/// trait implementation.
pub enum Extract {
    /// Split off the work between multiple invocations of the same [`Extract`]
    /// macro.
    ///
    /// This generates separate private modules for each forwarded
    /// implementation, allowing the same macro to be invoked multiple times
    /// with different configurations.
    ///
    /// # Syntax
    ///
    /// ```text
    /// target_trait for [forwarded, ...]
    /// ```
    ///
    /// # Example
    ///
    /// ```text
    /// MyExtractTrait for [
    ///     u16 use MetadataU16 become [u8, u16],
    ///     u32 use MetadataU32 become [u8, u16, u32],
    /// ]
    /// ```
    Splitoff(Path, Token![for], Bracket, Punctuated<Forwarded, Token![,]>),

    /// Forward the implementation of a source trait to a target trait for
    /// the same type.
    ///
    /// This directly expands a single forwarded implementation without
    /// creating a separate module.
    ///
    /// # Syntax
    ///
    /// ```text
    /// trait target_trait for forwarded
    /// ```
    ///
    /// # Example
    ///
    /// ```text
    /// trait MyExtractTrait for u16 use MetadataU16 become [u8, u16]
    /// ```
    Forward(Token![trait], Path, Token![for], Forwarded),
}

impl Parse for Extract {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        if lookahead.peek(Token![trait]) {
            let trait_token = input.parse()?;

            let target_trait = input.parse()?;

            let for_token = input.parse()?;

            let forwarded = input.parse()?;

            Ok(Self::Forward(trait_token, target_trait, for_token, forwarded))
        } else {
            let target_trait = input.parse()?;

            let for_token = input.parse()?;

            let content;
            let bracket_token = syn::bracketed!(content in input);

            let forwarded_list = content.parse_terminated(Forwarded::parse, Token![,])?;

            Ok(Self::Splitoff(target_trait, for_token, bracket_token, forwarded_list))
        }
    }
}

impl Extract {
    /// Expand the extract macro invocation.
    ///
    /// This generates either a split-off module structure for multiple
    /// forwarded implementations, or directly expands a single forwarded
    /// implementation.
    ///
    /// # Errors
    ///
    /// This will fail if the extraction fails.
    pub fn expand(self) -> syn::Result<TokenStream> {
        match self {
            Self::Splitoff(target_trait, _for_token, _bracket_token, forwarded_list) => {
                let mut modules = Vec::new();

                for forwarded in forwarded_list {
                    let target_type = &forwarded.target_type;
                    let source_trait = &forwarded.source_trait;

                    // Generate a module name from the target type and source
                    // trait
                    let module_name = {
                        let type_name = target_type.as_str();
                        let trait_name = source_trait
                            .segments
                            .last()
                            .map_or_else(|| "trait".to_string(), |seg| seg.ident.to_string());
                        let module_name_str = format!("__extract_{type_name}_{trait_name}");
                        Ident::new(&module_name_str, Span::call_site())
                    };

                    let expanded = forwarded.expand(&target_trait)?;

                    modules.push(quote! {
                        #[doc(hidden)]
                        mod #module_name {
                            use super::*;

                            #expanded
                        }

                        #[doc(inline)]
                        pub use #module_name::*;
                    });
                }

                Ok(quote! {
                    #(#modules)*
                })
            },
            Self::Forward(_trait_token, target_trait, _for_token, forwarded) => forwarded.expand(&target_trait),
        }
    }
}

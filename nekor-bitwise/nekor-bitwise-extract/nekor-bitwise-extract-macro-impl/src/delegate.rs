//! Metadata trait delegation infrastructure.
// delegate!([become] as MetadataU16 for [u8 become MetadataU16U8, u16 become
// MetadataU16U16])
//
// O type needs sealed trait

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Ident, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

use crate::primitive::Primitive;

/// The input macro structure expected by the `metadata` proc-macro.
pub struct Delegate {
    /// A marker token (`become`) to indicate whether to include the
    /// implementation `detail` module.
    become_token: Option<Token![become]>,

    /// The input primitive to be used in this delegate.
    input_primitive: Primitive,

    /// The name of the delegate trait implementation.
    name: Ident,

    /// The list of delegated trait implementations.
    trait_list: DelegatedList,
}

impl Delegate {
    /// Expand this delegate macro input.
    ///
    /// # Errors
    ///
    /// If each delegated trait implementation is not a valid trait, this will
    /// fail.
    pub fn expand(self) -> syn::Result<TokenStream> {
        let Self {
            become_token,
            input_primitive,
            name,
            trait_list,
            ..
        } = self;

        let trait_name_string = name.to_string();
        let trait_name_slice = trait_name_string.as_str();

        let optional_impl_detail = match become_token {
            Some(..) => {
                let detail_module_doc = format!("Implementation details for the `{trait_name_slice}*`-related traits.");

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
            },
            None => None,
        };

        let metadata_trait_doc = format!("A metadata delegator item for the `{}` type", input_primitive.as_str());

        let DelegatedList(delegate_impls) = trait_list;

        let delegate_count = delegate_impls.len();

        let delegate_impls = delegate_impls
            .into_iter()
            .map(|delegate_impl| DelegateTraitImpl::expand(delegate_impl, &name));

        let mut delegate_list = Vec::with_capacity(delegate_count);

        for v in delegate_impls {
            delegate_list.push(v?);
        }

        let quoted_tokens = quote! {
            #optional_impl_detail

            #[doc = #metadata_trait_doc]
            pub trait #name<const N: usize, const M: usize, O> {
                /// The number of bits to be extracted.
                const BITSET_WIDTH: usize;

                /// The bitwise mask composed of all to-be-extracted bits.
                const BITSET_MASK: Self;

                /// The fuse mask for the extract operation.
                const EXTRACT_MASK: Self;

                /// The fuse mask for the merge operation.
                const FUSE_MASK: Self;
            }

            #(
                #delegate_list
            )*
        };

        Ok(quoted_tokens)
    }
}

impl Parse for Delegate {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let become_token = input.parse()?;

        let input_primitive = input.parse()?;

        let _: Token![as] = input.parse()?;

        let delegate_trait_name = input.parse()?;

        let _: Token![for] = input.parse()?;

        let delegate_trait_impls = input.parse()?;

        Ok(Self {
            become_token,
            input_primitive,
            name: delegate_trait_name,
            trait_list: delegate_trait_impls,
        })
    }
}

/// A single delegated trait implementation.
pub struct DelegateTraitImpl {
    /// The primitive type that this trait implementation is for.
    primitive_type: Primitive,

    /// The name of the delegate trait implementation.
    delegate_trait_name: Ident,
}

impl DelegateTraitImpl {
    /// Expand to the appropiate implementation for this delegated trait.
    ///
    /// # Errors
    ///
    /// This is infallible, but may change in the future.
    pub fn expand(self, delegator_trait: &Ident) -> syn::Result<TokenStream> {
        let Self {
            primitive_type,
            delegate_trait_name,
            ..
        } = self;

        let output_type = primitive_type.as_ident();

        let quoted_tokens = quote! {
            #[automatically_derived]
            impl detail::Sealed for #output_type {}

            #[automatically_derived]
            impl<T, const N: usize, const M: usize> #delegator_trait<N, M, #output_type> for T
            where
                T: #delegate_trait_name<N, M>,
            {
                const BITSET_WIDTH: usize = <T as #delegate_trait_name<N, M>>::BITSET_WIDTH;

                const BITSET_MASK: Self = <T as #delegate_trait_name<N, M>>::BITSET_MASK;

                const EXTRACT_MASK: Self = <T as #delegate_trait_name<N, M>>::EXTRACT_MASK;

                const FUSE_MASK: Self = <T as #delegate_trait_name<N, M>>::FUSE_MASK;
            }
        };

        Ok(quoted_tokens)
    }
}

impl Parse for DelegateTraitImpl {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let primitive_type = input.parse()?;

        let _: Token![become] = input.parse()?;

        let delegate_trait_name = input.parse()?;

        Ok(Self {
            primitive_type,
            delegate_trait_name,
        })
    }
}

/// A list of delegated traits.
#[repr(transparent)]
pub struct DelegatedList(Punctuated<DelegateTraitImpl, Token![,]>);

impl Parse for DelegatedList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let list_content;

        let _ = syn::bracketed!(list_content in input);

        Ok(Self(Punctuated::<DelegateTraitImpl, Token![,]>::parse_terminated(
            &list_content,
        )?))
    }
}

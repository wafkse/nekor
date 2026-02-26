//! implement implementation infrastructure.

use core::num::NonZero;
use core::ops::RangeInclusive;

use proc_macro2::{Span, TokenStream};

use quote::quote;

use syn::{
    Ident, LitInt, Token,
    parse::{Parse, ParseStream},
};

use crate::primitive::Primitive;

/// An iterator over the bit ranges of an input-output type 2-tuple.
pub struct BitRanges {
    /// The input type to the range.
    range_input: Primitive,

    /// The output type to the range.
    range_output: Primitive,

    /// The state start index of the iterator.
    state_start_index: usize,

    /// The state end index of the iterator.
    state_end_index: Option<NonZero<usize>>,
}

impl BitRanges {
    /// Construct a new bit-ranges [`Iterator`] for the input-output integer
    /// primitive 2-tuple.
    #[inline]
    #[must_use]
    pub const fn inout(range_input: Primitive, range_output: Primitive) -> Self {
        let state_start_index = usize::MIN;
        let state_end_index = None;

        Self {
            range_input,
            range_output,
            state_start_index,
            state_end_index,
        }
    }
}

impl Iterator for BitRanges {
    type Item = RangeInclusive<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        let &mut Self {
            ref range_input,
            ref range_output,
            ref mut state_start_index,
            ref mut state_end_index,
        } = self;

        let (input_size, output_size) = (range_input.size(), range_output.size());

        let (input_bits, output_bits) = (input_size.bits(), output_size.bits());

        // NOTE: Here we're done iterating over all possible ranges.
        if *state_start_index >= input_bits {
            return None;
        }

        let initial_bitwidth = output_size.below().map_or(2, |s| s.bits() + 1);

        if let Some(end_index_plus_one) = state_end_index {
            let end_index = end_index_plus_one.get() - 1;
            let current_width = end_index - *state_start_index + 1;

            // Try next end position (increment width)
            let new_end = end_index + 1;
            let new_width = current_width + 1;

            if new_end < input_bits && new_width <= output_bits {
                // Can extend range at current start position
                *state_end_index = NonZero::new(new_end + 1);
                Some(*state_start_index..=new_end)
            } else {
                // Move to next start position, reset to initial_bitwidth
                let new_start = *state_start_index + 1;
                let new_end = new_start + initial_bitwidth - 1;

                if new_end >= input_bits {
                    // Can't fit initial_bitwidth at this position, done
                    *state_start_index = output_bits;
                    return None;
                }

                *state_start_index = new_start;
                *state_end_index = NonZero::new(new_end + 1);
                Some(new_start..=new_end)
            }
        } else {
            // Initialize with first range: start=0, width=initial_bitwidth
            let end = initial_bitwidth - 1;

            if initial_bitwidth > output_bits || end >= input_bits {
                // initial_bitwidth doesn't fit
                *state_start_index = output_bits;
                return None;
            }

            *state_end_index = NonZero::new(end + 1);
            Some(0..=end)
        }
    }
}

/// The input macro structure expected by the `implement`  proc-macro.
#[derive(Clone)]
pub struct Implement {
    /// A marker token to indicate whether to include a `detail::Sealed` trait
    /// in the output.
    ///
    /// This permits the existence of multiple [`Implement`] expansions in the
    /// same module.
    become_token: Option<Token![become]>,

    /// The identifier indicating the input type.
    input_type: Primitive,

    /// The identifier indicating the output type.
    output_type: Primitive,

    /// The name of the trait to-be-exposed by the associated macro.
    trait_name: Ident,
}

impl Implement {
    /// Expand to the actual implement trait definition.
    ///
    /// # Errors
    ///
    /// This function returns an error if the input type or output type is not a
    /// valid primitive type.
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

        let implement_trait_doc =
            format!("An implement item about a {type_name_out}-from-{type_name_in} extraction.");

        let input_type_ident = syn::parse_str::<syn::Type>(type_name_in)?;

        let impl_list =
            BitRanges::inout(input_type, output_type)
                .map(| target_range | {
                    let (&start, &end) = (target_range.start(), target_range.end());

                    let bitset_width = end - start + 1;

                    let start_literal = LitInt::new(&start.to_string(), Span::call_site());
                    let end_literal =
                        LitInt::new(&end.to_string(), Span::call_site());

                    let bitset_width_literal = LitInt::new(
                        &bitset_width.to_string(),
                        Span::call_site(),
                    );

                    let bitset_mask_value = quote! {
                        Self::MAX >> (Self::BITS as usize - #bitset_width_literal)
                    };

                    let extract_mask_value = quote! {
                        (Self::MAX >> (Self::BITS as usize - #bitset_width_literal)) << #start_literal
                    };

                    let fuse_mask_value = quote! {
                        !((Self::MAX >> (Self::BITS as usize - #bitset_width_literal)) << #start_literal)
                    };

                    quote! {
                        #[doc = #implement_trait_doc]
                        impl #trait_name<#start_literal, #end_literal> for #input_type_ident {
                            /// The number of bits to be extracted.
                            const BITSET_WIDTH: usize = #bitset_width_literal;

                            /// The bitwise mask composed of all to-be-extracted bits.
                            const BITSET_MASK: Self = #bitset_mask_value;

                            /// The fuse mask for the extract operation.
                            const EXTRACT_MASK: Self = #extract_mask_value;

                            /// The fuse mask for the merge operation.
                            const FUSE_MASK: Self = #fuse_mask_value;
                        }
                    }
                })
                .collect::<Vec<_>>();

        let implement_trait = quote! {
            #optional_impl_detail

            impl detail::Sealed for #input_type_ident {}

            #(#impl_list)*
        };

        Ok(implement_trait)
    }
}

impl Implement {
    /// Determine the input [`Primitive`] in this implement invocation.
    #[inline]
    #[must_use]
    pub const fn input(&self) -> &Primitive {
        let &Self { ref input_type, .. } = self;

        input_type
    }

    /// Determine the output [`Primitive`] in this implement invocation.
    #[inline]
    #[must_use]
    pub const fn output(&self) -> &Primitive {
        let &Self {
            ref output_type, ..
        } = self;

        output_type
    }

    /// Determine the trait name in this implement invocation.
    #[inline]
    #[must_use]
    pub const fn name(&self) -> &Ident {
        let &Self { ref trait_name, .. } = self;

        trait_name
    }
}

impl Parse for Implement {
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

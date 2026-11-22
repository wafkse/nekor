#![forbid(
    clippy::all,
    clippy::perf,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::panic,
    clippy::pedantic,
    unsafe_code,
    rustdoc::all
)]
#![doc = include_str!("../../README.md")]

// Here we need two functions: metadata, which creates a metadata trait, and
// implement, which implements it for a given type and its output type.
// Other one would be delegate, which handles the boilerplate for taking a
// MetadataXY to a top-level MetadataX trait association.

use nekor_bitwise_extract_macro_impl::{
    delegate::Delegate, extract::Extract, implement::Implement, metadata::Metadata,
};
use proc_macro::TokenStream;

/// Declare extraction metadata for one input and output width.
///
/// `metadata!([become] for Input -> Output as Name)` declares `Name<N, M>` with
/// inclusive endpoints. `become` also creates the local sealing module. Exactly one
/// expansion in that module must create it before implementations use the seal.
///
/// # Syntax
///
/// ```text
/// metadata!([become] for input_type -> output_type as TraitName)
/// ```
///
/// # Example
///
/// ```rust
/// use nekor_bitwise_extract_macro::{metadata, implement};
///
/// metadata!(become for u16 -> u8 as ByteField);
/// implement!(for u16 -> u8 as ByteField);
///
/// assert_eq!(<u16 as ByteField<4, 7>>::BITSET_WIDTH, 4);
/// assert_eq!(<u16 as ByteField<4, 7>>::BITSET_MASK, 0x0f);
/// assert_eq!(<u16 as ByteField<4, 7>>::EXTRACT_MASK, 0x00f0);
/// assert_eq!(<u16 as ByteField<4, 7>>::FUSE_MASK, 0xff0f);
/// ```
#[proc_macro]
pub fn metadata(input: TokenStream) -> TokenStream {
    match syn::parse(input) {
        Ok::<Metadata, _>(target_value) => {
            match Metadata::expand(target_value)
                .map(TokenStream::from)
                .map_err(|error| error.to_compile_error())
            {
                Ok(tokens) => tokens,
                Err(tokens) => TokenStream::from(tokens),
            }
        },
        Err(error) => error.to_compile_error().into(),
    }
}

/// Generate the field ranges assigned to one input and output width.
///
/// ```text
/// implement!([become] for Input -> Output as MetadataTrait)
/// ```
///
/// The metadata trait must already exist. `become` creates the local sealing module.
/// Omit it when another expansion has created that module. Ranges use inclusive
/// endpoints and the output-width partition selected by the range generator.
#[proc_macro]
pub fn implement(input: TokenStream) -> TokenStream {
    match syn::parse(input) {
        Ok::<Implement, _>(target_value) => {
            match Implement::expand(target_value)
                .map(TokenStream::from)
                .map_err(|error| error.to_compile_error())
            {
                Ok(tokens) => tokens,
                Err(tokens) => TokenStream::from(tokens),
            }
        },
        Err(error) => error.to_compile_error().into(),
    }
}

/// Combine output-specific metadata traits under an output type parameter.
///
/// ```text
/// delegate!([become] Input as Name for [
///     Output become MetadataTrait,
/// ])
/// ```
///
/// Each listed metadata trait supplies the constants for its output type. The resulting
/// `Name<N, M, O>` interface selects that trait through `O`. `become` also declares the
/// local sealing module when no earlier expansion has supplied it.
#[proc_macro]
pub fn delegate(input: proc_macro::TokenStream) -> TokenStream {
    match syn::parse(input) {
        Ok::<Delegate, _>(target_value) => {
            match Delegate::expand(target_value)
                .map(TokenStream::from)
                .map_err(|error| error.to_compile_error())
            {
                Ok(tokens) => tokens,
                Err(tokens) => TokenStream::from(tokens),
            }
        },
        Err(error) => error.to_compile_error().into(),
    }
}

/// Forward extraction metadata to an existing trait.
///
/// The grouped form places each forwarding in a hidden module so separate width sets
/// can be expanded independently. The direct form emits one forwarding in the current
/// module. Source and destination traits must expose compatible metadata constants.
///
/// ```text
/// extract!(TargetTrait for [
///     type1 use SourceTrait1 become [output_types...],
///     type2 use SourceTrait2 become [output_types...],
/// ])
/// ```
///
/// ## Direct forwarding mode
///
/// ```text
/// extract!(trait TargetTrait for input_type use SourceTrait become [output_types...])
/// ```
///
/// # Example (Split-off)
///
/// ```rust,ignore
/// extract!(MyExtractTrait for [
///     u16 use MetadataU16 become [u8, u16],
///     u32 use MetadataU32 become [u8, u16, u32],
/// ])
///
/// extract!(trait TargetTrait for u16 use MetadataU16 become [u8, u16])
/// ```
#[proc_macro]
pub fn extract(input: proc_macro::TokenStream) -> TokenStream {
    match syn::parse(input) {
        Ok::<Extract, _>(target_value) => {
            match Extract::expand(target_value)
                .map(TokenStream::from)
                .map_err(|error| error.to_compile_error())
            {
                Ok(tokens) => tokens,
                Err(tokens) => TokenStream::from(tokens),
            }
        },
        Err(error) => error.to_compile_error().into(),
    }
}

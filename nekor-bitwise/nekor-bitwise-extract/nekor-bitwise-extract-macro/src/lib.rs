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

/// Generate a metadata trait for bitwise extraction operations.
///
/// This macro creates a trait that defines the metadata for extracting bits
/// from one integer type to another. The trait includes associated constants
/// for bit masks and widths.
///
/// # Syntax
///
/// ```text
/// metadata!([become] for input_type -> output_type as TraitName)
/// ```
///
/// - `become` (optional): Include a `detail::Sealed` trait in the output
/// - `input_type`: The source integer type (e.g., `u16`)
/// - `output_type`: The target integer type (e.g., `u8`)
/// - `TraitName`: The name of the generated trait
///
/// # Example
///
/// ```rust,ignore
/// metadata!(become for u16 -> u8 as MetadataU16U8);
/// ```
///
/// This generates a trait with associated constants for extraction metadata:
///
/// ```text
/// pub trait MetadataU16U8<const N: usize, const M: usize>: detail::Sealed {
///     const BITSET_WIDTH: usize;
///     const BITSET_MASK: Self;
///     const EXTRACT_MASK: Self;
///     const FUSE_MASK: Self;
/// }
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
        }
        Err(error) => error.to_compile_error().into(),
    }
}

/// Implement the metadata trait for specific bit ranges.
///
/// This macro generates implementations of a metadata trait for all valid
/// bit extraction ranges between the input and output types. It creates
/// implementations for each valid starting and ending bit position.
///
/// # Syntax
///
/// ```text
/// implement!([become] for input_type -> output_type as TraitName)
/// ```
///
/// - `become` (optional): Skip generating the `detail::Sealed` implementation
/// - `input_type`: The source integer type (e.g., `u16`)
/// - `output_type`: The target integer type (e.g., `u8`)
/// - `TraitName`: The name of the trait to implement
///
/// # Example
///
/// ```rust,ignore
/// implement!(for u16 -> u8 as MetadataU16U8);
/// ```
///
/// This generates implementations for all valid bit ranges:
///
/// ```text
/// impl MetadataU16U8<0, 1> for u16 {
///     const BITSET_WIDTH: usize = 2;
///     const BITSET_MASK: Self = Self::MAX >> (Self::BITS as usize - 2);
///     // ... other constants
/// }
/// impl MetadataU16U8<0, 2> for u16 { /* ... */ }
/// // ... more implementations for all valid ranges
/// ```
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
        }
        Err(error) => error.to_compile_error().into(),
    }
}

/// Create a delegating trait that unifies multiple metadata traits.
///
/// This macro generates a trait that delegates to specific metadata traits
/// based on the output type parameter. It's used to create a unified
/// interface for extracting to multiple output types.
///
/// # Syntax
///
/// ```text
/// delegate!([become] input_type as DelegateTrait for [
///     output_type1 become MetadataTrait1,
///     output_type2 become MetadataTrait2,
/// ])
/// ```
///
/// - `become` (optional): Include a `detail::Sealed` trait in the output
/// - `input_type`: The source integer type (e.g., `u16`)
/// - `DelegateTrait`: The name of the generated delegating trait
/// - `output_typeN`: Output types to delegate for
/// - `MetadataTraitN`: The specific metadata trait for each output type
///
/// # Example
///
/// ```rust,ignore
/// delegate!(become u16 as MetadataU16 for [
///     u8 become MetadataU16U8,
///     u16 become MetadataU16U16,
/// ]);
/// ```
///
/// This generates a trait with a generic output type parameter:
///
/// ```text
/// pub trait MetadataU16<const N: usize, const M: usize, O> {
///     const BITSET_WIDTH: usize;
///     const BITSET_MASK: Self;
///     const EXTRACT_MASK: Self;
///     const FUSE_MASK: Self;
/// }
///
/// impl<T, const N: usize, const M: usize> MetadataU16<N, M, u8> for T
/// where
///     T: MetadataU16U8<N, M>,
/// {
///     // ... delegates to MetadataU16U8
/// }
/// ```
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
        }
        Err(error) => error.to_compile_error().into(),
    }
}

/// Forward trait implementations from source traits to a target trait.
///
/// This macro creates trait implementations that forward associated constants
/// from source metadata traits to a target trait. It supports two modes:
/// split-off (multiple forwardings in separate modules) and direct forwarding.
///
/// # Syntax
///
/// ## Split-off mode
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
/// ]);
/// ```
///
/// This generates separate hidden modules for each type, implementing
/// `MyExtractTrait` by forwarding to the source metadata traits.
///
/// # Example (Direct)
///
/// ```rust,ignore
/// extract!(trait MyExtractTrait for u16 use MetadataU16 become [u8, u16]);
/// ```
///
/// This directly generates implementations without creating separate modules:
///
/// ```text
/// impl MyExtractTrait<0, 1, u8> for u16 {
///     const BITSET_WIDTH: usize = <u16 as MetadataU16<0, 1, u8>>::BITSET_WIDTH;
///     // ... other forwarded constants
/// }
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
        }
        Err(error) => error.to_compile_error().into(),
    }
}

//! Derive crate for `nekor-project`.
//!
//! This is a procedural macro crate that implements the derive macros required
//! for `nekor-project` functionality.

use proc_macro::TokenStream;

use syn::{DeriveInput, parse_macro_input};

use nekor_project_core::ProjectItem;

#[proc_macro_derive(Project, attributes(project))]
pub fn project_derive(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);

    match ProjectItem::input(derive_input)
        .map(ProjectItem::expand)
        .flatten()
    {
        Ok(target_tokens) => TokenStream::from(target_tokens),
        Err(target_error) => TokenStream::from(syn::Error::into_compile_error(target_error)),
    }
}

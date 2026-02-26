//! Integer primitive.

use core::iter;

use proc_macro2::{Span, TokenStream, TokenTree};

use quote::ToTokens;

use syn::{
    Ident,
    parse::{Parse, ParseStream},
};

/// The integer size of an integer primitive.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
pub enum IntegerSize {
    /// 8-bit integer.
    Size8,

    /// 16-bit integer.
    Size16,

    /// 32-bit integer.
    Size32,

    /// 64-bit integer.
    Size64,

    /// 128-bit integer.
    Size128,
}

impl IntegerSize {
    /// Determine the [`IntegerSize`] below this one.
    #[inline]
    #[must_use]
    pub const fn below(&self) -> Option<Self> {
        match self {
            Self::Size8 => None,
            Self::Size16 => Some(Self::Size8),
            Self::Size32 => Some(Self::Size16),
            Self::Size64 => Some(Self::Size32),
            Self::Size128 => Some(Self::Size64),
        }
    }

    /// Determine the [`IntegerSize`] above this one.
    #[inline]
    #[must_use]
    pub const fn above(&self) -> Option<Self> {
        match self {
            Self::Size8 => Some(Self::Size16),
            Self::Size16 => Some(Self::Size32),
            Self::Size32 => Some(Self::Size64),
            Self::Size64 => Some(Self::Size128),
            Self::Size128 => None,
        }
    }

    /// Determine the number of bits for this integer size.
    #[inline]
    #[must_use]
    pub const fn bits(self) -> usize {
        match self {
            Self::Size8 => 8,
            Self::Size16 => 16,
            Self::Size32 => 32,
            Self::Size64 => 64,
            Self::Size128 => 128,
        }
    }
}

/// An integer primitive type in Rust.
#[derive(Debug, Clone)]
pub enum Primitive {
    /// Unsigned 8-bit integer.
    U8(Span),

    /// Unsigned 16-bit integer.
    U16(Span),

    /// Unsigned 32-bit integer.
    U32(Span),

    /// Unsigned 64-bit integer.
    U64(Span),

    /// Unsigned 128-bit integer.
    U128(Span),

    /// Signed 8-bit integer.
    I8(Span),

    /// Signed 16-bit integer.
    I16(Span),

    /// Signed 32-bit integer.
    I32(Span),

    /// Signed 64-bit integer.
    I64(Span),

    /// Signed 128-bit integer.
    I128(Span),
}

impl Primitive {
    /// Determine the size of the integer primitive.
    ///
    /// In the case of a `usize` or `isize`, the size is determined by the
    /// target architecture and is therefore not known by the procedural macro.
    #[inline]
    #[must_use]
    pub const fn size(&self) -> IntegerSize {
        match self {
            Self::U8(..) | Self::I8(..) => IntegerSize::Size8,
            Self::U16(..) | Self::I16(..) => IntegerSize::Size16,
            Self::U32(..) | Self::I32(..) => IntegerSize::Size32,
            Self::U64(..) | Self::I64(..) => IntegerSize::Size64,
            Self::U128(..) | Self::I128(..) => IntegerSize::Size128,
        }
    }
}

impl Primitive {
    /// Determine the string identifying this integer primitive.
    #[inline]
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::U8(_) => "u8",
            Self::U16(_) => "u16",
            Self::U32(_) => "u32",
            Self::U64(_) => "u64",
            Self::U128(_) => "u128",
            Self::I8(_) => "i8",
            Self::I16(_) => "i16",
            Self::I32(_) => "i32",
            Self::I64(_) => "i64",
            Self::I128(_) => "i128",
        }
    }

    /// Construct an identifier describing this primitive.
    ///
    /// This will make a roundtrip around [`Parse`] as well.
    #[inline]
    #[must_use]
    pub fn as_ident(&self) -> Ident {
        Ident::new(self.as_str(), self.span())
    }

    /// Determine the span for this integer primitive.
    #[inline]
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::U8(span)
            | Self::U16(span)
            | Self::U32(span)
            | Self::U64(span)
            | Self::U128(span)
            | Self::I8(span)
            | Self::I16(span)
            | Self::I32(span)
            | Self::I64(span)
            | Self::I128(span) => *span,
        }
    }
}

impl Parse for Primitive {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;

        match ident.to_string().as_str() {
            "u8" => Ok(Self::U8(ident.span())),
            "u16" => Ok(Self::U16(ident.span())),
            "u32" => Ok(Self::U32(ident.span())),
            "u64" => Ok(Self::U64(ident.span())),
            "u128" => Ok(Self::U128(ident.span())),
            "i8" => Ok(Self::I8(ident.span())),
            "i16" => Ok(Self::I16(ident.span())),
            "i32" => Ok(Self::I32(ident.span())),
            "i64" => Ok(Self::I64(ident.span())),
            "i128" => Ok(Self::I128(ident.span())),
            _ => Err(syn::Error::new(
                ident.span(),
                "invalid primitive type, expected: u8, u16, u32, u64, u128, i8, i16, i32, i64, \
                 i128, usize or isize",
            )),
        }
    }
}

impl ToTokens for Primitive {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(iter::once(TokenTree::Ident(Ident::new(
            self.as_str(),
            self.span(),
        ))));
    }
}

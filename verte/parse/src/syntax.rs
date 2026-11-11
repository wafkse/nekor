//! Concrete syntax occurrences whose punctuation and original source remain available.

/// Declare a concrete node with private syntax and a parser-established extent.
macro_rules! node {
    () => {};
    (
        $(#[$meta:meta])*

        $vis:vis struct $name:ident {
            $( $(#[$field_meta:meta])* $field:ident: $ty:ty, )*
        }
        $($rest:tt)*
    ) => {
        tokel::stream! {
            $(#[$meta])*
            #[derive(Debug, Clone, PartialEq, Eq)]
            // NOTE: `span` encloses the ordered syntax fields in their original source.
            $vis struct $name {
                $( $(#[$field_meta])* $field: $ty, )*
                /// The entire nonempty syntactic occurrence.
                span: Span,
            }

            impl $name {
                $(
                    $(#[$field_meta])*
                    #[inline]
                    $vis const fn $field(&self) -> &$ty {
                        let Self { $field, .. } = self;

                        $field
                    }
                )*
            }

            impl Spanned for $name {
                #[inline]
                fn span(&self) -> Span {
                    let &Self { span, .. } = self;

                    span
                }
            }
        }
        node! { $($rest)* }
    };
}

pub mod declaration;
pub mod document;
pub mod expression;
pub mod types;

pub mod group;
pub mod list;

/// Typed syntax surrounded by concrete parentheses.
pub type Parenthesized<T> = group::Delimited<T, crate::scan::punct::LeftParen, crate::scan::punct::RightParen>;

/// Typed syntax surrounded by concrete brackets.
pub type Bracketed<T> = group::Delimited<T, crate::scan::punct::LeftBracket, crate::scan::punct::RightBracket>;

/// Typed syntax surrounded by concrete braces.
pub type Braced<T> = group::Delimited<T, crate::scan::punct::LeftBrace, crate::scan::punct::RightBrace>;

/// A comma-separated syntax sequence with an optional final separator.
///
/// Empty lists and trailing commas are accepted by default. The grammar can
/// require at least one element or prohibit a comma after the final element.
pub type Punctuated<T, P, const REQUIRED: bool = false, const TRAILING: bool = true> =
    list::Punctuated<T, P, REQUIRED, TRAILING>;

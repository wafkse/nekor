//! Interned ASCII symbolic names with source extents.

use core::fmt;

use aldebaran::{
    id::ident::TaggedId,
    interner::storage::StorageId,
    logic::connective::Or,
    prelude::{Annotated, Assert, Cursor, Lex, Report, Span, Spanned},
    report::annotated::{InlineAnnotations, label::Label},
    text::{
        builtin::{Ascii, property},
        cursor::error::Expected,
    },
};
use fack::prelude::Error;

use super::context::ScanContext;

/// Identifier component expectations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Assert)]
pub enum Fragment {
    /// Initial ASCII letter or underscore.
    #[assert(use Or::one(property::AsciiAlphabetic, Ascii::Underscore) type Or<property::AsciiAlphabetic, Ascii>)]
    Start,

    /// Subsequent ASCII letter, digit, or underscore.
    #[assert(use Or::one(property::AsciiAlphanumeric, Ascii::Underscore) type Or<property::AsciiAlphanumeric, Ascii>)]
    Rest,
}

/// A symbolic name in its constructing storage domain.
///
/// Construction takes the opaque recognition input so the interned name and
/// its source extent cannot be supplied independently.
///
/// ```compile_fail
/// use aldebaran::prelude::{Lex, Span};
/// use verte_parse::scan::{context::ScanContext, ident::Ident};
///
/// let mut context = ScanContext::new();
/// let _ = Ident::build_with_context(("name", Span::unit(42)), &mut context);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
// NOTE(invariant): `id` resolves to the exact text covered by `span`.
pub struct Ident {
    /// Identity in the constructing interning domain.
    id: TaggedId<str, StorageId>,

    /// Original source byte extent.
    span: Span,
}

impl Ident {
    /// Retrieve the identifier's storage identity.
    #[inline]
    pub const fn id(&self) -> TaggedId<str, StorageId> {
        let &Self { id, .. } = self;

        id
    }
}

impl Spanned for Ident {
    #[inline]
    fn span(&self) -> Span {
        let &Self { span, .. } = self;

        span
    }
}

/// Identifier recognition or storage failure.
#[derive(Debug, Clone, Report)]
#[report(forward)]
pub enum IdentError {
    /// A lexical component was missing.
    Expected(Expectation),

    /// A reserved identifier cannot name a symbol.
    Reserved(Reserved),

    /// The storage could not assign an identity.
    Storage(Storage),
}

impl fmt::Display for IdentError {
    #[inline]
    fn fmt(&self, writer: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expected(error) => fmt::Display::fmt(error, writer),
            Self::Reserved(error) => fmt::Display::fmt(error, writer),
            Self::Storage(error) => fmt::Display::fmt(error, writer),
        }
    }
}

impl Error for IdentError {
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Expected(error) => Some(error),
            Self::Reserved(error) => Some(error),
            Self::Storage(error) => Some(error),
        }
    }
}

/// A missing identifier component.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("expected an identifier")]
#[error(source(cause))]
#[report(title = "expected an identifier", annotations = labels)]
// NOTE(invariant): The label and cause use the same source coordinates.
pub struct Expectation {
    /// Typed expectation and observed source component.
    cause: Expected<char, Fragment>,

    /// Primary failure location followed by related source context.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

/// A keyword used as an identifier.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("reserved identifier")]
#[report(title = "reserved identifier", annotations = labels)]
// NOTE(invariant): None.
pub struct Reserved {
    /// Primary failure location followed by related source context.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

/// An identifier could not be interned.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("identifier storage exhausted")]
#[report(title = "identifier storage exhausted", annotations = labels)]
// NOTE(invariant): None.
pub struct Storage {
    /// Primary failure location followed by related source context.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

/// A recognized symbolic name and its source extent.
#[derive(Debug)]
// NOTE(invariant): `text` is the source slice covered by `span` and satisfies the identifier
// grammar.
pub struct Input<'source> {
    /// The recognized name before interning.
    text: &'source str,

    /// Absolute byte extent of the recognized name.
    span: Span,
}

impl<'source> Lex<'source, str> for Ident {
    type Context = ScanContext<'source>;
    type Error = IdentError;
    type Input<'input>
        = Input<'source>
    where
        'source: 'input;

    #[inline]
    fn pass<'input>(c0: &'input mut Cursor<'source, str>) -> Result<Self::Input<'input>, IdentError>
    where
        'source: 'input,
    {
        let mut b0 = c0.branch();

        let (_, first) = b0.expect_is_next_spanned(Fragment::Start).map_err(|cause| {
            let labels = InlineAnnotations::new(
                Label::new(
                    "a symbolic name must start with an ASCII letter or underscore",
                    Annotated::target(&cause),
                ),
                [],
            );

            IdentError::Expected(Expectation { cause, labels })
        })?;

        let span = b0
            .ignore_while_spanned(Fragment::Rest)
            .map_or(first, |last| first.superset(last));
        let text = b0.slice(span).source();
        let mut c1 = Cursor::create(text);

        if super::keyword::Keyword::pass(&mut c1).is_ok() {
            let labels = InlineAnnotations::new(
                Label::new(
                    "choose another symbolic name, or quote this name when used as a data key",
                    span,
                ),
                [],
            );

            return Err(IdentError::Reserved(Reserved { labels }));
        }

        b0.backport();

        Ok(Input { text, span })
    }

    #[inline]
    fn build_with_context<'input>(
        Input { text, span }: Self::Input<'input>,
        context: &mut Self::Context,
    ) -> Result<Self, IdentError>
    where
        'source: 'input,
    {
        let id = context.storage_mut().try_store_tagged(text).ok_or_else(|| {
            let labels = InlineAnnotations::new(Label::new("cannot assign an identity to this name", span), []);

            IdentError::Storage(Storage { labels })
        })?;

        Ok(Self { id, span })
    }

    #[inline]
    fn lex_with_context(c0: &mut Cursor<'source, str>, context: &mut Self::Context) -> Result<Self, IdentError> {
        let mut b0 = c0.branch();
        let input = Self::pass(&mut b0)?;
        let value = Self::build_with_context(input, context)?;

        b0.backport();

        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boundaries_and_reserved_words() {
        let mut context = ScanContext::new();
        let mut c0 = Cursor::create("constituent rest");
        let value = Ident::lex_with_context(&mut c0, &mut context).expect("identifier");

        assert_eq!(value.span().range(), 0..11);
        let mut c0 = Cursor::create("const");

        assert!(matches!(
            Ident::lex_with_context(&mut c0, &mut context),
            Err(IdentError::Reserved(_))
        ));

        assert_eq!(c0.index(), 0);
    }
}

impl Expectation {
    /// Retrieve the failed lexical assertion and its observed component.
    #[inline]
    pub const fn cause(&self) -> &Expected<char, Fragment> {
        let Self { cause, .. } = self;

        cause
    }
}

#[cfg(test)]
mod preservation_tests {

    use aldebaran::prelude::{Cursor, Lex};

    use crate::scan::{context::ScanContext, ident::Ident, punct};

    #[test]
    fn identifier_boundaries_and_hyphens_follow_the_new_grammar() {
        for source in [
            "PageSize",
            "page_size",
            "PAGE_SIZE",
            "type_name",
            "Include",
            "_",
            "x86_64",
        ] {
            let mut cursor = Cursor::create(source);
            let mut context = ScanContext::new();
            let value = Ident::lex_with_context(&mut cursor, &mut context).expect("identifier");

            assert_eq!(
                context.storage().try_resolve_tagged(value.id()),
                Some(source),
                "identifier interning is case sensitive and exact"
            );

            assert!(cursor.is_eof(), "identifier must consume its entire name");
        }

        let mut cursor = Cursor::create("a-b");

        let mut context = ScanContext::new();

        Ident::lex_with_context(&mut cursor, &mut context).expect("left name");
        punct::Minus::lex(&mut cursor).expect("subtraction");
        Ident::lex_with_context(&mut cursor, &mut context).expect("right name");

        assert!(cursor.is_eof(), "hyphens separate names as subtraction operators");
    }
}

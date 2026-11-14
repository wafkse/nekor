//! Quoted strings with original source text and interned decoded content.

use alloc::{borrow::Cow, string::String};
use core::fmt;

use aldebaran::{
    ice::Ice,
    id::ident::TaggedId,
    interner::storage::StorageId,
    logic::connective::{And, Not, Or},
    prelude::{Annotated, Assert, Cursor, Lex, Report, Span, Spanned},
    report::annotated::{InlineAnnotations, label::Label},
    text::{
        builtin::{Char, property},
        cursor::error::Expected,
    },
};
use fack::prelude::Error;

use super::context::ScanContext;

pub mod escape;

/// Quoted-string expectations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Assert)]
pub enum Fragment {
    /// A double quote.
    #[assert(use Char('"') type Char)]
    Quote,

    /// An unescaped string scalar.
    #[assert(
        use And::them(
            Or::one(Not::that(property::AsciiControl), Char('\u{7f}')),
            Not::that(Or::one(Char('"'), Char('\\'))),
        )
        type And<Or<Not<property::AsciiControl>, Char>, Not<Or<Char, Char>>>
    )]
    Content,

    /// An escape introducer.
    #[assert(use escape::Fragment::Start type escape::Fragment)]
    Escape,
}

/// A quoted occurrence in its constructing string storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
// NOTE(invariant): The identity resolves to the validated decoding of the quoted span.
pub struct Literal(TaggedId<str, StorageId>, Span);

impl Literal {
    /// Retrieve the decoded content identity.
    #[inline]
    pub const fn id(&self) -> TaggedId<str, StorageId> {
        let &Self(id, _) = self;

        id
    }
}

impl Spanned for Literal {
    #[inline]
    fn span(&self) -> Span {
        let &Self(_, span) = self;

        span
    }
}

/// Quoted text awaiting decoding and storage.
#[derive(Debug)]
// NOTE(invariant): `span` covers validated quoted text in `source` and `escaped` records escape
// presence.
pub struct Input<'s> {
    /// Original source text in absolute byte coordinates.
    source: &'s str,

    /// Original source byte extent.
    span: Span,

    /// Whether the quoted content requires escape decoding.
    escaped: bool,
}

/// A string delimiter, body, escape, or storage failure.
#[derive(Debug, Clone, Report)]
#[report(forward)]
pub enum StringError {
    /// The opening quote was absent.
    Expected(Expectation),

    /// The string contains an invalid escape.
    Escape(escape::EscapeError),

    /// A closing quote was not found before invalid content or EOF.
    Unterminated(Unterminated),

    /// The decoded content could not be interned.
    Storage(Storage),
}

impl fmt::Display for StringError {
    #[inline]
    fn fmt(&self, writer: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expected(error) => fmt::Display::fmt(error, writer),
            Self::Escape(error) => fmt::Display::fmt(error, writer),
            Self::Unterminated(error) => fmt::Display::fmt(error, writer),
            Self::Storage(error) => fmt::Display::fmt(error, writer),
        }
    }
}

impl Error for StringError {
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Expected(error) => Some(error),
            Self::Escape(error) => Some(error),
            Self::Unterminated(error) => Some(error),
            Self::Storage(error) => Some(error),
        }
    }
}

/// A missing string opener.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("expected a quoted string")]
#[error(source(cause))]
#[report(title = "expected a quoted string", annotations = labels)]
// NOTE(invariant): None.
pub struct Expectation {
    /// Typed expectation and observed source component.
    cause: Expected<char, Fragment>,

    /// Primary failure location followed by related source context.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

/// An invalid or unfinished string body.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("unterminated string")]
#[report(title = "unterminated string", annotations = labels)]
// NOTE(invariant): The related label identifies this string's opener.
pub struct Unterminated {
    /// Primary failure location followed by related source context.
    labels: InlineAnnotations<Label<&'static str>, 1>,
}

/// Decoded string storage failed.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("string storage exhausted")]
#[report(title = "string storage exhausted", annotations = labels)]
// NOTE(invariant): None.
pub struct Storage {
    /// Primary failure location followed by related source context.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

impl<'s> Lex<'s, str> for Literal {
    type Context = ScanContext<'s>;
    type Error = StringError;
    type Input<'i>
        = Input<'s>
    where
        's: 'i;

    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "Keep substantial recognition or construction out of line across grammar entry points."
    )]
    fn pass<'i>(c0: &'i mut Cursor<'s, str>) -> Result<Self::Input<'i>, StringError>
    where
        's: 'i,
    {
        let mut b0 = c0.branch();

        let (_, open) = b0.expect_is_next_spanned(Fragment::Quote).map_err(|cause| {
            let labels = InlineAnnotations::new(
                Label::new("expected a double-quoted string", Annotated::target(&cause)),
                [],
            );

            StringError::Expected(Expectation { cause, labels })
        })?;

        let mut escaped = false;

        loop {
            b0.ignore_while(Fragment::Content);
            if b0.optional(Fragment::Quote).is_some() {
                break;
            }
            if b0.is_next(Fragment::Escape) {
                escape::Escape::pass(&mut b0).map_err(StringError::Escape)?;
                escaped = true;
            } else {
                let index = b0.index();
                let target = b0.any_spanned().map_or(Span::unit(index), |(_, span)| span);
                let labels = InlineAnnotations::new(
                    Label::new(
                        "expected a closing quote; escape line breaks and control characters",
                        target,
                    ),
                    [Label::new("this string starts here", open)],
                );

                return Err(StringError::Unterminated(Unterminated { labels }));
            }
        }

        let span = open.superset(Span::unit(b0.index() - 1));
        let source = b0.source();

        b0.backport();

        Ok(Input { source, span, escaped })
    }

    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "Keep substantial recognition or construction out of line across grammar entry points."
    )]
    fn build_with_context<'i>(
        Input { source, span, escaped }: Self::Input<'i>,
        context: &mut Self::Context,
    ) -> Result<Self, StringError>
    where
        's: 'i,
    {
        let content = Ice::some(source.get(span.start() + 1..span.end() - 1));
        let text = if escaped {
            let prefix = Ice::some(source.get(..span.end() - 1));
            let mut c0 = Cursor::at(prefix, span.start() + 1);
            let mut decoded = String::with_capacity(content.len());

            while !c0.is_eof() {
                let start = c0.index();

                c0.ignore_while(Fragment::Content);
                decoded.push_str(Ice::some(source.get(start..c0.index())));
                if !c0.is_eof() {
                    decoded.push(escape::Escape::lex(&mut c0).map_err(StringError::Escape)?.scalar());
                }
            }
            Cow::Owned(decoded)
        } else {
            Cow::Borrowed(content)
        };
        let id = context.storage_mut().try_store_tagged(text).ok_or_else(|| {
            let labels = InlineAnnotations::new(Label::new("cannot retain this decoded string", span), []);

            StringError::Storage(Storage { labels })
        })?;

        Ok(Self(id, span))
    }

    #[inline]
    fn lex_with_context(c0: &mut Cursor<'s, str>, context: &mut Self::Context) -> Result<Self, StringError> {
        let mut b0 = c0.branch();
        let input = <Self as Lex<'s, str>>::pass(&mut b0)?;
        let value = <Self as Lex<'s, str>>::build_with_context(input, context)?;

        b0.backport();

        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use aldebaran::report::annotated::Annotations as _;

    use super::*;

    #[test]
    fn utf8_and_original_text() {
        let mut context = ScanContext::new();
        let mut c0 = Cursor::create("\"é\\u{1f408}#\"");
        let value = Literal::lex_with_context(&mut c0, &mut context).expect("string");

        assert_eq!(value.span().range(), 0..14);

        assert_eq!(context.storage().try_resolve_tagged(value.id()), Some("é🐈#"));
    }
    #[test]
    fn reports_include_openers() {
        let mut c0 = Cursor::create("\"é\n");
        let error = Literal::lex(&mut c0).expect_err("control character");

        assert_eq!(error.annotations().list().expect("annotations").rest().len(), 1);

        assert_eq!(c0.index(), 0);
    }
}

pub mod syntax;

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

    use alloc::borrow::ToOwned as _;

    use aldebaran::prelude::{Cursor, Lex, Spanned};

    use crate::scan::{context::ScanContext, string, trivia::whitespace::Whitespace};

    #[test]
    fn string_decoding_shares_identity_without_losing_source_text() {
        let source = "\"λ\" \"\\u{3bb}\" \"\\n\\r\\t\\0\\\\\\\"\" \"\"";
        let mut cursor = Cursor::create(source);
        let mut context = ScanContext::new();
        let plain =
            <string::Literal as Lex<'_, str>>::lex_with_context(&mut cursor, &mut context).expect("literal Unicode");

        Whitespace::lex(&mut cursor).expect("space");

        let escaped =
            <string::Literal as Lex<'_, str>>::lex_with_context(&mut cursor, &mut context).expect("escaped Unicode");

        assert_eq!(
            plain.id(),
            escaped.id(),
            "equal decoded strings share their interner identity"
        );

        assert_ne!(plain.span(), escaped.span(), "occurrence locations remain distinct");
        Whitespace::lex(&mut cursor).expect("space");

        let controls =
            <string::Literal as Lex<'_, str>>::lex_with_context(&mut cursor, &mut context).expect("supported escapes");

        assert_eq!(
            context.storage().try_resolve_tagged(controls.id()),
            Some("\n\r\t\0\\\""),
            "escape decoding preserves exact Unicode content"
        );
        Whitespace::lex(&mut cursor).expect("space");

        let empty =
            <string::Literal as Lex<'_, str>>::lex_with_context(&mut cursor, &mut context).expect("empty string");

        assert_eq!(
            context.storage().try_resolve_tagged(empty.id()),
            Some(""),
            "empty content remains representable without an empty Span"
        );

        assert!(cursor.is_eof(), "all quoted occurrences should be consumed");

        assert_eq!(
            context
                .storage()
                .try_resolve_tagged(plain.id())
                .expect("decoded value")
                .to_owned(),
            "λ",
            "interned values survive subsequent insertions"
        );
    }
}

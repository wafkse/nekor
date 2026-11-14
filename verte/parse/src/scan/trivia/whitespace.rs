//! Handling of whitespace inside the lexical scanner.

use aldebaran::{
    prelude::{Annotated, Assert, Cursor, Lex, Report, Span, Spanned},
    report::annotated::{InlineAnnotations, label::Label},
    text::{builtin::property, cursor::error::Expected},
};
use fack::prelude::Error;

/// The fragment for [`Whitespace`] lexical scanning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Assert)]
#[assert(use property::HorizontalWhitespace type property::HorizontalWhitespace)]
pub struct Fragment;

/// A whitespace token.
///
/// This is a contiguous run of horizontal Unicode Whitespace characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
// NOTE(invariant): The cursor run over the `Span` was composed of Unicode Whitespace
// codepoints.
pub struct Whitespace(Span);

/// Source extent established by successful recognition of [`Whitespace`].
#[derive(Debug)]
// NOTE(invariant): The span was recognized by `Whitespace::pass`.
pub struct Input(Span);

impl<'source> Lex<'source, str> for Whitespace {
    type Context = ();
    type Error = WhitespaceError;
    type Input<'input>
        = Input
    where
        'source: 'input;

    #[inline]
    fn pass<'input>(c0: &'input mut Cursor<'source, str>) -> Result<Self::Input<'input>, Self::Error>
    where
        'source: 'input,
    {
        let mut b0 = c0.branch();

        let (.., s0) = b0.expect_is_next_spanned(self::Fragment).map_err(|cause| {
            let labels =
                InlineAnnotations::new(Label::new("expected horizontal spacing", Annotated::target(&cause)), []);

            WhitespaceError { cause, labels }
        })?;

        let span = b0.ignore_while_spanned(self::Fragment).map_or(s0, |s1| s0.superset(s1));

        b0.backport();

        Ok(Input(span))
    }

    #[inline]
    fn build_with_context<'input>(Input(span): Self::Input<'input>, _: &mut Self::Context) -> Result<Self, Self::Error>
    where
        'source: 'input,
        Self: Sized,
    {
        Ok(Self(span))
    }
}

impl Spanned for Whitespace {
    #[inline]
    fn span(&self) -> Span {
        let &Self(span) = self;

        span
    }
}

/// Horizontal spacing was required here.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("expected horizontal spacing")]
#[error(source(cause))]
#[report(title = "expected horizontal spacing", annotations = labels)]
// NOTE(invariant): None.
pub struct WhitespaceError {
    /// Failed spacing assertion.
    cause: Expected<char, Fragment>,

    /// Source location of the unmet expectation.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

impl WhitespaceError {
    /// Retrieve the failed lexical assertion and its observed component.
    #[inline]
    pub const fn cause(&self) -> &Expected<char, Fragment> {
        let Self { cause, .. } = self;

        cause
    }
}

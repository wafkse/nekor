//! Horizontal spacing, comments, and physical line endings as ordinary lexemes.

use core::{error::Error, fmt};

use aldebaran::prelude::{Assert, Cursor, Lex, Report, Span, Spanned};

use super::context::ScanContext;

pub mod comment;
pub mod line_break;
pub mod whitespace;

/// Trivia recognition alternatives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Assert)]
pub enum Fragment {
    /// Horizontal spacing.
    #[assert(use whitespace::Fragment type whitespace::Fragment)]
    Space,

    /// A physical ending.
    #[assert(use line_break::Fragment::Start type line_break::Fragment)]
    Newline,

    /// A comment introducer.
    #[assert(use comment::Fragment::Hash type comment::Fragment)]
    Comment,
}

/// A source occurrence outside the value grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Trivia {
    /// Horizontal spacing.
    Whitespace(whitespace::Whitespace),

    /// A comment excluding its terminator.
    Comment(comment::Comment),

    /// A physical LF or CRLF.
    LineBreak(line_break::LineBreak),
}

impl Spanned for Trivia {
    #[inline]
    fn span(&self) -> Span {
        match self {
            Self::Whitespace(v) => v.span(),
            Self::Comment(v) => v.span(),
            Self::LineBreak(v) => v.span(),
        }
    }
}

/// Failure in the selected trivia construct.
#[derive(Debug, Clone, Report)]
#[report(forward)]
pub enum TriviaError {
    /// Spacing was absent.
    Whitespace(whitespace::WhitespaceError),

    /// Comment syntax or storage failed.
    Comment(comment::CommentError),

    /// A physical line ending was invalid.
    LineBreak(line_break::LineBreakError),
}

impl fmt::Display for TriviaError {
    #[inline]
    fn fmt(&self, writer: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Whitespace(error) => fmt::Display::fmt(error, writer),
            Self::Comment(error) => fmt::Display::fmt(error, writer),
            Self::LineBreak(error) => fmt::Display::fmt(error, writer),
        }
    }
}

impl Error for TriviaError {
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Whitespace(error) => Some(error),
            Self::Comment(error) => Some(error),
            Self::LineBreak(error) => Some(error),
        }
    }
}

/// Recognized trivia awaiting construction.
#[derive(Debug)]
pub enum Input<'s> {
    /// Horizontal source extent.
    Whitespace(whitespace::Input),

    /// Original comment text and source extent.
    Comment(comment::Input<'s>),

    /// Physical line-ending extent.
    LineBreak(line_break::Input),
}

impl<'s> Lex<'s, str> for Trivia {
    type Context = ScanContext<'s>;
    type Error = TriviaError;
    type Input<'i>
        = Input<'s>
    where
        's: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut Cursor<'s, str>) -> Result<Self::Input<'i>, TriviaError>
    where
        's: 'i,
    {
        if c0.is_next(Fragment::Space) {
            return whitespace::Whitespace::pass(c0)
                .map(Input::Whitespace)
                .map_err(TriviaError::Whitespace);
        }

        if c0.is_next(Fragment::Newline) {
            return line_break::LineBreak::pass(c0)
                .map(Input::LineBreak)
                .map_err(TriviaError::LineBreak);
        }

        comment::Comment::pass(c0)
            .map(Input::Comment)
            .map_err(TriviaError::Comment)
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, context: &mut Self::Context) -> Result<Self, TriviaError>
    where
        's: 'i,
    {
        match input {
            Input::Whitespace(span) => whitespace::Whitespace::build(span)
                .map(Self::Whitespace)
                .map_err(TriviaError::Whitespace),
            Input::LineBreak(span) => line_break::LineBreak::build(span)
                .map(Self::LineBreak)
                .map_err(TriviaError::LineBreak),
            Input::Comment(input) => comment::Comment::build_with_context(input, context)
                .map(Self::Comment)
                .map_err(TriviaError::Comment),
        }
    }

    #[inline]
    fn lex_with_context(c0: &mut Cursor<'s, str>, context: &mut Self::Context) -> Result<Self, TriviaError> {
        let mut b0 = c0.branch();
        let input = Self::pass(&mut b0)?;
        let value = Self::build_with_context(input, context)?;

        b0.backport();

        Ok(value)
    }
}

#[cfg(test)]
mod preservation_tests {

    use aldebaran::prelude::{Cursor, Lex, Spanned};

    use crate::scan::{context::ScanContext, trivia::Trivia};

    #[test]
    fn whitespace_and_line_breaks_have_disjoint_exact_extents() {
        let mut cursor = Cursor::create(" \t\u{2003}\r\n #λ\n");
        let mut context = ScanContext::new();
        let first = Trivia::lex_with_context(&mut cursor, &mut context).expect("horizontal run");

        assert!(
            matches!(first, Trivia::Whitespace(_)),
            "Unicode Zs belongs to horizontal whitespace"
        );

        assert_eq!(first.span().range(), 0..5, "Unicode spans use byte coordinates");
        let second = Trivia::lex_with_context(&mut cursor, &mut context).expect("CRLF");

        assert!(matches!(second, Trivia::LineBreak(_)), "CRLF remains a distinct object");

        assert_eq!(second.span().range(), 5..7, "CRLF is indivisible");
        let third = Trivia::lex_with_context(&mut cursor, &mut context).expect("space");

        assert_eq!(third.span().range(), 7..8, "space after a break starts a new run");
        let fourth = Trivia::lex_with_context(&mut cursor, &mut context).expect("comment");

        assert_eq!(fourth.span().range(), 8..11, "comment excludes its line ending");
        let fifth = Trivia::lex_with_context(&mut cursor, &mut context).expect("LF");

        assert_eq!(fifth.span().range(), 11..12, "LF is source backed");

        assert!(cursor.is_eof(), "all original source must be consumed");
    }
}

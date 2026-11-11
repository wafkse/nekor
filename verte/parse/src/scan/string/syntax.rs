//! Selection of an already recognized string from a syntax stream.

use core::fmt;

use aldebaran::{
    ice::Ice,
    prelude::{Annotated, Assert, Lex, Report},
    report::annotated::{InlineAnnotations, label::Label},
    text::cursor::error::Expected,
};
use fack::prelude::Error;

use crate::scan::tree::{self, SyntaxCursor, TokenTree, source::Tokens};

/// The quoted occurrence expected by a string-valued grammar production.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Assert)]
#[assert(input(TokenTree))]
pub enum Fragment {
    /// A quoted string literal.
    #[assert(matches(TokenTree::String(_)) as "a quoted string")]
    String,
}

/// A quoted occurrence was required or the source could not be lexed.
#[derive(Debug, Clone, Report)]
#[report(forward)]
pub enum StringSelectionError {
    /// A quoted string was absent.
    Expected(Expectation),

    /// Source lexing failed.
    Source(tree::TokenError),
}

impl fmt::Display for StringSelectionError {
    #[inline]
    fn fmt(&self, writer: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expected(error) => fmt::Display::fmt(error, writer),
            Self::Source(error) => fmt::Display::fmt(error, writer),
        }
    }
}

impl Error for StringSelectionError {
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Expected(error) => Some(error),
            Self::Source(error) => Some(error),
        }
    }
}

/// A missing quoted string in a syntax sequence.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("expected a quoted string")]
#[error(source(cause))]
#[report(title = "expected a quoted string", annotations = labels)]
// NOTE(invariant): None.
pub struct Expectation {
    /// Grammar expectation and the token or boundary found at failure.
    cause: Expected<TokenTree, Fragment>,

    /// Primary failure location followed by related source context.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

impl Expectation {
    /// Retrieve the syntax fragment expected at the failure location.
    #[inline]
    pub const fn cause(&self) -> &Expected<TokenTree, Fragment> {
        let Self { cause, .. } = self;

        cause
    }
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for super::Literal {
    type Context = ();
    type Error = StringSelectionError;
    type Input<'i>
        = TokenTree
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<TokenTree, StringSelectionError>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();
        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::String))
        }
        .map_err(StringSelectionError::Source)?
        .map_err(|cause| {
            let labels = InlineAnnotations::new(
                Label::new(
                    "enumeration alternatives must be quoted strings",
                    Annotated::target(&cause),
                ),
                [],
            );

            StringSelectionError::Expected(Expectation { cause, labels })
        })?;

        b0.backport();

        Ok(token)
    }

    #[inline]
    fn build_with_context<'i>(token: TokenTree, _: &mut ()) -> Result<Self, StringSelectionError>
    where
        't: 'i,
    {
        match token {
            TokenTree::String(value) => Ok(value),
            _ => Ice::impossible(None),
        }
    }
}

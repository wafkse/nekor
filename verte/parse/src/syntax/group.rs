//! Typed syntax bounded by an independently recognized delimiter group.

use alloc::boxed::Box;
use core::fmt;

use aldebaran::{
    ice::Ice,
    prelude::{Cursor, Lex, Report, Span, Spanned},
    report::annotated::{InlineAnnotations, label::Label},
    text::cursor::error::Expected,
};
use fack::prelude::Error;

use crate::scan::{
    punct,
    tree::{
        self, Stream, SyntaxCursor, TokenTree,
        group::{Delimiter, syntax::Fragment},
        source::Tokens,
    },
};

/// Typed content enclosed by matching delimiter occurrences.
#[derive(Debug, Clone, PartialEq, Eq)]
// NOTE(invariant): `open` and `close` match and bound `value` in the same source.
pub struct Delimited<T, L, R> {
    /// Opening delimiter occurrence.
    open: L,

    /// Enclosed syntax.
    value: T,

    /// Closing delimiter occurrence.
    close: R,
}

impl<T, L, R> Delimited<T, L, R> {
    /// Separate the matching delimiters from their content.
    #[inline]
    pub fn parts(self) -> (L, T, R) {
        let Self { open, value, close } = self;

        (open, value, close)
    }

    /// Retrieve the opening delimiter.
    #[inline]
    pub const fn open(&self) -> &L {
        let Self { open, .. } = self;

        open
    }

    /// Retrieve the enclosed syntax.
    #[inline]
    pub const fn value(&self) -> &T {
        let Self { value, .. } = self;

        value
    }

    /// Retrieve the closing delimiter.
    #[inline]
    pub const fn close(&self) -> &R {
        let Self { close, .. } = self;

        close
    }
}

impl<T, L: Spanned, R: Spanned> Spanned for Delimited<T, L, R> {
    #[inline]
    fn span(&self) -> Span {
        let Self { open, close, .. } = self;

        open.span().superset(close.span())
    }
}

/// Failures while entering or exhausting a typed group.
#[derive(Debug, Clone, Report)]
#[report(forward)]
pub enum GroupError<E> {
    /// The required delimiter kind was absent.
    Expected(Expectation),

    /// Enclosed syntax failed.
    Content(Box<E>),

    /// Lexical input failed.
    Source(tree::TokenError),

    /// Unconsumed syntax remains before the closing delimiter.
    Trailing(Trailing),
}

impl<E> fmt::Display for GroupError<E>
where
    E: fmt::Display,
{
    #[inline]
    fn fmt(&self, writer: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expected(error) => fmt::Display::fmt(error, writer),
            Self::Content(error) => fmt::Display::fmt(error, writer),
            Self::Source(error) => fmt::Display::fmt(error, writer),
            Self::Trailing(error) => fmt::Display::fmt(error, writer),
        }
    }
}

impl<E> Error for GroupError<E>
where
    E: Error + 'static,
{
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Expected(error) => Some(error),
            Self::Content(error) => Some(error.as_ref()),
            Self::Source(error) => Some(error),
            Self::Trailing(error) => Some(error),
        }
    }
}

/// A group with the required delimiter kind was expected.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("expected a delimiter group")]
#[error(source(cause))]
#[report(title = "expected a delimiter group", annotations = labels)]
// NOTE(invariant): None.
pub struct Expectation {
    /// Required delimiter kind and the token or boundary that was found.
    cause: Expected<TokenTree, Fragment>,

    /// Primary failure location followed by related source context.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

impl Expectation {
    /// Retrieve the delimiter expectation without discarding its observed token.
    #[inline]
    pub const fn cause(&self) -> &Expected<TokenTree, Fragment> {
        let Self { cause, .. } = self;

        cause
    }
}

/// A typed production ended before its enclosing group did.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("unexpected syntax inside group")]
#[report(title = "unexpected syntax inside group", annotations = labels)]
// NOTE(invariant): The related label identifies the group owning the remaining input.
pub struct Trailing {
    /// Primary failure location followed by related source context.
    labels: InlineAnnotations<Label<&'static str>, 1>,
}

/// Implement the bounded lexical contract for concrete delimiter pairs.
macro_rules! group {
    () => {};
    (($left:ident, $right:ident) => $variant:ident = $description:literal) => {
        tokel::stream! {
            impl<'s, 't, T, E> Lex<'t, &'t Tokens<'s>> for Delimited<T, punct::$left, punct::$right>
            where
                T: for<'a> Lex<'a, &'a Tokens<'s>, Context = (), Error = E>,
            {
                type Context = ();
                type Error = GroupError<E>;
                type Input<'i>
                    = (punct::$left, T, punct::$right)
                where
                    't: 'i;

                #[inline]
                fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, Self::Error>
                where
                    't: 'i,
                {
                    let mut b0 = c0.branch();
                    let target = {
                        let trivia = b0.source().trivia();

                        b0.ignore_while(trivia);

                        b0.source()
                            .checked(b0.peek().map_or(Span::unit(b0.index()), |token| token.span()))
                    }
                    .map_err(GroupError::Source)?;

                    let target = match {
                        let trivia = b0.source().trivia();

                        b0.ignore_while(trivia);

                        b0.source().checked(b0.peek())
                    }
                    .map_err(GroupError::Source)?
                    {
                        Some(TokenTree::Group(group)) => group.delimiter().open(),
                        _ => target,
                    };
                    let token = {
                        let trivia = b0.source().trivia();

                        b0.ignore_while(trivia);

                        b0.source().checked(b0.expect_is_next(Fragment::$variant))
                    }
                    .map_err(GroupError::Source)?
                    .map_err(|cause| {
                        let labels = InlineAnnotations::new(Label::new($description, target), []);

                        GroupError::Expected(Expectation { cause, labels })
                    })?;

                    let TokenTree::Group(group) = token else {
                        Ice::impossible(None)
                    };

                    let Delimiter::$variant(open, close) = group.delimiter() else {
                        Ice::impossible(None)
                    };

                    let (start, end) = group.content();

                    let stream = b0.source().stream();
                    let source = stream.source();
                    let context = stream.context().clone();
                    let source = Ice::some(source.get(..end));
                    let stream = Stream::new(Cursor::at(source, start), context, Some(group));

                    let tokens = Tokens::new(stream);
                    let tokens_ref = &tokens;
                    let mut c1 = Cursor::at(&tokens_ref, tokens.stream().position());
                    let value = T::lex(&mut c1).map_err(|error| GroupError::Content(Box::new(error)))?;

                    if let Some(token) = {
                        let trivia = c1.source().trivia();

                        c1.ignore_while(trivia);

                        c1.source().checked(c1.peek())
                    }
                    .map_err(GroupError::Source)?
                    {
                        let labels = InlineAnnotations::new(
                            Label::new(
                                "expected the closing delimiter after this production",
                                token.span(),
                            ),
                            [Label::new("this group starts here", open.span())],
                        );

                        return Err(GroupError::Trailing(Trailing { labels }));
                    }

                    b0.backport();

                    Ok((open, value, close))
                }

                #[inline]
                fn build_with_context<'i>((open, value, close): Self::Input<'i>, _: &mut ()) -> Result<Self, Self::Error>
                where
                    't: 'i,
                {
                    Ok(Self { open, value, close })
                }
            }
        }
    };
}
group!((LeftParen, RightParen) => Parenthesis = "expected a parenthesized production");
group!((LeftBracket, RightBracket) => Bracket = "expected a bracketed production");
group!((LeftBrace, RightBrace) => Brace = "expected a braced production");

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;

    use aldebaran::prelude::{Annotated, Cursor};

    use super::*;
    use crate::{
        scan::{context::Shared, tree::source::Tokens},
        syntax::{Braced, Bracketed, Parenthesized, expression::Expr},
    };

    #[test]
    fn wrong_delimiter_kinds_retain_the_expectation_and_cursor_position() {
        for (source, required) in [
            ("[]", Fragment::Parenthesis),
            ("{}", Fragment::Bracket),
            ("()", Fragment::Brace),
        ] {
            let stream = Stream::new(Cursor::create(source), Shared::new(), None);

            let tokens = Tokens::new(stream);
            let tokens_ref = &tokens;
            let mut c0 = Cursor::at(&tokens_ref, tokens.stream().position());
            let result = match required {
                Fragment::Parenthesis => Parenthesized::<Box<Expr>>::lex(&mut c0).map(|_| ()),
                Fragment::Bracket => Bracketed::<Box<Expr>>::lex(&mut c0).map(|_| ()),
                Fragment::Brace => Braced::<Box<Expr>>::lex(&mut c0).map(|_| ()),
            };

            let GroupError::Expected(error) = result.expect_err("wrong delimiter kind") else {
                unreachable!("the delimiter expectation must fail before the group content")
            };

            assert_eq!(c0.index(), 0);

            assert_eq!(error.annotations().primary().target().range(), 0..1);

            assert_eq!(Annotated::target(error.cause()).range(), 0..source.len());

            assert!(matches!(
                {
                    let trivia = c0.source().trivia();

                    c0.ignore_while(trivia);

                    c0.source().checked(c0.peek())
                },
                Ok(Some(TokenTree::Group(_)))
            ));
        }
    }
}

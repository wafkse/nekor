//! Separated syntax sequences with grammar-defined cardinality and trailing separators.

use alloc::{boxed::Box, vec::Vec};
use core::fmt;

use aldebaran::{
    ice::Ice,
    prelude::{Assert, Lex, Report, Span, Spanned},
    report::annotated::{InlineAnnotations, label::Label},
};
use fack::prelude::Error;

use crate::scan::{
    punct,
    tree::{self, SyntaxCursor, TokenTree, source::Tokens},
};

/// List separator expectations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Assert)]
#[assert(input(TokenTree))]
pub enum Fragment {
    /// A comma between elements.
    #[assert(matches(TokenTree::Punctuation(punct::Punctuation::Comma(_))) as "a comma")]
    Comma,
}

/// A separated sequence containing each concrete separator.
///
/// The grammar controls whether the sequence may be empty and whether a final
/// separator is allowed. Both policies apply within the enclosing group.
///
/// Recognition currently supports comma separators.
#[derive(Debug, Clone, PartialEq, Eq)]
// NOTE(invariant): Separator count is the element count or that count minus one; empty lists have
// neither.
pub struct Punctuated<T, P, const REQUIRED: bool = false, const TRAILING: bool = true> {
    /// Elements in source order.
    items: Vec<T>,

    /// Separators following the corresponding elements.
    separators: Vec<P>,
}

impl<T, P, const REQUIRED: bool, const TRAILING: bool> Punctuated<T, P, REQUIRED, TRAILING> {
    /// Retrieve elements in source order.
    #[inline]
    pub fn items(&self) -> &[T] {
        let Self { items, .. } = self;

        items
    }

    /// Retrieve separators, including a trailing separator.
    #[inline]
    pub fn separators(&self) -> &[P] {
        let Self { separators, .. } = self;

        separators
    }
}

/// Failure in a separated sequence.
#[derive(Debug, Clone, Report)]
#[report(forward)]
pub enum ListError<E> {
    /// A separator, element, or list cardinality was invalid.
    Shape(Shape),

    /// An element failed to parse.
    Element(Box<E>),

    /// A source lexical failure occurred.
    Source(tree::TokenError),
}

impl<E> fmt::Display for ListError<E>
where
    E: fmt::Display,
{
    #[inline]
    fn fmt(&self, writer: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shape(error) => fmt::Display::fmt(error, writer),
            Self::Element(error) => fmt::Display::fmt(error, writer),
            Self::Source(error) => fmt::Display::fmt(error, writer),
        }
    }
}

impl<E> Error for ListError<E>
where
    E: Error + 'static,
{
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Shape(error) => Some(error),
            Self::Element(error) => Some(error.as_ref()),
            Self::Source(error) => Some(error),
        }
    }
}

/// An unmet list shape requirement.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("invalid separated list")]
#[report(title = "invalid separated list", annotations = labels)]
// NOTE(invariant): Related context locates the preceding element or separator.
pub struct Shape {
    /// Primary failure location followed by related source context.
    labels: InlineAnnotations<Label<&'static str>, 1>,
}

impl<'s, 't, T, E, const REQUIRED: bool, const TRAILING: bool> Lex<'t, &'t Tokens<'s>>
    for Punctuated<T, punct::Comma, REQUIRED, TRAILING>
where
    T: for<'a> Lex<'a, &'a Tokens<'s>, Context = (), Error = E> + Spanned,
{
    type Context = ();
    type Error = ListError<E>;
    type Input<'i>
        = (Vec<T>, Vec<punct::Comma>)
    where
        't: 'i;

    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "Keep substantial recognition or construction out of line across grammar entry points."
    )]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, Self::Error>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();
        let mut items = Vec::new();
        let mut separators = Vec::new();

        if {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.peek())
        }
        .map_err(ListError::Source)?
        .is_none()
        {
            if REQUIRED {
                let target = {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source()
                        .checked(b0.peek().map_or(Span::unit(b0.index()), |token| token.span()))
                }
                .map_err(ListError::Source)?;
                let labels = InlineAnnotations::new(
                    Label::new("this list requires at least one element", target),
                    [Label::new(
                        "this list group starts here",
                        b0.source()
                            .stream()
                            .scope()
                            .map_or(target, |group| group.delimiter().open()),
                    )],
                );

                return Err(ListError::Shape(Shape { labels }));
            }
        } else {
            loop {
                let element = T::lex(&mut b0).map_err(|error| ListError::Element(Box::new(error)))?;
                let previous = element.span();

                items.push(element);

                match {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source().checked(b0.peek())
                }
                .map_err(ListError::Source)?
                {
                    None => break,
                    Some(token) if Fragment::Comma.assert(token) => {
                        let Some(TokenTree::Punctuation(punct::Punctuation::Comma(comma))) = {
                            let trivia = b0.source().trivia();

                            b0.ignore_while(trivia);

                            b0.source().checked(b0.any())
                        }
                        .map_err(ListError::Source)?
                        else {
                            Ice::impossible(None)
                        };

                        separators.push(comma);
                        if {
                            let trivia = b0.source().trivia();

                            b0.ignore_while(trivia);

                            b0.source().checked(b0.peek())
                        }
                        .map_err(ListError::Source)?
                        .is_none()
                        {
                            if TRAILING {
                                break;
                            }

                            let target = {
                                let trivia = b0.source().trivia();

                                b0.ignore_while(trivia);

                                b0.source()
                                    .checked(b0.peek().map_or(Span::unit(b0.index()), |token| token.span()))
                            }
                            .map_err(ListError::Source)?;
                            let labels = InlineAnnotations::new(
                                Label::new("this list does not permit a trailing comma", comma.span()),
                                [Label::new("expected another element before this boundary", target)],
                            );

                            return Err(ListError::Shape(Shape { labels }));
                        }
                    },
                    Some(token) => {
                        let labels = InlineAnnotations::new(
                            Label::new("expected `,` or the closing delimiter", token.span()),
                            [Label::new("this list element is already complete", previous)],
                        );

                        return Err(ListError::Shape(Shape { labels }));
                    },
                }
            }
        }

        b0.backport();

        Ok((items, separators))
    }

    #[inline]
    fn build_with_context<'i>((items, separators): Self::Input<'i>, _: &mut ()) -> Result<Self, Self::Error>
    where
        't: 'i,
    {
        Ok(Self { items, separators })
    }
}

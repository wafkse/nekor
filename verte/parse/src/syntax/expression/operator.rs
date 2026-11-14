//! Concrete operator families with grammar-defined binding strength.

use core::fmt;

use aldebaran::{
    ice::Ice,
    prelude::{Assert, Lex, Report, Span, Spanned},
    report::annotated::{InlineAnnotations, label::Label},
    text::cursor::error::Expected,
};
use fack::prelude::Error;

use crate::scan::{
    punct,
    tree::{self, SyntaxCursor, TokenTree, source::Tokens},
};

/// Token expectations for the operator families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Assert)]
#[assert(input(TokenTree))]
pub enum Fragment {
    /// A binary operator.
    #[assert(
        matches(
            TokenTree::Punctuation(
                punct::Punctuation::PipePipe(_)
                | punct::Punctuation::AmpAmp(_)
                | punct::Punctuation::Pipe(_)
                | punct::Punctuation::Caret(_)
                | punct::Punctuation::Amp(_)
                | punct::Punctuation::EqEq(_)
                | punct::Punctuation::BangEq(_)
                | punct::Punctuation::Lt(_)
                | punct::Punctuation::LtEq(_)
                | punct::Punctuation::Gt(_)
                | punct::Punctuation::GtEq(_)
                | punct::Punctuation::DotDot(_)
                | punct::Punctuation::DotDotEq(_)
                | punct::Punctuation::DotDotPlus(_)
                | punct::Punctuation::At(_)
                | punct::Punctuation::LtLt(_)
                | punct::Punctuation::GtGt(_)
                | punct::Punctuation::Plus(_)
                | punct::Punctuation::Minus(_)
                | punct::Punctuation::Star(_)
                | punct::Punctuation::Slash(_)
                | punct::Punctuation::Percent(_),
            )
        ) as "a binary operator"
    )]
    Binary,

    /// A unary operator.
    #[assert(
        matches(
            TokenTree::Punctuation(
                punct::Punctuation::Plus(_)
                | punct::Punctuation::Minus(_)
                | punct::Punctuation::Bang(_)
                | punct::Punctuation::Tilde(_),
            )
        ) as "a unary operator"
    )]
    Unary,

    /// A predicate operator.
    #[assert(
        matches(
            TokenTree::Punctuation(
                punct::Punctuation::EqEq(_)
                | punct::Punctuation::BangEq(_)
                | punct::Punctuation::Lt(_)
                | punct::Punctuation::LtEq(_)
                | punct::Punctuation::Gt(_)
                | punct::Punctuation::GtEq(_)
                | punct::Punctuation::Percent(_),
            )
        ) as "a predicate operator"
    )]
    Predicate,
}

/// The selected operator family was absent.
#[derive(Debug, Clone, Report)]
#[report(forward)]
pub enum OperatorError {
    /// Unmet operator expectation.
    Expected(Expectation),

    /// Source lexing failed.
    Source(tree::TokenError),
}

impl fmt::Display for OperatorError {
    #[inline]
    fn fmt(&self, writer: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expected(error) => fmt::Display::fmt(error, writer),
            Self::Source(error) => fmt::Display::fmt(error, writer),
        }
    }
}

impl Error for OperatorError {
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Expected(error) => Some(error),
            Self::Source(error) => Some(error),
        }
    }
}

/// An operator was required at this location.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("expected an operator")]
#[error(source(cause))]
#[report(title = "expected an operator", annotations = labels)]
// NOTE(invariant): None.
pub struct Expectation {
    /// Required operator family and observed occurrence.
    cause: Expected<TokenTree, Fragment>,

    /// Primary failure location followed by related source context.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

/// The concrete binary operator alternatives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binary {
    /// Logical disjunction, spelled `||`.
    Or(punct::PipePipe),

    /// Logical conjunction, spelled `&&`.
    And(punct::AmpAmp),

    /// Bitwise or predicate union, spelled `|`.
    Union(punct::Pipe),

    /// Bitwise exclusive or, spelled `^`.
    Xor(punct::Caret),

    /// Bitwise or predicate intersection, spelled `&`.
    Intersection(punct::Amp),

    /// Equality comparison, spelled `==`.
    Equal(punct::EqEq),

    /// Inequality comparison, spelled `!=`.
    Unequal(punct::BangEq),

    /// Strictly less than, spelled `<`.
    Less(punct::Lt),

    /// Less than or equal to, spelled `<=`.
    LessEqual(punct::LtEq),

    /// Strictly greater than, spelled `>`.
    Greater(punct::Gt),

    /// Greater than or equal to, spelled `>=`.
    GreaterEqual(punct::GtEq),

    /// An interval with an exclusive upper bound, spelled `..`.
    Exclusive(punct::DotDot),

    /// An interval with an inclusive upper bound, spelled `..=`.
    Inclusive(punct::DotDotEq),

    /// An interval specified by its start and extent, spelled `..+`.
    Extent(punct::DotDotPlus),

    /// Rule application, spelled `@`.
    Apply(punct::At),

    /// Left shift, spelled `<<`.
    ShiftLeft(punct::LtLt),

    /// Right shift, spelled `>>`.
    ShiftRight(punct::GtGt),

    /// Addition, spelled `+`.
    Add(punct::Plus),

    /// Subtraction, spelled `-`.
    Subtract(punct::Minus),

    /// Multiplication, spelled `*`.
    Multiply(punct::Star),

    /// Division, spelled `/`.
    Divide(punct::Slash),

    /// Remainder, spelled `%`.
    Remainder(punct::Percent),
}

impl Binary {
    /// Retrieve the grammar-defined binding strength.
    #[inline]
    pub const fn binding(&self) -> u8 {
        match self {
            Self::Or(_) => 1,
            Self::And(_) => 2,
            Self::Union(_) => 3,
            Self::Xor(_) => 4,
            Self::Intersection(_) => 5,
            Self::Equal(_) => 6,
            Self::Unequal(_) => 6,
            Self::Less(_) => 6,
            Self::LessEqual(_) => 6,
            Self::Greater(_) => 6,
            Self::GreaterEqual(_) => 6,
            Self::Exclusive(_) => 7,
            Self::Inclusive(_) => 7,
            Self::Extent(_) => 7,
            Self::Apply(_) => 8,
            Self::ShiftLeft(_) => 9,
            Self::ShiftRight(_) => 9,
            Self::Add(_) => 10,
            Self::Subtract(_) => 10,
            Self::Multiply(_) => 11,
            Self::Divide(_) => 11,
            Self::Remainder(_) => 11,
        }
    }
}

impl Spanned for Binary {
    #[inline]
    fn span(&self) -> Span {
        match self {
            Self::Or(value) => value.span(),
            Self::And(value) => value.span(),
            Self::Union(value) => value.span(),
            Self::Xor(value) => value.span(),
            Self::Intersection(value) => value.span(),
            Self::Equal(value) => value.span(),
            Self::Unequal(value) => value.span(),
            Self::Less(value) => value.span(),
            Self::LessEqual(value) => value.span(),
            Self::Greater(value) => value.span(),
            Self::GreaterEqual(value) => value.span(),
            Self::Exclusive(value) => value.span(),
            Self::Inclusive(value) => value.span(),
            Self::Extent(value) => value.span(),
            Self::Apply(value) => value.span(),
            Self::ShiftLeft(value) => value.span(),
            Self::ShiftRight(value) => value.span(),
            Self::Add(value) => value.span(),
            Self::Subtract(value) => value.span(),
            Self::Multiply(value) => value.span(),
            Self::Divide(value) => value.span(),
            Self::Remainder(value) => value.span(),
        }
    }
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Binary {
    type Context = ();
    type Error = OperatorError;
    type Input<'i>
        = punct::Punctuation
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, OperatorError>
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
        .map_err(OperatorError::Source)?;
        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::Binary))
        }
        .map_err(OperatorError::Source)?
        .map_err(|cause| {
            let labels = InlineAnnotations::new(Label::new("expected a binary operator", target), []);

            OperatorError::Expected(Expectation { cause, labels })
        })?;

        let TokenTree::Punctuation(value) = token else {
            Ice::impossible(None)
        };

        b0.backport();

        Ok(value)
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, OperatorError>
    where
        't: 'i,
    {
        Ok(match input {
            punct::Punctuation::PipePipe(value) => Self::Or(value),
            punct::Punctuation::AmpAmp(value) => Self::And(value),
            punct::Punctuation::Pipe(value) => Self::Union(value),
            punct::Punctuation::Caret(value) => Self::Xor(value),
            punct::Punctuation::Amp(value) => Self::Intersection(value),
            punct::Punctuation::EqEq(value) => Self::Equal(value),
            punct::Punctuation::BangEq(value) => Self::Unequal(value),
            punct::Punctuation::Lt(value) => Self::Less(value),
            punct::Punctuation::LtEq(value) => Self::LessEqual(value),
            punct::Punctuation::Gt(value) => Self::Greater(value),
            punct::Punctuation::GtEq(value) => Self::GreaterEqual(value),
            punct::Punctuation::DotDot(value) => Self::Exclusive(value),
            punct::Punctuation::DotDotEq(value) => Self::Inclusive(value),
            punct::Punctuation::DotDotPlus(value) => Self::Extent(value),
            punct::Punctuation::At(value) => Self::Apply(value),
            punct::Punctuation::LtLt(value) => Self::ShiftLeft(value),
            punct::Punctuation::GtGt(value) => Self::ShiftRight(value),
            punct::Punctuation::Plus(value) => Self::Add(value),
            punct::Punctuation::Minus(value) => Self::Subtract(value),
            punct::Punctuation::Star(value) => Self::Multiply(value),
            punct::Punctuation::Slash(value) => Self::Divide(value),
            punct::Punctuation::Percent(value) => Self::Remainder(value),
            _ => Ice::impossible(None),
        })
    }
}

/// The concrete unary operator alternatives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unary {
    /// A positive prefix, spelled `+`.
    Positive(punct::Plus),

    /// A negative prefix, spelled `-`.
    Negative(punct::Minus),

    /// Logical negation, spelled `!`.
    Not(punct::Bang),

    /// Bitwise complement, spelled `~`.
    Complement(punct::Tilde),
}

impl Unary {
    /// Retrieve the grammar-defined binding strength.
    #[inline]
    pub const fn binding(&self) -> u8 {
        match self {
            Self::Positive(_) => 0,
            Self::Negative(_) => 0,
            Self::Not(_) => 0,
            Self::Complement(_) => 0,
        }
    }
}

impl Spanned for Unary {
    #[inline]
    fn span(&self) -> Span {
        match self {
            Self::Positive(value) => value.span(),
            Self::Negative(value) => value.span(),
            Self::Not(value) => value.span(),
            Self::Complement(value) => value.span(),
        }
    }
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Unary {
    type Context = ();
    type Error = OperatorError;
    type Input<'i>
        = punct::Punctuation
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, OperatorError>
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
        .map_err(OperatorError::Source)?;
        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::Unary))
        }
        .map_err(OperatorError::Source)?
        .map_err(|cause| {
            let labels = InlineAnnotations::new(Label::new("expected a unary operator", target), []);

            OperatorError::Expected(Expectation { cause, labels })
        })?;

        let TokenTree::Punctuation(value) = token else {
            Ice::impossible(None)
        };

        b0.backport();

        Ok(value)
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, OperatorError>
    where
        't: 'i,
    {
        Ok(match input {
            punct::Punctuation::Plus(value) => Self::Positive(value),
            punct::Punctuation::Minus(value) => Self::Negative(value),
            punct::Punctuation::Bang(value) => Self::Not(value),
            punct::Punctuation::Tilde(value) => Self::Complement(value),
            _ => Ice::impossible(None),
        })
    }
}

/// The concrete predicate operator alternatives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Predicate {
    /// Equality comparison, spelled `==`.
    Equal(punct::EqEq),

    /// Inequality comparison, spelled `!=`.
    Unequal(punct::BangEq),

    /// Strictly less than, spelled `<`.
    Less(punct::Lt),

    /// Less than or equal to, spelled `<=`.
    LessEqual(punct::LtEq),

    /// Strictly greater than, spelled `>`.
    Greater(punct::Gt),

    /// Greater than or equal to, spelled `>=`.
    GreaterEqual(punct::GtEq),

    /// Divisibility by the following bound, spelled `%`.
    Divisible(punct::Percent),
}

impl Predicate {
    /// Retrieve the grammar-defined binding strength.
    #[inline]
    pub const fn binding(&self) -> u8 {
        match self {
            Self::Equal(_) => 0,
            Self::Unequal(_) => 0,
            Self::Less(_) => 0,
            Self::LessEqual(_) => 0,
            Self::Greater(_) => 0,
            Self::GreaterEqual(_) => 0,
            Self::Divisible(_) => 0,
        }
    }
}

impl Spanned for Predicate {
    #[inline]
    fn span(&self) -> Span {
        match self {
            Self::Equal(value) => value.span(),
            Self::Unequal(value) => value.span(),
            Self::Less(value) => value.span(),
            Self::LessEqual(value) => value.span(),
            Self::Greater(value) => value.span(),
            Self::GreaterEqual(value) => value.span(),
            Self::Divisible(value) => value.span(),
        }
    }
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Predicate {
    type Context = ();
    type Error = OperatorError;
    type Input<'i>
        = punct::Punctuation
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, OperatorError>
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
        .map_err(OperatorError::Source)?;
        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::Predicate))
        }
        .map_err(OperatorError::Source)?
        .map_err(|cause| {
            let labels = InlineAnnotations::new(Label::new("expected a predicate operator", target), []);

            OperatorError::Expected(Expectation { cause, labels })
        })?;

        let TokenTree::Punctuation(value) = token else {
            Ice::impossible(None)
        };

        b0.backport();

        Ok(value)
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, OperatorError>
    where
        't: 'i,
    {
        Ok(match input {
            punct::Punctuation::EqEq(value) => Self::Equal(value),
            punct::Punctuation::BangEq(value) => Self::Unequal(value),
            punct::Punctuation::Lt(value) => Self::Less(value),
            punct::Punctuation::LtEq(value) => Self::LessEqual(value),
            punct::Punctuation::Gt(value) => Self::Greater(value),
            punct::Punctuation::GtEq(value) => Self::GreaterEqual(value),
            punct::Punctuation::Percent(value) => Self::Divisible(value),
            _ => Ice::impossible(None),
        })
    }
}

impl Expectation {
    /// Retrieve the typed operator expectation.
    #[inline]
    pub const fn cause(&self) -> &Expected<TokenTree, Fragment> {
        let Self { cause, .. } = self;

        cause
    }
}

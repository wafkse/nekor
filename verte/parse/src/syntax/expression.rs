//! Unified value and predicate expressions with explicit concrete syntax.

use alloc::{boxed::Box, vec::Vec};
use core::fmt;

use aldebaran::{
    ice::Ice,
    prelude::{Annotated, Assert, Lex, Report, Span, Spanned},
    report::annotated::{InlineAnnotations, label::Label},
    text::cursor::error::Expected,
};
use fack::prelude::Error;

use super::{
    Braced, Bracketed, Parenthesized, Punctuated,
    declaration::{Key, KeyError},
};
use crate::scan::{
    ident, keyword, number, punct, string,
    tree::{self, SyntaxCursor, TokenTree, group::Delimiter, source::Tokens},
};

pub mod operator;

/// A scalar literal with its concrete lexical occurrence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Literal {
    /// An exact numeric occurrence.
    Number(number::Number),

    /// A decoded string with its original quoted source extent.
    String(string::Literal),

    /// A true keyword occurrence.
    True(keyword::True),

    /// A false keyword occurrence.
    False(keyword::False),
}

impl Spanned for Literal {
    #[inline]
    fn span(&self) -> Span {
        match self {
            Self::Number(value) => value.span(),
            Self::String(value) => value.span(),
            Self::True(value) => value.span(),
            Self::False(value) => value.span(),
        }
    }
}

/// An expression whose value or predicate interpretation is resolved after parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    /// A scalar literal.
    Literal(Literal),

    /// A symbolic reference without parser-side name resolution.
    Name(ident::Ident),

    /// A leading dot selecting from a predicate candidate.
    Selector(Selector),

    /// Explicit parentheses around an expression.
    Parenthesized(Parenthesized<Box<Expr>>),

    /// An array value with optional trailing comma.
    Array(Bracketed<Punctuated<Expr, punct::Comma>>),

    /// A record value with optional trailing comma.
    Record(Record),

    /// Member and index operations in source order.
    Postfix(Postfix),

    /// Arithmetic or logical prefix operations in source order.
    Unary(Unary),

    /// A prefix predicate and its postfix-expression bound.
    Predicate(Predicate),

    /// A prefix power predicate with the original unsigned argument text.
    PowerPredicate(PowerPredicate),

    /// Right-associative exponentiation.
    Power(Power),

    /// A flat sequence of operators at the same binding strength.
    Binary(Binary),
}

/// A brace-delimited record initializer.
pub type Record = Braced<Punctuated<RecordMember, punct::Comma>>;

node! {
    /// A leading candidate-member selection.
    pub struct Selector {
        /// The leading selector punctuation.
        dot: punct::Dot,

        /// The selected member name.
        member: ident::Ident,
    }

    /// A record initializer member.
    pub struct RecordMember {
        /// The identifier or quoted key.
        key: Key,

        /// The record assignment punctuation.
        equals: punct::Eq,

        /// The member's initializer expression.
        value: Expr,
    }

    /// A postfix expression and its ordered access operations.
    pub struct Postfix {
        /// The operand before the first access operation.
        base: Box<Expr>,

        /// Member or index operations in source order.
        operations: Vec<Access>,
    }

    /// A postfix member access.
    pub struct Member {
        /// Member access punctuation.
        dot: punct::Dot,

        /// The member's identifier occurrence.
        name: ident::Ident,
    }

    /// Recursive unary prefixes stored without recursion for a long prefix chain.
    pub struct Unary {
        /// Prefixes in their original outer-to-inner order.
        operators: Vec<operator::Unary>,

        /// The operand following the prefixes.
        operand: Box<Expr>,
    }

    /// A prefix predicate and its syntactically restricted bound.
    pub struct Predicate {
        /// The comparison or alignment punctuation.
        operator: operator::Predicate,

        /// The postfix expression providing the bound.
        bound: Box<Expr>,
    }

    /// A prefix power predicate, before semantic checking of its argument.
    pub struct PowerPredicate {
        /// Prefix predicate punctuation.
        caret: punct::Caret,

        /// The unsigned integer argument occurrence.
        exponent: number::Integer,
    }

    /// An exponentiation whose right operand follows the unary production.
    pub struct Power {
        /// The postfix expression being raised to a power.
        base: Box<Expr>,

        /// The exponentiation punctuation.
        operator: punct::StarStar,

        /// The recursively parsed unary exponent.
        exponent: Box<Expr>,
    }

    /// A left-associative sequence at a fixed binding strength.
    pub struct Binary {
        /// The first operand, before any operator in the sequence.
        first: Box<Expr>,

        /// Subsequent operators and their right operands.
        rest: Vec<BinaryTail>,
    }

    /// An operator followed by its right operand.
    pub struct BinaryTail {
        /// The concrete binary operator occurrence.
        operator: operator::Binary,

        /// The operand at the next tighter binding strength.
        operand: Expr,
    }
}

/// A postfix access whose delimiters remain concrete syntax.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Access {
    /// A dot and member identifier.
    Member(Member),

    /// A bracketed index expression.
    Index(Bracketed<Box<Expr>>),
}

impl Spanned for Access {
    #[inline]
    fn span(&self) -> Span {
        match self {
            Self::Member(value) => value.span(),
            Self::Index(value) => value.span(),
        }
    }
}

impl Spanned for Expr {
    #[inline]
    fn span(&self) -> Span {
        match self {
            Self::Literal(value) => value.span(),
            Self::Name(value) => value.span(),
            Self::Selector(value) => value.span(),
            Self::Parenthesized(value) => value.span(),
            Self::Array(value) => value.span(),
            Self::Record(value) => value.span(),
            Self::Postfix(value) => value.span(),
            Self::Unary(value) => value.span(),
            Self::Predicate(value) => value.span(),
            Self::PowerPredicate(value) => value.span(),
            Self::Power(value) => value.span(),
            Self::Binary(value) => value.span(),
        }
    }
}

/// Structural expectations of the expression grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Assert)]
#[assert(input(TokenTree))]
pub enum Fragment {
    /// Any expression prefix.
    #[assert(
        matches(
            TokenTree::Ident(_)
            | TokenTree::Number(_)
            | TokenTree::String(_)
            | TokenTree::Group(_)
            | TokenTree::Keyword(keyword::Keyword::True(_) | keyword::Keyword::False(_))
            | TokenTree::Punctuation(
                punct::Punctuation::Dot(_)
                | punct::Punctuation::Plus(_)
                | punct::Punctuation::Minus(_)
                | punct::Punctuation::Bang(_)
                | punct::Punctuation::Tilde(_)
                | punct::Punctuation::EqEq(_)
                | punct::Punctuation::BangEq(_)
                | punct::Punctuation::Lt(_)
                | punct::Punctuation::LtEq(_)
                | punct::Punctuation::Gt(_)
                | punct::Punctuation::GtEq(_)
                | punct::Punctuation::Percent(_)
                | punct::Punctuation::Caret(_),
            )
        ) as "an expression"
    )]
    Start,

    /// A symbolic member name.
    #[assert(matches(TokenTree::Ident(_)) as "an identifier")]
    Ident,

    /// The candidate or member selector.
    #[assert(matches(TokenTree::Punctuation(punct::Punctuation::Dot(_))) as "`.`")]
    Dot,

    /// Record member assignment.
    #[assert(matches(TokenTree::Punctuation(punct::Punctuation::Eq(_))) as "`=`")]
    Equals,

    /// An exponentiation operator.
    #[assert(matches(TokenTree::Punctuation(punct::Punctuation::StarStar(_))) as "`**`")]
    Power,

    /// A prefix power predicate.
    #[assert(matches(TokenTree::Punctuation(punct::Punctuation::Caret(_))) as "`^`")]
    Caret,

    /// An unsigned integer argument.
    #[assert(matches(TokenTree::Number(number::Number::Integer(_))) as "an unsigned integer")]
    Integer,

    /// The logical disjunction grammar level.
    #[assert(
        matches(TokenTree::Punctuation(punct::Punctuation::PipePipe(_))) as "a logical disjunction operator"
    )]
    LogicalOr,

    /// The logical conjunction grammar level.
    #[assert(
        matches(TokenTree::Punctuation(punct::Punctuation::AmpAmp(_))) as "a logical conjunction operator"
    )]
    LogicalAnd,

    /// The union grammar level.
    #[assert(matches(TokenTree::Punctuation(punct::Punctuation::Pipe(_))) as "a union operator")]
    Union,

    /// The xor grammar level.
    #[assert(matches(TokenTree::Punctuation(punct::Punctuation::Caret(_))) as "an exclusive-or operator")]
    Xor,

    /// The intersection grammar level.
    #[assert(
        matches(TokenTree::Punctuation(punct::Punctuation::Amp(_))) as "an intersection operator"
    )]
    Intersection,

    /// The comparison grammar level.
    #[assert(
        matches(
            TokenTree::Punctuation(
                punct::Punctuation::EqEq(_)
                | punct::Punctuation::BangEq(_)
                | punct::Punctuation::Lt(_)
                | punct::Punctuation::LtEq(_)
                | punct::Punctuation::Gt(_)
                | punct::Punctuation::GtEq(_),
            )
        ) as "a comparison operator"
    )]
    Comparison,

    /// The interval grammar level.
    #[assert(
        matches(
            TokenTree::Punctuation(
                punct::Punctuation::DotDot(_)
                | punct::Punctuation::DotDotEq(_)
                | punct::Punctuation::DotDotPlus(_),
            )
        ) as "an interval operator"
    )]
    Interval,

    /// The application grammar level.
    #[assert(matches(TokenTree::Punctuation(punct::Punctuation::At(_))) as "an application operator")]
    Application,

    /// The shift grammar level.
    #[assert(
        matches(
            TokenTree::Punctuation(punct::Punctuation::LtLt(_) | punct::Punctuation::GtGt(_))
        ) as "a shift operator"
    )]
    Shift,

    /// The additive grammar level.
    #[assert(
        matches(
            TokenTree::Punctuation(punct::Punctuation::Plus(_) | punct::Punctuation::Minus(_))
        ) as "an additive operator"
    )]
    Additive,

    /// The multiplicative grammar level.
    #[assert(
        matches(
            TokenTree::Punctuation(
                punct::Punctuation::Star(_)
                | punct::Punctuation::Slash(_)
                | punct::Punctuation::Percent(_),
            )
        ) as "a multiplicative operator"
    )]
    Multiplicative,

    /// A primary expression that can begin a predicate bound.
    #[assert(
        matches(
            TokenTree::Ident(_)
            | TokenTree::Number(_)
            | TokenTree::String(_)
            | TokenTree::Group(_)
            | TokenTree::Keyword(keyword::Keyword::True(_) | keyword::Keyword::False(_))
            | TokenTree::Punctuation(punct::Punctuation::Dot(_))
        ) as "a postfix expression"
    )]
    PostfixStart,

    /// Any group in postfix position, checked for bracket delimiters by its production.
    #[assert(matches(TokenTree::Group(_)) as "an index group")]
    Index,
}

/// Failure within the expression grammar.
#[derive(Debug, Clone, Report)]
#[report(forward)]
pub enum ExpressionError {
    /// An expression was missing without an enclosing group.
    Missing(Missing),

    /// An expression component was absent.
    Expected(Expectation),

    /// Source lexing failed.
    Source(tree::TokenError),

    /// An operator was required.
    Operator(operator::OperatorError),

    /// A record key failed.
    Key(KeyError),

    /// An operator has no right operand.
    Operand(Operand),

    /// A nonassociative operator was repeated.
    Chained(Chained),

    /// A parenthesized expression failed.
    Parenthesized(super::group::GroupError<ExpressionError>),

    /// A bracketed expression failed.
    Index(super::group::GroupError<ExpressionError>),

    /// An array initializer failed.
    Array(super::group::GroupError<super::list::ListError<ExpressionError>>),

    /// A record initializer failed.
    Record(super::group::GroupError<super::list::ListError<ExpressionError>>),
}

impl fmt::Display for ExpressionError {
    #[inline]
    fn fmt(&self, writer: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(error) => fmt::Display::fmt(error, writer),
            Self::Expected(error) => fmt::Display::fmt(error, writer),
            Self::Source(error) => fmt::Display::fmt(error, writer),
            Self::Operator(error) => fmt::Display::fmt(error, writer),
            Self::Key(error) => fmt::Display::fmt(error, writer),
            Self::Operand(error) => fmt::Display::fmt(error, writer),
            Self::Chained(error) => fmt::Display::fmt(error, writer),
            Self::Parenthesized(error) => fmt::Display::fmt(error, writer),
            Self::Index(error) => fmt::Display::fmt(error, writer),
            Self::Array(error) => fmt::Display::fmt(error, writer),
            Self::Record(error) => fmt::Display::fmt(error, writer),
        }
    }
}

impl Error for ExpressionError {
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Missing(error) => Some(error),
            Self::Expected(error) => Some(error),
            Self::Source(error) => Some(error),
            Self::Operator(error) => Some(error),
            Self::Key(error) => Some(error),
            Self::Operand(error) => Some(error),
            Self::Chained(error) => Some(error),
            Self::Parenthesized(error) => Some(error),
            Self::Index(error) => Some(error),
            Self::Array(error) => Some(error),
            Self::Record(error) => Some(error),
        }
    }
}

/// An expression was required at a source boundary or another token.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("expected an expression")]
#[error(source(cause))]
#[report(title = "expected an expression", annotations = labels)]
// NOTE(invariant): None.
pub struct Missing {
    /// Grammar expectation and the token or boundary found at failure.
    cause: Expected<TokenTree, Fragment>,

    /// The token or boundary at which an expression was required.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

impl Missing {
    /// Retrieve the syntax fragment expected at the failure location.
    #[inline]
    pub const fn cause(&self) -> &Expected<TokenTree, Fragment> {
        let Self { cause, .. } = self;

        cause
    }
}

/// An unmet expression component expectation.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("expected expression syntax")]
#[error(source(cause))]
#[report(title = "expected expression syntax", annotations = labels)]
// NOTE(invariant): Related context identifies the production needing this component.
pub struct Expectation {
    /// Grammar expectation and the token or boundary found at failure.
    cause: Expected<TokenTree, Fragment>,

    /// Primary failure location followed by related source context.
    labels: InlineAnnotations<Label<&'static str>, 1>,
}

impl Expectation {
    /// Retrieve the syntax fragment expected at the failure location.
    #[inline]
    pub const fn cause(&self) -> &Expected<TokenTree, Fragment> {
        let Self { cause, .. } = self;

        cause
    }
}

/// An operator missing its required operand.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("missing operator operand")]
#[error(source(cause))]
#[report(title = "missing operator operand", annotations = labels)]
// NOTE(invariant): The related label identifies the operator requiring an operand.
pub struct Operand {
    /// Grammar expectation and the token or boundary found at failure.
    cause: Expected<TokenTree, Fragment>,

    /// Primary failure location followed by related source context.
    labels: InlineAnnotations<Label<&'static str>, 1>,
}

impl Operand {
    /// Retrieve the syntax fragment expected at the failure location.
    #[inline]
    pub const fn cause(&self) -> &Expected<TokenTree, Fragment> {
        let Self { cause, .. } = self;

        cause
    }
}

/// A range or comparison was chained without explicit grouping.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("chained nonassociative operator")]
#[report(title = "chained nonassociative operator", annotations = labels)]
// NOTE(invariant): Both annotations identify operators at the same grammar level.
pub struct Chained {
    /// Primary failure location followed by related source context.
    labels: InlineAnnotations<Label<&'static str>, 1>,
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for RecordMember {
    type Context = ();
    type Error = ExpressionError;
    type Input<'i>
        = (Key, punct::Eq, Expr)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, ExpressionError>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();
        let key = Key::lex(&mut b0).map_err(ExpressionError::Key)?;
        let start = key.span();
        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::Equals))
        }
        .map_err(ExpressionError::Source)?
        .map_err(|cause| {
            let target = Annotated::target(&cause);
            let labels = InlineAnnotations::new(
                Label::new("record members use `key = expression`", target),
                [Label::new("this production starts here", start)],
            );

            ExpressionError::Expected(Expectation { cause, labels })
        })?;

        let TokenTree::Punctuation(punct::Punctuation::Eq(equals)) = token else {
            Ice::impossible(None)
        };

        let value = Expr::lex(&mut b0)?;

        b0.backport();

        Ok((key, equals, value))
    }

    #[inline]
    fn build_with_context<'i>((key, equals, value): Self::Input<'i>, _: &mut ()) -> Result<Self, ExpressionError>
    where
        't: 'i,
    {
        let span = key.span().superset(value.span());

        Ok(Self {
            key,
            equals,
            value,
            span,
        })
    }
}

/// A primary expression before postfix and operator composition.
#[derive(Debug)]
// NOTE(invariant): The contained expression is a primary production.
struct Primary(Expr);

/// Recognized primary alternatives awaiting node construction.
#[derive(Debug)]
enum PrimaryInput {
    /// A scalar occurrence.
    Literal(Literal),

    /// A symbolic name.
    Name(ident::Ident),

    /// A candidate selector.
    Selector(punct::Dot, ident::Ident),

    /// Explicit grouping.
    Parenthesized(Parenthesized<Box<Expr>>),

    /// Array syntax.
    Array(Bracketed<Punctuated<Expr, punct::Comma>>),

    /// Record syntax.
    Record(Record),
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Primary {
    type Context = ();
    type Error = ExpressionError;
    type Input<'i>
        = PrimaryInput
    where
        't: 'i;

    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<PrimaryInput, ExpressionError>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();
        let start = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source()
                .checked(b0.peek().map_or(Span::unit(b0.index()), |token| token.span()))
        }
        .map_err(ExpressionError::Source)?;
        let input = match {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.peek())
        }
        .map_err(ExpressionError::Source)?
        {
            Some(TokenTree::Ident(value)) => {
                {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source().checked(b0.any())
                }
                .map_err(ExpressionError::Source)?;
                PrimaryInput::Name(value)
            },
            Some(TokenTree::Number(value)) => {
                {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source().checked(b0.any())
                }
                .map_err(ExpressionError::Source)?;
                PrimaryInput::Literal(Literal::Number(value))
            },
            Some(TokenTree::String(value)) => {
                {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source().checked(b0.any())
                }
                .map_err(ExpressionError::Source)?;
                PrimaryInput::Literal(Literal::String(value))
            },
            Some(TokenTree::Keyword(keyword::Keyword::True(value))) => {
                {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source().checked(b0.any())
                }
                .map_err(ExpressionError::Source)?;
                PrimaryInput::Literal(Literal::True(value))
            },
            Some(TokenTree::Keyword(keyword::Keyword::False(value))) => {
                {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source().checked(b0.any())
                }
                .map_err(ExpressionError::Source)?;
                PrimaryInput::Literal(Literal::False(value))
            },
            Some(TokenTree::Punctuation(punct::Punctuation::Dot(dot))) => {
                {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source().checked(b0.any())
                }
                .map_err(ExpressionError::Source)?;

                let token = {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source().checked(b0.expect_is_next(Fragment::Ident))
                }
                .map_err(ExpressionError::Source)?
                .map_err(|cause| {
                    let target = Annotated::target(&cause);
                    let labels = InlineAnnotations::new(
                        Label::new("expected a member identifier after `.`", target),
                        [Label::new("this production starts here", start)],
                    );

                    ExpressionError::Expected(Expectation { cause, labels })
                })?;

                let TokenTree::Ident(member) = token else {
                    Ice::impossible(None)
                };

                PrimaryInput::Selector(dot, member)
            },
            Some(TokenTree::Group(group)) => match group.delimiter() {
                Delimiter::Parenthesis(..) => PrimaryInput::Parenthesized(
                    Parenthesized::<Box<Expr>>::lex(&mut b0).map_err(ExpressionError::Parenthesized)?,
                ),
                Delimiter::Bracket(..) => PrimaryInput::Array(
                    Bracketed::<Punctuated<Expr, punct::Comma>>::lex(&mut b0).map_err(ExpressionError::Array)?,
                ),
                Delimiter::Brace(..) => PrimaryInput::Record(Record::lex(&mut b0).map_err(ExpressionError::Record)?),
            },
            _ => {
                let readiness = {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source().checked(b0.expect_is_next(Fragment::PostfixStart))
                }
                .map_err(ExpressionError::Source)?;

                let Err(cause) = readiness else { Ice::impossible(None) };

                let message = "expected a literal, symbolic name, selector, or grouped expression";

                return Err(match b0.source().stream().scope() {
                    Some(group) => {
                        let labels = InlineAnnotations::new(
                            Label::new(message, start),
                            [Label::new(
                                "this expression group starts here",
                                group.delimiter().open(),
                            )],
                        );

                        ExpressionError::Expected(Expectation { cause, labels })
                    },
                    None => {
                        let labels = InlineAnnotations::new(Label::new(message, start), []);

                        ExpressionError::Missing(Missing { cause, labels })
                    },
                });
            },
        };

        b0.backport();

        Ok(input)
    }

    #[inline]
    fn build_with_context<'i>(input: PrimaryInput, _: &mut ()) -> Result<Self, ExpressionError>
    where
        't: 'i,
    {
        let value = match input {
            PrimaryInput::Literal(value) => Expr::Literal(value),
            PrimaryInput::Name(value) => Expr::Name(value),
            PrimaryInput::Selector(dot, member) => {
                let span = dot.span().superset(member.span());

                Expr::Selector(Selector { dot, member, span })
            },
            PrimaryInput::Parenthesized(value) => Expr::Parenthesized(value),
            PrimaryInput::Array(value) => Expr::Array(value),
            PrimaryInput::Record(value) => Expr::Record(value),
        };

        Ok(Self(value))
    }
}

/// A primary followed by member and index accesses.
#[derive(Debug)]
// NOTE(invariant): The contained expression is a postfix production.
struct PostfixExpr(Expr);

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for PostfixExpr {
    type Context = ();
    type Error = ExpressionError;
    type Input<'i>
        = (Expr, Vec<Access>)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, ExpressionError>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();

        let Primary(base) = Primary::lex(&mut b0)?;

        let mut operations = Vec::new();

        loop {
            if {
                let trivia = b0.source().trivia();

                b0.ignore_while(trivia);

                b0.source().checked(b0.is_next(Fragment::Dot))
            }
            .map_err(ExpressionError::Source)?
            {
                let start = {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source()
                        .checked(b0.peek().map_or(Span::unit(b0.index()), |token| token.span()))
                }
                .map_err(ExpressionError::Source)?;
                let token = {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source().checked(b0.expect_is_next(Fragment::Dot))
                }
                .map_err(ExpressionError::Source)?
                .map_err(|cause| {
                    let target = Annotated::target(&cause);
                    let labels = InlineAnnotations::new(
                        Label::new("expected a member selector", target),
                        [Label::new("this production starts here", start)],
                    );

                    ExpressionError::Expected(Expectation { cause, labels })
                })?;

                let TokenTree::Punctuation(punct::Punctuation::Dot(dot)) = token else {
                    Ice::impossible(None)
                };

                let token = {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source().checked(b0.expect_is_next(Fragment::Ident))
                }
                .map_err(ExpressionError::Source)?
                .map_err(|cause| {
                    let target = Annotated::target(&cause);
                    let labels = InlineAnnotations::new(
                        Label::new("expected a member identifier after `.`", target),
                        [Label::new("this production starts here", start)],
                    );

                    ExpressionError::Expected(Expectation { cause, labels })
                })?;

                let TokenTree::Ident(name) = token else {
                    Ice::impossible(None)
                };

                let span = dot.span().superset(name.span());

                operations.push(Access::Member(Member { dot, name, span }));
            } else if let Some(TokenTree::Group(group)) = {
                let trivia = b0.source().trivia();

                b0.ignore_while(trivia);

                b0.source().checked(b0.peek())
            }
            .map_err(ExpressionError::Source)?
            {
                if let Delimiter::Bracket(..) = group.delimiter() {
                    operations.push(Access::Index(
                        Bracketed::<Box<Expr>>::lex(&mut b0).map_err(ExpressionError::Index)?,
                    ));
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        b0.backport();

        Ok((base, operations))
    }

    #[inline]
    fn build_with_context<'i>((base, operations): Self::Input<'i>, _: &mut ()) -> Result<Self, ExpressionError>
    where
        't: 'i,
    {
        let value = match operations.last() {
            None => base,
            Some(last) => {
                let span = base.span().superset(last.span());
                let base = Box::new(base);

                Expr::Postfix(Postfix { base, operations, span })
            },
        };

        Ok(Self(value))
    }
}

/// Exponentiation with a unary right operand.
#[derive(Debug)]
// NOTE(invariant): The contained expression follows the power production.
struct PowerExpr(Expr);

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for PowerExpr {
    type Context = ();
    type Error = ExpressionError;
    type Input<'i>
        = (Expr, Option<(punct::StarStar, Expr)>)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, ExpressionError>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();

        let PostfixExpr(base) = PostfixExpr::lex(&mut b0)?;

        let tail = if {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.is_next(Fragment::Power))
        }
        .map_err(ExpressionError::Source)?
        {
            let start = {
                let trivia = b0.source().trivia();

                b0.ignore_while(trivia);

                b0.source()
                    .checked(b0.peek().map_or(Span::unit(b0.index()), |token| token.span()))
            }
            .map_err(ExpressionError::Source)?;
            let token = {
                let trivia = b0.source().trivia();

                b0.ignore_while(trivia);

                b0.source().checked(b0.expect_is_next(Fragment::Power))
            }
            .map_err(ExpressionError::Source)?
            .map_err(|cause| {
                let target = Annotated::target(&cause);
                let labels = InlineAnnotations::new(
                    Label::new("expected exponentiation", target),
                    [Label::new("this production starts here", start)],
                );

                ExpressionError::Expected(Expectation { cause, labels })
            })?;

            let TokenTree::Punctuation(punct::Punctuation::StarStar(operator)) = token else {
                Ice::impossible(None)
            };

            let readiness = {
                let mut b1 = b0.branch();
                let trivia = b1.source().trivia();

                b1.ignore_while(trivia);

                b1.source().checked(b1.expect_is_next(Fragment::Start))
            }
            .map_err(ExpressionError::Source)?;

            if let Err(cause) = readiness {
                let target = {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source()
                        .checked(b0.peek().map_or(Span::unit(b0.index()), |token| token.span()))
                }
                .map_err(ExpressionError::Source)?;
                let labels = InlineAnnotations::new(
                    Label::new("expected an exponent after `**`", target),
                    [Label::new("this operator requires a right operand", operator.span())],
                );

                return Err(ExpressionError::Operand(Operand { cause, labels }));
            }

            let UnaryExpr(exponent) = UnaryExpr::lex(&mut b0)?;

            Some((operator, exponent))
        } else {
            None
        };

        b0.backport();

        Ok((base, tail))
    }

    #[inline]
    fn build_with_context<'i>((base, tail): Self::Input<'i>, _: &mut ()) -> Result<Self, ExpressionError>
    where
        't: 'i,
    {
        let value = match tail {
            None => base,
            Some((operator, exponent)) => {
                let span = base.span().superset(exponent.span());
                let base = Box::new(base);
                let exponent = Box::new(exponent);

                Expr::Power(Power {
                    base,
                    operator,
                    exponent,
                    span,
                })
            },
        };

        Ok(Self(value))
    }
}

/// Recursive unary syntax, storing consecutive simple prefixes without recursion.
#[derive(Debug)]
// NOTE(invariant): The contained expression follows the unary production.
struct UnaryExpr(Expr);

/// The final operand of a simple prefix sequence.
#[derive(Debug)]
enum UnaryInput {
    /// A power production.
    Power(Expr),

    /// A relation or alignment predicate.
    Predicate(operator::Predicate, Expr),

    /// A prefix power predicate.
    PowerPredicate(punct::Caret, number::Integer),
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for UnaryExpr {
    type Context = ();
    type Error = ExpressionError;
    type Input<'i>
        = (Vec<operator::Unary>, UnaryInput)
    where
        't: 'i;

    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, ExpressionError>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();
        let mut operators = Vec::new();

        while {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.is_next(operator::Fragment::Unary))
        }
        .map_err(ExpressionError::Source)?
        {
            operators.push(operator::Unary::lex(&mut b0).map_err(ExpressionError::Operator)?);
        }

        let operand = if {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.is_next(operator::Fragment::Predicate))
        }
        .map_err(ExpressionError::Source)?
        {
            let operator = operator::Predicate::lex(&mut b0).map_err(ExpressionError::Operator)?;

            let readiness = {
                let mut b1 = b0.branch();
                let trivia = b1.source().trivia();

                b1.ignore_while(trivia);

                b1.source().checked(b1.expect_is_next(Fragment::PostfixStart))
            }
            .map_err(ExpressionError::Source)?;

            if let Err(cause) = readiness {
                let target = {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source()
                        .checked(b0.peek().map_or(Span::unit(b0.index()), |token| token.span()))
                }
                .map_err(ExpressionError::Source)?;
                let labels = InlineAnnotations::new(
                    Label::new(
                        "expected a postfix-expression bound; parenthesize arithmetic bounds",
                        target,
                    ),
                    [Label::new("this predicate requires a bound", operator.span())],
                );

                return Err(ExpressionError::Operand(Operand { cause, labels }));
            }

            let PostfixExpr(bound) = PostfixExpr::lex(&mut b0)?;

            UnaryInput::Predicate(operator, bound)
        } else if {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.is_next(Fragment::Caret))
        }
        .map_err(ExpressionError::Source)?
        {
            let start = {
                let trivia = b0.source().trivia();

                b0.ignore_while(trivia);

                b0.source()
                    .checked(b0.peek().map_or(Span::unit(b0.index()), |token| token.span()))
            }
            .map_err(ExpressionError::Source)?;
            let token = {
                let trivia = b0.source().trivia();

                b0.ignore_while(trivia);

                b0.source().checked(b0.expect_is_next(Fragment::Caret))
            }
            .map_err(ExpressionError::Source)?
            .map_err(|cause| {
                let target = Annotated::target(&cause);
                let labels = InlineAnnotations::new(
                    Label::new("expected a power predicate", target),
                    [Label::new("this production starts here", start)],
                );

                ExpressionError::Expected(Expectation { cause, labels })
            })?;

            let TokenTree::Punctuation(punct::Punctuation::Caret(caret)) = token else {
                Ice::impossible(None)
            };

            let token = {
                let trivia = b0.source().trivia();

                b0.ignore_while(trivia);

                b0.source().checked(b0.expect_is_next(Fragment::Integer))
            }
            .map_err(ExpressionError::Source)?
            .map_err(|cause| {
                let target = Annotated::target(&cause);
                let labels = InlineAnnotations::new(
                    Label::new("a prefix power predicate requires an unsigned integer", target),
                    [Label::new("this production starts here", start)],
                );

                ExpressionError::Expected(Expectation { cause, labels })
            })?;

            let TokenTree::Number(number::Number::Integer(exponent)) = token else {
                Ice::impossible(None)
            };

            UnaryInput::PowerPredicate(caret, exponent)
        } else {
            let readiness = {
                let mut b1 = b0.branch();
                let trivia = b1.source().trivia();

                b1.ignore_while(trivia);

                b1.source().checked(b1.expect_is_next(Fragment::Start))
            }
            .map_err(ExpressionError::Source)?;

            if let (Some(operator), Err(cause)) = (operators.last(), readiness) {
                let target = {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source()
                        .checked(b0.peek().map_or(Span::unit(b0.index()), |token| token.span()))
                }
                .map_err(ExpressionError::Source)?;
                let labels = InlineAnnotations::new(
                    Label::new("expected an operand after this prefix", target),
                    [Label::new("this prefix requires an operand", operator.span())],
                );

                return Err(ExpressionError::Operand(Operand { cause, labels }));
            }

            let PowerExpr(value) = PowerExpr::lex(&mut b0)?;

            UnaryInput::Power(value)
        };

        b0.backport();

        Ok((operators, operand))
    }

    #[inline]
    fn build_with_context<'i>((operators, operand): Self::Input<'i>, _: &mut ()) -> Result<Self, ExpressionError>
    where
        't: 'i,
    {
        let operand = match operand {
            UnaryInput::Power(value) => value,
            UnaryInput::Predicate(operator, bound) => {
                let span = operator.span().superset(bound.span());
                let bound = Box::new(bound);

                Expr::Predicate(Predicate { operator, bound, span })
            },
            UnaryInput::PowerPredicate(caret, exponent) => {
                let span = caret.span().superset(exponent.span());

                Expr::PowerPredicate(PowerPredicate { caret, exponent, span })
            },
        };
        let value = match operators.first() {
            None => operand,
            Some(first) => {
                let span = first.span().superset(operand.span());
                let operand = Box::new(operand);

                Expr::Unary(Unary {
                    operators,
                    operand,
                    span,
                })
            },
        };

        Ok(Self(value))
    }
}

/// The multiplicative expression production.
#[derive(Debug)]
// NOTE(invariant): The contained expression has no ungrouped operator weaker than this production.
struct Multiplicative(Expr);

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Multiplicative {
    type Context = ();
    type Error = ExpressionError;
    type Input<'i>
        = (Expr, Vec<(operator::Binary, Expr)>)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, ExpressionError>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();

        let UnaryExpr(first) = UnaryExpr::lex(&mut b0)?;

        let mut rest = Vec::new();

        while {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.is_next(Fragment::Multiplicative))
        }
        .map_err(ExpressionError::Source)?
        {
            let operator = operator::Binary::lex(&mut b0).map_err(ExpressionError::Operator)?;

            let readiness = {
                let mut b1 = b0.branch();
                let trivia = b1.source().trivia();

                b1.ignore_while(trivia);

                b1.source().checked(b1.expect_is_next(Fragment::Start))
            }
            .map_err(ExpressionError::Source)?;

            if let Err(cause) = readiness {
                let target = {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source()
                        .checked(b0.peek().map_or(Span::unit(b0.index()), |token| token.span()))
                }
                .map_err(ExpressionError::Source)?;
                let labels = InlineAnnotations::new(
                    Label::new("expected a right operand after this operator", target),
                    [Label::new("this operator requires a right operand", operator.span())],
                );

                return Err(ExpressionError::Operand(Operand { cause, labels }));
            }

            let UnaryExpr(operand) = UnaryExpr::lex(&mut b0)?;

            rest.push((operator, operand));
        }

        b0.backport();

        Ok((first, rest))
    }

    #[inline]
    fn build_with_context<'i>((first, tails): Self::Input<'i>, _: &mut ()) -> Result<Self, ExpressionError>
    where
        't: 'i,
    {
        let value = match tails.last() {
            None => first,
            Some((_, last)) => {
                let span = first.span().superset(last.span());
                let first = Box::new(first);
                let mut rest = Vec::with_capacity(tails.len());

                for (operator, operand) in tails {
                    let span = operator.span().superset(operand.span());

                    rest.push(BinaryTail {
                        operator,
                        operand,
                        span,
                    });
                }
                Expr::Binary(Binary { first, rest, span })
            },
        };

        Ok(Self(value))
    }
}

/// The additive expression production.
#[derive(Debug)]
// NOTE(invariant): The contained expression has no ungrouped operator weaker than this production.
struct Additive(Expr);

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Additive {
    type Context = ();
    type Error = ExpressionError;
    type Input<'i>
        = (Expr, Vec<(operator::Binary, Expr)>)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, ExpressionError>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();

        let Multiplicative(first) = Multiplicative::lex(&mut b0)?;

        let mut rest = Vec::new();

        while {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.is_next(Fragment::Additive))
        }
        .map_err(ExpressionError::Source)?
        {
            let operator = operator::Binary::lex(&mut b0).map_err(ExpressionError::Operator)?;

            let readiness = {
                let mut b1 = b0.branch();
                let trivia = b1.source().trivia();

                b1.ignore_while(trivia);

                b1.source().checked(b1.expect_is_next(Fragment::Start))
            }
            .map_err(ExpressionError::Source)?;

            if let Err(cause) = readiness {
                let target = {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source()
                        .checked(b0.peek().map_or(Span::unit(b0.index()), |token| token.span()))
                }
                .map_err(ExpressionError::Source)?;
                let labels = InlineAnnotations::new(
                    Label::new("expected a right operand after this operator", target),
                    [Label::new("this operator requires a right operand", operator.span())],
                );

                return Err(ExpressionError::Operand(Operand { cause, labels }));
            }

            let Multiplicative(operand) = Multiplicative::lex(&mut b0)?;

            rest.push((operator, operand));
        }

        b0.backport();

        Ok((first, rest))
    }

    #[inline]
    fn build_with_context<'i>((first, tails): Self::Input<'i>, _: &mut ()) -> Result<Self, ExpressionError>
    where
        't: 'i,
    {
        let value = match tails.last() {
            None => first,
            Some((_, last)) => {
                let span = first.span().superset(last.span());
                let first = Box::new(first);
                let mut rest = Vec::with_capacity(tails.len());

                for (operator, operand) in tails {
                    let span = operator.span().superset(operand.span());

                    rest.push(BinaryTail {
                        operator,
                        operand,
                        span,
                    });
                }
                Expr::Binary(Binary { first, rest, span })
            },
        };

        Ok(Self(value))
    }
}

/// The shift expression production.
#[derive(Debug)]
// NOTE(invariant): The contained expression has no ungrouped operator weaker than this production.
struct Shift(Expr);

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Shift {
    type Context = ();
    type Error = ExpressionError;
    type Input<'i>
        = (Expr, Vec<(operator::Binary, Expr)>)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, ExpressionError>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();

        let Additive(first) = Additive::lex(&mut b0)?;

        let mut rest = Vec::new();

        while {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.is_next(Fragment::Shift))
        }
        .map_err(ExpressionError::Source)?
        {
            let operator = operator::Binary::lex(&mut b0).map_err(ExpressionError::Operator)?;

            let readiness = {
                let mut b1 = b0.branch();
                let trivia = b1.source().trivia();

                b1.ignore_while(trivia);

                b1.source().checked(b1.expect_is_next(Fragment::Start))
            }
            .map_err(ExpressionError::Source)?;

            if let Err(cause) = readiness {
                let target = {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source()
                        .checked(b0.peek().map_or(Span::unit(b0.index()), |token| token.span()))
                }
                .map_err(ExpressionError::Source)?;
                let labels = InlineAnnotations::new(
                    Label::new("expected a right operand after this operator", target),
                    [Label::new("this operator requires a right operand", operator.span())],
                );

                return Err(ExpressionError::Operand(Operand { cause, labels }));
            }

            let Additive(operand) = Additive::lex(&mut b0)?;

            rest.push((operator, operand));
        }

        b0.backport();

        Ok((first, rest))
    }

    #[inline]
    fn build_with_context<'i>((first, tails): Self::Input<'i>, _: &mut ()) -> Result<Self, ExpressionError>
    where
        't: 'i,
    {
        let value = match tails.last() {
            None => first,
            Some((_, last)) => {
                let span = first.span().superset(last.span());
                let first = Box::new(first);
                let mut rest = Vec::with_capacity(tails.len());

                for (operator, operand) in tails {
                    let span = operator.span().superset(operand.span());

                    rest.push(BinaryTail {
                        operator,
                        operand,
                        span,
                    });
                }
                Expr::Binary(Binary { first, rest, span })
            },
        };

        Ok(Self(value))
    }
}

/// The application expression production.
#[derive(Debug)]
// NOTE(invariant): The contained expression has no ungrouped operator weaker than this production.
struct Application(Expr);

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Application {
    type Context = ();
    type Error = ExpressionError;
    type Input<'i>
        = (Expr, Vec<(operator::Binary, Expr)>)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, ExpressionError>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();

        let Shift(first) = Shift::lex(&mut b0)?;

        let mut rest = Vec::new();

        while {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.is_next(Fragment::Application))
        }
        .map_err(ExpressionError::Source)?
        {
            let operator = operator::Binary::lex(&mut b0).map_err(ExpressionError::Operator)?;

            let readiness = {
                let mut b1 = b0.branch();
                let trivia = b1.source().trivia();

                b1.ignore_while(trivia);

                b1.source().checked(b1.expect_is_next(Fragment::Start))
            }
            .map_err(ExpressionError::Source)?;

            if let Err(cause) = readiness {
                let target = {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source()
                        .checked(b0.peek().map_or(Span::unit(b0.index()), |token| token.span()))
                }
                .map_err(ExpressionError::Source)?;
                let labels = InlineAnnotations::new(
                    Label::new("expected a right operand after this operator", target),
                    [Label::new("this operator requires a right operand", operator.span())],
                );

                return Err(ExpressionError::Operand(Operand { cause, labels }));
            }

            let Shift(operand) = Shift::lex(&mut b0)?;

            rest.push((operator, operand));
        }

        b0.backport();

        Ok((first, rest))
    }

    #[inline]
    fn build_with_context<'i>((first, tails): Self::Input<'i>, _: &mut ()) -> Result<Self, ExpressionError>
    where
        't: 'i,
    {
        let value = match tails.last() {
            None => first,
            Some((_, last)) => {
                let span = first.span().superset(last.span());
                let first = Box::new(first);
                let mut rest = Vec::with_capacity(tails.len());

                for (operator, operand) in tails {
                    let span = operator.span().superset(operand.span());

                    rest.push(BinaryTail {
                        operator,
                        operand,
                        span,
                    });
                }
                Expr::Binary(Binary { first, rest, span })
            },
        };

        Ok(Self(value))
    }
}

/// The interval expression production.
#[derive(Debug)]
// NOTE(invariant): The contained expression has no ungrouped operator weaker than this production.
struct Interval(Expr);

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Interval {
    type Context = ();
    type Error = ExpressionError;
    type Input<'i>
        = (Expr, Vec<(operator::Binary, Expr)>)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, ExpressionError>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();

        let Application(first) = Application::lex(&mut b0)?;

        let mut rest = Vec::new();

        while {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.is_next(Fragment::Interval))
        }
        .map_err(ExpressionError::Source)?
        {
            let operator = operator::Binary::lex(&mut b0).map_err(ExpressionError::Operator)?;

            if let Some((previous, _)) = rest.first() {
                let labels = InlineAnnotations::new(
                    Label::new("parenthesize nested ranges or comparisons explicitly", operator.span()),
                    [Label::new(
                        "the first operator at this level is here",
                        operator::Binary::span(previous),
                    )],
                );

                return Err(ExpressionError::Chained(Chained { labels }));
            }

            let readiness = {
                let mut b1 = b0.branch();
                let trivia = b1.source().trivia();

                b1.ignore_while(trivia);

                b1.source().checked(b1.expect_is_next(Fragment::Start))
            }
            .map_err(ExpressionError::Source)?;

            if let Err(cause) = readiness {
                let target = {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source()
                        .checked(b0.peek().map_or(Span::unit(b0.index()), |token| token.span()))
                }
                .map_err(ExpressionError::Source)?;
                let labels = InlineAnnotations::new(
                    Label::new("expected a right operand after this operator", target),
                    [Label::new("this operator requires a right operand", operator.span())],
                );

                return Err(ExpressionError::Operand(Operand { cause, labels }));
            }

            let Application(operand) = Application::lex(&mut b0)?;

            rest.push((operator, operand));
        }

        b0.backport();

        Ok((first, rest))
    }

    #[inline]
    fn build_with_context<'i>((first, tails): Self::Input<'i>, _: &mut ()) -> Result<Self, ExpressionError>
    where
        't: 'i,
    {
        let value = match tails.last() {
            None => first,
            Some((_, last)) => {
                let span = first.span().superset(last.span());
                let first = Box::new(first);
                let mut rest = Vec::with_capacity(tails.len());

                for (operator, operand) in tails {
                    let span = operator.span().superset(operand.span());

                    rest.push(BinaryTail {
                        operator,
                        operand,
                        span,
                    });
                }
                Expr::Binary(Binary { first, rest, span })
            },
        };

        Ok(Self(value))
    }
}

/// The comparison expression production.
#[derive(Debug)]
// NOTE(invariant): The contained expression has no ungrouped operator weaker than this production.
struct Comparison(Expr);

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Comparison {
    type Context = ();
    type Error = ExpressionError;
    type Input<'i>
        = (Expr, Vec<(operator::Binary, Expr)>)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, ExpressionError>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();

        let Interval(first) = Interval::lex(&mut b0)?;

        let mut rest = Vec::new();

        while {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.is_next(Fragment::Comparison))
        }
        .map_err(ExpressionError::Source)?
        {
            let operator = operator::Binary::lex(&mut b0).map_err(ExpressionError::Operator)?;

            if let Some((previous, _)) = rest.first() {
                let labels = InlineAnnotations::new(
                    Label::new("parenthesize nested ranges or comparisons explicitly", operator.span()),
                    [Label::new(
                        "the first operator at this level is here",
                        operator::Binary::span(previous),
                    )],
                );

                return Err(ExpressionError::Chained(Chained { labels }));
            }

            let readiness = {
                let mut b1 = b0.branch();
                let trivia = b1.source().trivia();

                b1.ignore_while(trivia);

                b1.source().checked(b1.expect_is_next(Fragment::Start))
            }
            .map_err(ExpressionError::Source)?;

            if let Err(cause) = readiness {
                let target = {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source()
                        .checked(b0.peek().map_or(Span::unit(b0.index()), |token| token.span()))
                }
                .map_err(ExpressionError::Source)?;
                let labels = InlineAnnotations::new(
                    Label::new("expected a right operand after this operator", target),
                    [Label::new("this operator requires a right operand", operator.span())],
                );

                return Err(ExpressionError::Operand(Operand { cause, labels }));
            }

            let Interval(operand) = Interval::lex(&mut b0)?;

            rest.push((operator, operand));
        }

        b0.backport();

        Ok((first, rest))
    }

    #[inline]
    fn build_with_context<'i>((first, tails): Self::Input<'i>, _: &mut ()) -> Result<Self, ExpressionError>
    where
        't: 'i,
    {
        let value = match tails.last() {
            None => first,
            Some((_, last)) => {
                let span = first.span().superset(last.span());
                let first = Box::new(first);
                let mut rest = Vec::with_capacity(tails.len());

                for (operator, operand) in tails {
                    let span = operator.span().superset(operand.span());

                    rest.push(BinaryTail {
                        operator,
                        operand,
                        span,
                    });
                }
                Expr::Binary(Binary { first, rest, span })
            },
        };

        Ok(Self(value))
    }
}

/// The intersection expression production.
#[derive(Debug)]
// NOTE(invariant): The contained expression has no ungrouped operator weaker than this production.
struct Intersection(Expr);

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Intersection {
    type Context = ();
    type Error = ExpressionError;
    type Input<'i>
        = (Expr, Vec<(operator::Binary, Expr)>)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, ExpressionError>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();

        let Comparison(first) = Comparison::lex(&mut b0)?;

        let mut rest = Vec::new();

        while {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.is_next(Fragment::Intersection))
        }
        .map_err(ExpressionError::Source)?
        {
            let operator = operator::Binary::lex(&mut b0).map_err(ExpressionError::Operator)?;

            let readiness = {
                let mut b1 = b0.branch();
                let trivia = b1.source().trivia();

                b1.ignore_while(trivia);

                b1.source().checked(b1.expect_is_next(Fragment::Start))
            }
            .map_err(ExpressionError::Source)?;

            if let Err(cause) = readiness {
                let target = {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source()
                        .checked(b0.peek().map_or(Span::unit(b0.index()), |token| token.span()))
                }
                .map_err(ExpressionError::Source)?;
                let labels = InlineAnnotations::new(
                    Label::new("expected a right operand after this operator", target),
                    [Label::new("this operator requires a right operand", operator.span())],
                );

                return Err(ExpressionError::Operand(Operand { cause, labels }));
            }

            let Comparison(operand) = Comparison::lex(&mut b0)?;

            rest.push((operator, operand));
        }

        b0.backport();

        Ok((first, rest))
    }

    #[inline]
    fn build_with_context<'i>((first, tails): Self::Input<'i>, _: &mut ()) -> Result<Self, ExpressionError>
    where
        't: 'i,
    {
        let value = match tails.last() {
            None => first,
            Some((_, last)) => {
                let span = first.span().superset(last.span());
                let first = Box::new(first);
                let mut rest = Vec::with_capacity(tails.len());

                for (operator, operand) in tails {
                    let span = operator.span().superset(operand.span());

                    rest.push(BinaryTail {
                        operator,
                        operand,
                        span,
                    });
                }
                Expr::Binary(Binary { first, rest, span })
            },
        };

        Ok(Self(value))
    }
}

/// The xor expression production.
#[derive(Debug)]
// NOTE(invariant): The contained expression has no ungrouped operator weaker than this production.
struct Xor(Expr);

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Xor {
    type Context = ();
    type Error = ExpressionError;
    type Input<'i>
        = (Expr, Vec<(operator::Binary, Expr)>)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, ExpressionError>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();

        let Intersection(first) = Intersection::lex(&mut b0)?;

        let mut rest = Vec::new();

        while {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.is_next(Fragment::Xor))
        }
        .map_err(ExpressionError::Source)?
        {
            let operator = operator::Binary::lex(&mut b0).map_err(ExpressionError::Operator)?;

            let readiness = {
                let mut b1 = b0.branch();
                let trivia = b1.source().trivia();

                b1.ignore_while(trivia);

                b1.source().checked(b1.expect_is_next(Fragment::Start))
            }
            .map_err(ExpressionError::Source)?;

            if let Err(cause) = readiness {
                let target = {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source()
                        .checked(b0.peek().map_or(Span::unit(b0.index()), |token| token.span()))
                }
                .map_err(ExpressionError::Source)?;
                let labels = InlineAnnotations::new(
                    Label::new("expected a right operand after this operator", target),
                    [Label::new("this operator requires a right operand", operator.span())],
                );

                return Err(ExpressionError::Operand(Operand { cause, labels }));
            }

            let Intersection(operand) = Intersection::lex(&mut b0)?;

            rest.push((operator, operand));
        }

        b0.backport();

        Ok((first, rest))
    }

    #[inline]
    fn build_with_context<'i>((first, tails): Self::Input<'i>, _: &mut ()) -> Result<Self, ExpressionError>
    where
        't: 'i,
    {
        let value = match tails.last() {
            None => first,
            Some((_, last)) => {
                let span = first.span().superset(last.span());
                let first = Box::new(first);
                let mut rest = Vec::with_capacity(tails.len());

                for (operator, operand) in tails {
                    let span = operator.span().superset(operand.span());

                    rest.push(BinaryTail {
                        operator,
                        operand,
                        span,
                    });
                }
                Expr::Binary(Binary { first, rest, span })
            },
        };

        Ok(Self(value))
    }
}

/// The union expression production.
#[derive(Debug)]
// NOTE(invariant): The contained expression has no ungrouped operator weaker than this production.
struct Union(Expr);

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Union {
    type Context = ();
    type Error = ExpressionError;
    type Input<'i>
        = (Expr, Vec<(operator::Binary, Expr)>)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, ExpressionError>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();

        let Xor(first) = Xor::lex(&mut b0)?;

        let mut rest = Vec::new();

        while {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.is_next(Fragment::Union))
        }
        .map_err(ExpressionError::Source)?
        {
            let operator = operator::Binary::lex(&mut b0).map_err(ExpressionError::Operator)?;

            let readiness = {
                let mut b1 = b0.branch();
                let trivia = b1.source().trivia();

                b1.ignore_while(trivia);

                b1.source().checked(b1.expect_is_next(Fragment::Start))
            }
            .map_err(ExpressionError::Source)?;

            if let Err(cause) = readiness {
                let target = {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source()
                        .checked(b0.peek().map_or(Span::unit(b0.index()), |token| token.span()))
                }
                .map_err(ExpressionError::Source)?;
                let labels = InlineAnnotations::new(
                    Label::new("expected a right operand after this operator", target),
                    [Label::new("this operator requires a right operand", operator.span())],
                );

                return Err(ExpressionError::Operand(Operand { cause, labels }));
            }

            let Xor(operand) = Xor::lex(&mut b0)?;

            rest.push((operator, operand));
        }

        b0.backport();

        Ok((first, rest))
    }

    #[inline]
    fn build_with_context<'i>((first, tails): Self::Input<'i>, _: &mut ()) -> Result<Self, ExpressionError>
    where
        't: 'i,
    {
        let value = match tails.last() {
            None => first,
            Some((_, last)) => {
                let span = first.span().superset(last.span());
                let first = Box::new(first);
                let mut rest = Vec::with_capacity(tails.len());

                for (operator, operand) in tails {
                    let span = operator.span().superset(operand.span());

                    rest.push(BinaryTail {
                        operator,
                        operand,
                        span,
                    });
                }
                Expr::Binary(Binary { first, rest, span })
            },
        };

        Ok(Self(value))
    }
}

/// The logical conjunction expression production.
#[derive(Debug)]
// NOTE(invariant): The contained expression has no ungrouped operator weaker than this production.
struct LogicalAnd(Expr);

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for LogicalAnd {
    type Context = ();
    type Error = ExpressionError;
    type Input<'i>
        = (Expr, Vec<(operator::Binary, Expr)>)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, ExpressionError>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();

        let Union(first) = Union::lex(&mut b0)?;

        let mut rest = Vec::new();

        while {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.is_next(Fragment::LogicalAnd))
        }
        .map_err(ExpressionError::Source)?
        {
            let operator = operator::Binary::lex(&mut b0).map_err(ExpressionError::Operator)?;

            let readiness = {
                let mut b1 = b0.branch();
                let trivia = b1.source().trivia();

                b1.ignore_while(trivia);

                b1.source().checked(b1.expect_is_next(Fragment::Start))
            }
            .map_err(ExpressionError::Source)?;

            if let Err(cause) = readiness {
                let target = {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source()
                        .checked(b0.peek().map_or(Span::unit(b0.index()), |token| token.span()))
                }
                .map_err(ExpressionError::Source)?;
                let labels = InlineAnnotations::new(
                    Label::new("expected a right operand after this operator", target),
                    [Label::new("this operator requires a right operand", operator.span())],
                );

                return Err(ExpressionError::Operand(Operand { cause, labels }));
            }

            let Union(operand) = Union::lex(&mut b0)?;

            rest.push((operator, operand));
        }

        b0.backport();

        Ok((first, rest))
    }

    #[inline]
    fn build_with_context<'i>((first, tails): Self::Input<'i>, _: &mut ()) -> Result<Self, ExpressionError>
    where
        't: 'i,
    {
        let value = match tails.last() {
            None => first,
            Some((_, last)) => {
                let span = first.span().superset(last.span());
                let first = Box::new(first);
                let mut rest = Vec::with_capacity(tails.len());

                for (operator, operand) in tails {
                    let span = operator.span().superset(operand.span());

                    rest.push(BinaryTail {
                        operator,
                        operand,
                        span,
                    });
                }
                Expr::Binary(Binary { first, rest, span })
            },
        };

        Ok(Self(value))
    }
}

/// The logical disjunction expression production.
#[derive(Debug)]
// NOTE(invariant): The contained expression has no ungrouped operator weaker than this production.
struct LogicalOr(Expr);

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for LogicalOr {
    type Context = ();
    type Error = ExpressionError;
    type Input<'i>
        = (Expr, Vec<(operator::Binary, Expr)>)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, ExpressionError>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();

        let LogicalAnd(first) = LogicalAnd::lex(&mut b0)?;

        let mut rest = Vec::new();

        while {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.is_next(Fragment::LogicalOr))
        }
        .map_err(ExpressionError::Source)?
        {
            let operator = operator::Binary::lex(&mut b0).map_err(ExpressionError::Operator)?;

            let readiness = {
                let mut b1 = b0.branch();
                let trivia = b1.source().trivia();

                b1.ignore_while(trivia);

                b1.source().checked(b1.expect_is_next(Fragment::Start))
            }
            .map_err(ExpressionError::Source)?;

            if let Err(cause) = readiness {
                let target = {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source()
                        .checked(b0.peek().map_or(Span::unit(b0.index()), |token| token.span()))
                }
                .map_err(ExpressionError::Source)?;
                let labels = InlineAnnotations::new(
                    Label::new("expected a right operand after this operator", target),
                    [Label::new("this operator requires a right operand", operator.span())],
                );

                return Err(ExpressionError::Operand(Operand { cause, labels }));
            }

            let LogicalAnd(operand) = LogicalAnd::lex(&mut b0)?;

            rest.push((operator, operand));
        }

        b0.backport();

        Ok((first, rest))
    }

    #[inline]
    fn build_with_context<'i>((first, tails): Self::Input<'i>, _: &mut ()) -> Result<Self, ExpressionError>
    where
        't: 'i,
    {
        let value = match tails.last() {
            None => first,
            Some((_, last)) => {
                let span = first.span().superset(last.span());
                let first = Box::new(first);
                let mut rest = Vec::with_capacity(tails.len());

                for (operator, operand) in tails {
                    let span = operator.span().superset(operand.span());

                    rest.push(BinaryTail {
                        operator,
                        operand,
                        span,
                    });
                }
                Expr::Binary(Binary { first, rest, span })
            },
        };

        Ok(Self(value))
    }
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Expr {
    type Context = ();
    type Error = ExpressionError;
    type Input<'i>
        = (Expr, Vec<(operator::Binary, Expr)>)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, ExpressionError>
    where
        't: 'i,
    {
        LogicalOr::pass(c0)
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, ExpressionError>
    where
        't: 'i,
    {
        let LogicalOr(value) = LogicalOr::build(input)?;

        Ok(value)
    }
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Box<Expr> {
    type Context = ();
    type Error = ExpressionError;
    type Input<'i>
        = <Expr as Lex<'t, &'t Tokens<'s>>>::Input<'i>
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, ExpressionError>
    where
        't: 'i,
    {
        Expr::pass(c0)
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, ExpressionError>
    where
        't: 'i,
    {
        Expr::build(input).map(Box::new)
    }
}

#[cfg(test)]
mod tests {
    use alloc::{borrow::ToOwned as _, format, string::String};
    use core::error::Error;

    use aldebaran::{
        prelude::{Annotated, Annotations, Cursor, Lex, Spanned},
        text::cursor::error::Expected,
    };

    use super::{Expr, ExpressionError, Fragment, *};
    use crate::scan::{
        context::Shared,
        tree::{Stream, TokenTree, source::Tokens},
    };

    /// Expression source used to compare grouping without discarding explicit syntax.
    enum Expression<'source> {
        /// The source used for syntax-shape observations.
        Source(&'source str),
    }

    impl Expression<'_> {
        /// Render only the grammatical grouping being asserted by these tests.
        fn shape(&self, expression: &Expr) -> String {
            let &Self::Source(text) = self;

            match expression {
                Expr::Binary(value) => {
                    let mut shape = self.shape(value.first());

                    for tail in value.rest() {
                        let operator = text.get(tail.operator().span().range()).expect("operator span");

                        shape = format!("({shape} {operator} {})", self.shape(tail.operand()));
                    }
                    shape
                },
                Expr::Power(value) => format!("({} ** {})", self.shape(value.base()), self.shape(value.exponent())),
                Expr::Unary(value) => {
                    let mut shape = self.shape(value.operand());

                    for operator in value.operators().iter().rev() {
                        let operator = text.get(operator.span().range()).expect("prefix span");

                        shape = format!("({operator}{shape})");
                    }
                    shape
                },
                Expr::Parenthesized(value) => format!("group({})", self.shape(value.value())),
                _ => text.get(expression.span().range()).expect("expression span").to_owned(),
            }
        }
    }

    #[test]
    fn precedence_and_associativity_follow_the_unified_grammar() {
        for (source, expected) in [
            ("1+2*3", "(1 + (2 * 3))"),
            ("1*2+3", "((1 * 2) + 3)"),
            ("a-b-c", "((a - b) - c)"),
            ("a<<b+c", "(a << (b + c))"),
            ("a@b<<c", "(a @ (b << c))"),
            ("a..b@c", "(a .. (b @ c))"),
            ("a<b..c", "(a < (b .. c))"),
            ("a&b<c", "(a & (b < c))"),
            ("a^b&c", "(a ^ (b & c))"),
            ("a|b^c", "(a | (b ^ c))"),
            ("a&&b|c", "(a && (b | c))"),
            ("a||b&&c", "(a || (b && c))"),
            ("2**3**4", "(2 ** (3 ** 4))"),
            ("-2**2", "(-(2 ** 2))"),
            ("2**-3", "(2 ** (-3))"),
            ("(1+2)*3", "(group((1 + 2)) * 3)"),
            ("1+(2+3)", "(1 + group((2 + 3)))"),
            ("a|b^c&d<e..f", "(a | (b ^ (c & (d < (e .. f)))))"),
        ] {
            let fixture = Expression::Source(source);
            let expression = {
                let text = source;

                let context = Shared::new();
                let stream = Stream::new(Cursor::create(text), context, None);

                let tokens = Tokens::new(stream);
                let tokens_ref = &tokens;
                let mut c0 = Cursor::at(&tokens_ref, tokens.stream().position());
                let value = Expr::lex(&mut c0).expect("valid expression");

                assert!(
                    {
                        let trivia = c0.source().trivia();

                        c0.ignore_while(trivia);

                        c0.source().checked(c0.peek())
                    }
                    .expect("trailing trivia")
                    .is_none()
                );

                value
            };

            assert_eq!(fixture.shape(&expression), expected, "incorrect grouping for {source}");
        }
    }

    #[test]
    fn every_binary_operator_preserves_its_concrete_source_occurrence() {
        for operator in [
            "||", "&&", "|", "^", "&", "==", "!=", "<", "<=", ">", ">=", "..", "..=", "..+", "@", "<<", ">>", "+", "-",
            "*", "/", "%",
        ] {
            let source = format!("a {operator} b");

            let Expr::Binary(value) = ({
                let text = &source;

                let context = Shared::new();
                let stream = Stream::new(Cursor::create(text), context, None);

                let tokens = Tokens::new(stream);
                let tokens_ref = &tokens;
                let mut c0 = Cursor::at(&tokens_ref, tokens.stream().position());
                let value = Expr::lex(&mut c0).expect("valid expression");

                assert!(
                    {
                        let trivia = c0.source().trivia();

                        c0.ignore_while(trivia);

                        c0.source().checked(c0.peek())
                    }
                    .expect("trailing trivia")
                    .is_none()
                );

                value
            }) else {
                unreachable!("expected a binary expression")
            };

            let tail = value.rest().first().expect("binary tail");

            assert_eq!(
                source.get(tail.operator().span().range()),
                Some(operator),
                "compound punctuation must have a single exact span"
            );

            assert_eq!(
                value.rest().len(),
                1,
                "binary expression must retain its single operation"
            );
        }
    }

    #[test]
    fn predicate_prefixes_and_postfix_access_keep_their_roles() {
        let Expr::Predicate(value) = ({
            let text = "% a.b[0]";

            let context = Shared::new();
            let stream = Stream::new(Cursor::create(text), context, None);

            let tokens = Tokens::new(stream);
            let tokens_ref = &tokens;
            let mut c0 = Cursor::at(&tokens_ref, tokens.stream().position());
            let value = Expr::lex(&mut c0).expect("valid expression");

            assert!(
                {
                    let trivia = c0.source().trivia();

                    c0.ignore_while(trivia);

                    c0.source().checked(c0.peek())
                }
                .expect("trailing trivia")
                .is_none()
            );

            value
        }) else {
            unreachable!("prefix alignment predicate")
        };

        assert!(
            matches!(value.bound().as_ref(), Expr::Postfix(_)),
            "predicate bounds consume a postfix expression"
        );
        let Expr::Binary(value) = ({
            let text = ">= a * 2";

            let context = Shared::new();
            let stream = Stream::new(Cursor::create(text), context, None);

            let tokens = Tokens::new(stream);
            let tokens_ref = &tokens;
            let mut c0 = Cursor::at(&tokens_ref, tokens.stream().position());
            let value = Expr::lex(&mut c0).expect("valid expression");

            assert!(
                {
                    let trivia = c0.source().trivia();

                    c0.ignore_while(trivia);

                    c0.source().checked(c0.peek())
                }
                .expect("trailing trivia")
                .is_none()
            );

            value
        }) else {
            unreachable!("unified binary expression")
        };

        assert!(
            matches!(value.first().as_ref(), Expr::Predicate(_)),
            "ungrouped arithmetic is outside the predicate bound"
        );
        let Expr::Predicate(value) = ({
            let text = ">= (a * 2)";

            let context = Shared::new();
            let stream = Stream::new(Cursor::create(text), context, None);

            let tokens = Tokens::new(stream);
            let tokens_ref = &tokens;
            let mut c0 = Cursor::at(&tokens_ref, tokens.stream().position());
            let value = Expr::lex(&mut c0).expect("valid expression");

            assert!(
                {
                    let trivia = c0.source().trivia();

                    c0.ignore_while(trivia);

                    c0.source().checked(c0.peek())
                }
                .expect("trailing trivia")
                .is_none()
            );

            value
        }) else {
            unreachable!("grouped predicate bound")
        };

        assert!(
            matches!(value.bound().as_ref(), Expr::Parenthesized(_)),
            "arithmetic bounds retain their explicit parentheses"
        );

        assert!(
            matches!(
                {
                    let text = "^2";

                    let context = Shared::new();
                    let stream = Stream::new(Cursor::create(text), context, None);

                    let tokens = Tokens::new(stream);
                    let tokens_ref = &tokens;
                    let mut c0 = Cursor::at(&tokens_ref, tokens.stream().position());
                    let value = Expr::lex(&mut c0).expect("valid expression");

                    assert!(
                        {
                            let trivia = c0.source().trivia();

                            c0.ignore_while(trivia);

                            c0.source().checked(c0.peek())
                        }
                        .expect("trailing trivia")
                        .is_none()
                    );

                    value
                },
                Expr::PowerPredicate(_)
            ),
            "prefix caret differs from binary XOR"
        );
        let Expr::Postfix(value) = ({
            let text = ".outer.inner[0]";

            let context = Shared::new();
            let stream = Stream::new(Cursor::create(text), context, None);

            let tokens = Tokens::new(stream);
            let tokens_ref = &tokens;
            let mut c0 = Cursor::at(&tokens_ref, tokens.stream().position());
            let value = Expr::lex(&mut c0).expect("valid expression");

            assert!(
                {
                    let trivia = c0.source().trivia();

                    c0.ignore_while(trivia);

                    c0.source().checked(c0.peek())
                }
                .expect("trailing trivia")
                .is_none()
            );

            value
        }) else {
            unreachable!("candidate access chain")
        };

        assert!(
            matches!(value.base().as_ref(), Expr::Selector(_)),
            "leading selection remains distinct from member access"
        );

        assert_eq!(value.operations().len(), 2, "member and index steps must be retained");
    }

    #[test]
    fn prefix_parsing_preserves_the_outer_separator() {
        let text = "1 + 2, \"";
        let context = Shared::new();
        let stream = Stream::new(Cursor::create(text), context, None);

        let tokens = Tokens::new(stream);
        let tokens_ref = &tokens;
        let mut c0 = Cursor::at(&tokens_ref, tokens.stream().position());
        let value = Expr::lex(&mut c0).expect("expression prefix");

        assert_eq!(value.span().range(), 0..5);

        assert!(matches!(
            {
                let trivia = c0.source().trivia();

                c0.ignore_while(trivia);

                c0.source().checked(c0.peek())
            },
            Ok(Some(TokenTree::Punctuation(punct::Punctuation::Comma(_))))
        ));
    }
    #[test]
    fn missing_operands_retain_the_required_fragment_and_operator() {
        for source in ["1 +", "2 **", "-", ">=", "%"] {
            let stream = Stream::new(Cursor::create(source), Shared::new(), None);

            let tokens = Tokens::new(stream);
            let tokens_ref = &tokens;
            let mut c0 = Cursor::at(&tokens_ref, tokens.stream().position());

            let ExpressionError::Operand(error) = Expr::lex(&mut c0).expect_err("missing operand") else {
                unreachable!("the operand failure must retain its owning operator")
            };

            let labels = error.annotations().list().expect("operand labels");
            let cause = Error::source(&error).expect("typed cursor failure");

            assert!(cause.downcast_ref::<Expected<TokenTree, Fragment>>().is_some());

            assert_eq!(Annotated::target(error.cause()).start(), source.len());

            assert_eq!(labels.first().target().start(), source.len());

            assert!(labels.rest()[0].target().start() < source.len());

            assert_eq!(c0.index(), 0);
        }
    }
}

//! Structural types and rule associations before name resolution.

use alloc::boxed::Box;
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
    expression::{self, Expr},
};
use crate::scan::{
    ident, keyword, number, punct, string,
    tree::{self, SyntaxCursor, TokenTree, group::Delimiter, source::Tokens},
};

node! {
    /// A type atom with an optional explicitly written refinement.
    pub struct Type {
        /// The unrefined syntactic type.
        atom: Atom,

        /// The optional rule association and its `@` occurrence.
        refinement: Option<Refinement>,
    }

    /// A named type with structurally uniform constructor arguments.
    pub struct Named {
        /// The type or constructor identifier.
        name: ident::Ident,

        /// Optional nonempty bracketed type arguments.
        arguments: Option<Bracketed<Punctuated<Type, punct::Comma, true, false>>>,
    }

    /// A list or fixed-size array type.
    pub struct Array {
        /// The array type's opening bracket.
        open: punct::LeftBracket,

        /// The element type, including any refinement.
        element: Box<Type>,

        /// The optional fixed-length clause.
        length: Option<LengthClause>,

        /// The matching closing bracket.
        close: punct::RightBracket,
    }

    /// A fixed array's length clause.
    pub struct LengthClause {
        /// The separator before the length.
        semicolon: punct::Semicolon,

        /// An integer occurrence or symbolic constant reference.
        value: Length,
    }

    /// A structural record type member.
    pub struct Member {
        /// The identifier or quoted member key.
        key: Key,

        /// The member's type separator.
        colon: punct::Colon,

        /// The member's syntactic type expression.
        ty: Type,
    }

    /// A closed string-backed enumeration type.
    pub struct Enumeration {
        /// The reserved `enum` occurrence.
        keyword: keyword::Enum,

        /// A nonempty list of quoted alternatives without a trailing comma.
        alternatives: Bracketed<Punctuated<string::Literal, punct::Comma, true, false>>,
    }

    /// A refinement association whose grouping remains explicit.
    pub struct Refinement {
        /// The refinement punctuation.
        at: punct::At,

        /// A named rule or parenthesized unified expression.
        rule: Association,
    }
}

/// The syntactic alternatives of an unrefined type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Atom {
    /// A name with optional constructor arguments.
    Named(Named),

    /// A list or fixed-array type.
    Array(Array),

    /// A closed structural record type.
    Record(Braced<Punctuated<Member, punct::Comma>>),

    /// A closed enumeration type.
    Enumeration(Enumeration),

    /// A grouped type expression.
    Parenthesized(Parenthesized<Box<Type>>),
}

impl Spanned for Atom {
    #[inline]
    fn span(&self) -> Span {
        match self {
            Self::Named(value) => value.span(),
            Self::Array(value) => value.span(),
            Self::Record(value) => value.span(),
            Self::Enumeration(value) => value.span(),
            Self::Parenthesized(value) => value.span(),
        }
    }
}

/// An array length before name resolution and integer representability checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Length {
    /// An unsigned integer occurrence.
    Integer(number::Integer),

    /// A constant identifier occurrence.
    Name(ident::Ident),
}

impl Spanned for Length {
    #[inline]
    fn span(&self) -> Span {
        match self {
            Self::Integer(value) => value.span(),
            Self::Name(value) => value.span(),
        }
    }
}

/// A syntactically named or explicitly grouped rule association.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Association {
    /// A bare rule identifier.
    Name(ident::Ident),

    /// A parenthesized expression whose predicate meaning is checked later.
    Inline(Parenthesized<Box<Expr>>),
}

impl Spanned for Association {
    #[inline]
    fn span(&self) -> Span {
        match self {
            Self::Name(value) => value.span(),
            Self::Inline(value) => value.span(),
        }
    }
}

/// Structural type expectations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Assert)]
#[assert(input(TokenTree))]
pub enum Fragment {
    /// A structural type atom.
    #[assert(
        matches(
            TokenTree::Ident(_)
            | TokenTree::Group(_)
            | TokenTree::Keyword(keyword::Keyword::Enum(_))
        ) as "a type expression"
    )]
    Start,

    /// A symbolic type or rule name.
    #[assert(matches(TokenTree::Ident(_)) as "an identifier")]
    Ident,

    /// A type refinement.
    #[assert(matches(TokenTree::Punctuation(punct::Punctuation::At(_))) as "`@`")]
    At,

    /// A member type separator.
    #[assert(matches(TokenTree::Punctuation(punct::Punctuation::Colon(_))) as "`:`")]
    Colon,

    /// A fixed-array length separator.
    #[assert(matches(TokenTree::Punctuation(punct::Punctuation::Semicolon(_))) as "`;`")]
    Semicolon,

    /// An enumeration introducer.
    #[assert(matches(TokenTree::Keyword(keyword::Keyword::Enum(_))) as "`enum`")]
    Enum,

    /// An integer or symbolic array length.
    #[assert(
        matches(
            TokenTree::Ident(_) | TokenTree::Number(number::Number::Integer(_))
        ) as "an unsigned integer or identifier"
    )]
    Length,
}

/// Failure in a type production.
#[derive(Debug, Clone, Report)]
#[report(forward)]
pub enum TypeError {
    /// A type was required outside a group.
    Missing(Missing),

    /// Required type syntax was absent.
    Expected(Expectation),

    /// Source lexing failed.
    Source(tree::TokenError),

    /// A structural member key failed.
    Key(KeyError),

    /// A grouped type expression failed.
    Parenthesized(super::group::GroupError<TypeError>),

    /// Array content failed.
    Array(super::group::GroupError<TypeError>),

    /// Constructor arguments failed.
    Arguments(super::group::GroupError<super::list::ListError<TypeError>>),

    /// Record members failed.
    Record(super::group::GroupError<super::list::ListError<TypeError>>),

    /// Enumeration alternatives failed.
    Enumeration(super::group::GroupError<super::list::ListError<string::syntax::StringSelectionError>>),

    /// An inline rule expression failed.
    Association(super::group::GroupError<expression::ExpressionError>),
}

impl fmt::Display for TypeError {
    #[inline]
    fn fmt(&self, writer: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(error) => fmt::Display::fmt(error, writer),
            Self::Expected(error) => fmt::Display::fmt(error, writer),
            Self::Source(error) => fmt::Display::fmt(error, writer),
            Self::Key(error) => fmt::Display::fmt(error, writer),
            Self::Parenthesized(error) => fmt::Display::fmt(error, writer),
            Self::Array(error) => fmt::Display::fmt(error, writer),
            Self::Arguments(error) => fmt::Display::fmt(error, writer),
            Self::Record(error) => fmt::Display::fmt(error, writer),
            Self::Enumeration(error) => fmt::Display::fmt(error, writer),
            Self::Association(error) => fmt::Display::fmt(error, writer),
        }
    }
}

impl Error for TypeError {
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Missing(error) => Some(error),
            Self::Expected(error) => Some(error),
            Self::Source(error) => Some(error),
            Self::Key(error) => Some(error),
            Self::Parenthesized(error) => Some(error),
            Self::Array(error) => Some(error),
            Self::Arguments(error) => Some(error),
            Self::Record(error) => Some(error),
            Self::Enumeration(error) => Some(error),
            Self::Association(error) => Some(error),
        }
    }
}

/// A type was required without an enclosing group to annotate.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("expected a type")]
#[error(source(cause))]
#[report(title = "expected a type", annotations = labels)]
// NOTE(invariant): None.
pub struct Missing {
    /// Grammar expectation and the token or boundary found at failure.
    cause: Expected<TokenTree, Fragment>,

    /// The missing type's source boundary or unexpected token.
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

/// A type component was expected at this location.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("expected type syntax")]
#[error(source(cause))]
#[report(title = "expected type syntax", annotations = labels)]
// NOTE(invariant): Related context locates the construct requiring this component.
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

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Type {
    type Context = ();
    type Error = TypeError;
    type Input<'i>
        = (Atom, Option<Refinement>)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, TypeError>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();
        let atom = Atom::lex(&mut b0)?;
        let refinement = if {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.is_next(Fragment::At))
        }
        .map_err(TypeError::Source)?
        {
            Some(Refinement::lex(&mut b0)?)
        } else {
            None
        };

        b0.backport();

        Ok((atom, refinement))
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, TypeError>
    where
        't: 'i,
    {
        let (atom, refinement) = input;

        let span = refinement
            .as_ref()
            .map_or(atom.span(), |rule| atom.span().superset(rule.span()));

        Ok(Self { atom, refinement, span })
    }
}

/// Type alternatives recognized before choosing the concrete atom node.
#[derive(Debug)]
pub enum AtomInput {
    /// A named constructor.
    Named(Named),

    /// An array type.
    Array(Array),

    /// Structural record members.
    Record(Braced<Punctuated<Member, punct::Comma>>),

    /// Closed string alternatives.
    Enumeration(Enumeration),

    /// A grouped type.
    Parenthesized(Parenthesized<Box<Type>>),
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Atom {
    type Context = ();
    type Error = TypeError;
    type Input<'i>
        = AtomInput
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, TypeError>
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
        .map_err(TypeError::Source)?;
        let input = match {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.peek())
        }
        .map_err(TypeError::Source)?
        {
            Some(TokenTree::Ident(_)) => AtomInput::Named(Named::lex(&mut b0)?),
            Some(TokenTree::Keyword(keyword::Keyword::Enum(_))) => AtomInput::Enumeration(Enumeration::lex(&mut b0)?),
            Some(TokenTree::Group(group)) => match group.delimiter() {
                Delimiter::Parenthesis(..) => AtomInput::Parenthesized(
                    Parenthesized::<Box<Type>>::lex(&mut b0).map_err(TypeError::Parenthesized)?,
                ),
                Delimiter::Bracket(..) => AtomInput::Array(Array::lex(&mut b0)?),
                Delimiter::Brace(..) => AtomInput::Record(
                    Braced::<Punctuated<Member, punct::Comma>>::lex(&mut b0).map_err(TypeError::Record)?,
                ),
            },
            _ => {
                let readiness = {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source().checked(b0.expect_is_next(Fragment::Start))
                }
                .map_err(TypeError::Source)?;

                let Err(cause) = readiness else { Ice::impossible(None) };

                let message = "expected a type name, array, record, enumeration, or grouped type";

                return Err(match b0.source().stream().scope() {
                    Some(group) => {
                        let labels = InlineAnnotations::new(
                            Label::new(message, start),
                            [Label::new("this type group starts here", group.delimiter().open())],
                        );

                        TypeError::Expected(Expectation { cause, labels })
                    },
                    None => {
                        let labels = InlineAnnotations::new(Label::new(message, start), []);

                        TypeError::Missing(Missing { cause, labels })
                    },
                });
            },
        };

        b0.backport();

        Ok(input)
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, TypeError>
    where
        't: 'i,
    {
        Ok(match input {
            AtomInput::Named(v) => Self::Named(v),
            AtomInput::Array(v) => Self::Array(v),
            AtomInput::Record(v) => Self::Record(v),
            AtomInput::Enumeration(v) => Self::Enumeration(v),
            AtomInput::Parenthesized(v) => Self::Parenthesized(v),
        })
    }
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Named {
    type Context = ();
    type Error = TypeError;
    type Input<'i>
        = (
        ident::Ident,
        Option<Bracketed<Punctuated<Type, punct::Comma, true, false>>>,
    )
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, TypeError>
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
        .map_err(TypeError::Source)?;
        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::Ident))
        }
        .map_err(TypeError::Source)?
        .map_err(|cause| {
            let target = Annotated::target(&cause);
            let labels = InlineAnnotations::new(
                Label::new("expected a type name", target),
                [Label::new("this type production starts here", start)],
            );

            TypeError::Expected(Expectation { cause, labels })
        })?;

        let TokenTree::Ident(name) = token else {
            Ice::impossible(None)
        };

        let arguments = match {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.peek())
        }
        .map_err(TypeError::Source)?
        {
            Some(TokenTree::Group(group)) if matches!(group.delimiter(), Delimiter::Bracket(..)) => Some(
                Bracketed::<Punctuated<Type, punct::Comma, true, false>>::lex(&mut b0).map_err(TypeError::Arguments)?,
            ),
            _ => None,
        };

        b0.backport();

        Ok((name, arguments))
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, TypeError>
    where
        't: 'i,
    {
        let (name, arguments) = input;

        let span = arguments
            .as_ref()
            .map_or(name.span(), |args| name.span().superset(args.span()));

        Ok(Self { name, arguments, span })
    }
}

/// The element type and optional fixed-length clause inside array brackets.
#[derive(Debug)]
// NOTE(invariant): The length clause follows the element type in the same group.
struct ArrayContent {
    /// The array element type.
    element: Box<Type>,

    /// Optional explicit fixed-length clause.
    length: Option<LengthClause>,
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for ArrayContent {
    type Context = ();
    type Error = TypeError;
    type Input<'i>
        = (Box<Type>, Option<LengthClause>)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, TypeError>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();
        let element = Box::<Type>::lex(&mut b0)?;
        let length = if {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.is_next(Fragment::Semicolon))
        }
        .map_err(TypeError::Source)?
        {
            Some(LengthClause::lex(&mut b0)?)
        } else {
            None
        };

        b0.backport();

        Ok((element, length))
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, TypeError>
    where
        't: 'i,
    {
        let (element, length) = input;

        Ok(Self { element, length })
    }
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Array {
    type Context = ();
    type Error = TypeError;
    type Input<'i>
        = (punct::LeftBracket, Box<Type>, Option<LengthClause>, punct::RightBracket)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, TypeError>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();
        let group = Bracketed::<ArrayContent>::lex(&mut b0).map_err(TypeError::Array)?;

        let (open, ArrayContent { element, length }, close) = group.parts();

        b0.backport();

        Ok((open, element, length, close))
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, TypeError>
    where
        't: 'i,
    {
        let (open, element, length, close) = input;

        let span = open.span().superset(close.span());

        Ok(Self {
            open,
            element,
            length,
            close,
            span,
        })
    }
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for LengthClause {
    type Context = ();
    type Error = TypeError;
    type Input<'i>
        = (punct::Semicolon, Length)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, TypeError>
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
        .map_err(TypeError::Source)?;
        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::Semicolon))
        }
        .map_err(TypeError::Source)?
        .map_err(|cause| {
            let target = Annotated::target(&cause);
            let labels = InlineAnnotations::new(
                Label::new("expected `;` before an array length", target),
                [Label::new("this type production starts here", start)],
            );

            TypeError::Expected(Expectation { cause, labels })
        })?;

        let TokenTree::Punctuation(punct::Punctuation::Semicolon(semicolon)) = token else {
            Ice::impossible(None)
        };

        let value = Length::lex(&mut b0)?;

        b0.backport();

        Ok((semicolon, value))
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, TypeError>
    where
        't: 'i,
    {
        let (semicolon, value) = input;

        let span = semicolon.span().superset(value.span());

        Ok(Self { semicolon, value, span })
    }
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Length {
    type Context = ();
    type Error = TypeError;
    type Input<'i>
        = TokenTree
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, TypeError>
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
        .map_err(TypeError::Source)?;
        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::Length))
        }
        .map_err(TypeError::Source)?
        .map_err(|cause| {
            let target = Annotated::target(&cause);
            let labels = InlineAnnotations::new(
                Label::new(
                    "array length must be an unsigned integer or symbolic identifier",
                    target,
                ),
                [Label::new("this type production starts here", start)],
            );

            TypeError::Expected(Expectation { cause, labels })
        })?;

        b0.backport();

        Ok(token)
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, TypeError>
    where
        't: 'i,
    {
        Ok(match input {
            TokenTree::Ident(v) => Self::Name(v),
            TokenTree::Number(number::Number::Integer(v)) => Self::Integer(v),
            _ => Ice::impossible(None),
        })
    }
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Member {
    type Context = ();
    type Error = TypeError;
    type Input<'i>
        = (Key, punct::Colon, Type)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, TypeError>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();
        let key = Key::lex(&mut b0).map_err(TypeError::Key)?;
        let start = key.span();
        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::Colon))
        }
        .map_err(TypeError::Source)?
        .map_err(|cause| {
            let target = Annotated::target(&cause);
            let labels = InlineAnnotations::new(
                Label::new("type members require `key: type`", target),
                [Label::new("this type production starts here", start)],
            );

            TypeError::Expected(Expectation { cause, labels })
        })?;

        let TokenTree::Punctuation(punct::Punctuation::Colon(colon)) = token else {
            Ice::impossible(None)
        };

        let ty = Type::lex(&mut b0)?;

        b0.backport();

        Ok((key, colon, ty))
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, TypeError>
    where
        't: 'i,
    {
        let (key, colon, ty) = input;

        let span = key.span().superset(ty.span());

        Ok(Self { key, colon, ty, span })
    }
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Enumeration {
    type Context = ();
    type Error = TypeError;
    type Input<'i>
        = (
        keyword::Enum,
        Bracketed<Punctuated<string::Literal, punct::Comma, true, false>>,
    )
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, TypeError>
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
        .map_err(TypeError::Source)?;
        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::Enum))
        }
        .map_err(TypeError::Source)?
        .map_err(|cause| {
            let target = Annotated::target(&cause);
            let labels = InlineAnnotations::new(
                Label::new("expected `enum`", target),
                [Label::new("this type production starts here", start)],
            );

            TypeError::Expected(Expectation { cause, labels })
        })?;

        let TokenTree::Keyword(keyword::Keyword::Enum(keyword)) = token else {
            Ice::impossible(None)
        };

        let alternatives = Bracketed::<Punctuated<string::Literal, punct::Comma, true, false>>::lex(&mut b0)
            .map_err(TypeError::Enumeration)?;

        b0.backport();

        Ok((keyword, alternatives))
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, TypeError>
    where
        't: 'i,
    {
        let (keyword, alternatives) = input;

        let span = keyword.span().superset(alternatives.span());

        Ok(Self {
            keyword,
            alternatives,
            span,
        })
    }
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Refinement {
    type Context = ();
    type Error = TypeError;
    type Input<'i>
        = (punct::At, Association)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, TypeError>
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
        .map_err(TypeError::Source)?;
        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::At))
        }
        .map_err(TypeError::Source)?
        .map_err(|cause| {
            let target = Annotated::target(&cause);
            let labels = InlineAnnotations::new(
                Label::new("expected `@` before a type rule", target),
                [Label::new("this type production starts here", start)],
            );

            TypeError::Expected(Expectation { cause, labels })
        })?;

        let TokenTree::Punctuation(punct::Punctuation::At(at)) = token else {
            Ice::impossible(None)
        };

        let rule = Association::lex(&mut b0)?;

        b0.backport();

        Ok((at, rule))
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, TypeError>
    where
        't: 'i,
    {
        let (at, rule) = input;

        let span = at.span().superset(rule.span());

        Ok(Self { at, rule, span })
    }
}

/// A rule-name occurrence or explicitly grouped expression.
#[derive(Debug)]
pub enum AssociationInput {
    /// A named rule.
    Name(ident::Ident),

    /// An inline rule expression.
    Inline(Parenthesized<Box<Expr>>),
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Association {
    type Context = ();
    type Error = TypeError;
    type Input<'i>
        = AssociationInput
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, TypeError>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();
        let input = match {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.peek())
        }
        .map_err(TypeError::Source)?
        {
            Some(TokenTree::Group(group)) if matches!(group.delimiter(), Delimiter::Parenthesis(..)) => {
                AssociationInput::Inline(Parenthesized::<Box<Expr>>::lex(&mut b0).map_err(TypeError::Association)?)
            },
            _ => {
                let start = {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source()
                        .checked(b0.peek().map_or(Span::unit(b0.index()), |token| token.span()))
                }
                .map_err(TypeError::Source)?;
                let token = {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source().checked(b0.expect_is_next(Fragment::Ident))
                }
                .map_err(TypeError::Source)?
                .map_err(|cause| {
                    let target = Annotated::target(&cause);
                    let labels = InlineAnnotations::new(
                        Label::new("expected a rule name or a parenthesized expression", target),
                        [Label::new("this type production starts here", start)],
                    );

                    TypeError::Expected(Expectation { cause, labels })
                })?;

                let TokenTree::Ident(name) = token else {
                    Ice::impossible(None)
                };

                AssociationInput::Name(name)
            },
        };

        b0.backport();

        Ok(input)
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, TypeError>
    where
        't: 'i,
    {
        Ok(match input {
            AssociationInput::Name(v) => Self::Name(v),
            AssociationInput::Inline(v) => Self::Inline(v),
        })
    }
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Box<Type> {
    type Context = ();
    type Error = TypeError;
    type Input<'i>
        = <Type as Lex<'t, &'t Tokens<'s>>>::Input<'i>
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, TypeError>
    where
        't: 'i,
    {
        Type::pass(c0)
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, TypeError>
    where
        't: 'i,
    {
        Type::build(input).map(Box::new)
    }
}

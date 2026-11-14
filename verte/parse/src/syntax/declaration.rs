//! Declarations, data keys, and intrinsic directives before semantic checking.

use core::fmt;

use aldebaran::{
    ice::Ice,
    prelude::{Annotated, Assert, Lex, Report, Span, Spanned},
    report::annotated::{InlineAnnotations, label::Label},
    text::cursor::error::Expected,
};
use fack::prelude::Error;

use super::{
    Parenthesized,
    expression::{self, Expr, Record},
    types::{self, Type},
};
use crate::scan::{
    ident, keyword, punct, string,
    tree::{self, SyntaxCursor, TokenTree, source::Tokens},
};

/// Concrete alternatives of the Key production.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    /// A non-reserved identifier occurrence.
    Name(ident::Ident),

    /// A quoted key, including keys whose decoded value is a reserved word.
    Quoted(string::Literal),
}

impl Spanned for Key {
    #[inline]
    fn span(&self) -> Span {
        match self {
            Self::Name(value) => value.span(),
            Self::Quoted(value) => value.span(),
        }
    }
}

/// Concrete alternatives of the FieldPolicy production.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldPolicy {
    /// Child-preferred recursive composition.
    Merge(keyword::Merge),

    /// Parent-preferred recursive composition.
    Join(keyword::Join),

    /// Child-preferred composition with array concatenation.
    Admerge(keyword::Admerge),

    /// Parent-preferred composition with array concatenation.
    Adjoin(keyword::Adjoin),
}

impl Spanned for FieldPolicy {
    #[inline]
    fn span(&self) -> Span {
        match self {
            Self::Merge(value) => value.span(),
            Self::Join(value) => value.span(),
            Self::Admerge(value) => value.span(),
            Self::Adjoin(value) => value.span(),
        }
    }
}

/// Concrete alternatives of the InheritancePolicy production.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InheritancePolicy {
    /// A policy also available for individual fields.
    Field(FieldPolicy),

    /// An explicit prohibition of parent configuration composition.
    Forbid(keyword::Forbid),
}

impl Spanned for InheritancePolicy {
    #[inline]
    fn span(&self) -> Span {
        match self {
            Self::Field(value) => value.span(),
            Self::Forbid(value) => value.span(),
        }
    }
}

/// Concrete alternatives of the Field production.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Field {
    /// A field without a policy wrapper.
    Plain(Binding),

    /// A field with explicit policy and parentheses.
    Policy(PolicyField),
}

impl Spanned for Field {
    #[inline]
    fn span(&self) -> Span {
        match self {
            Self::Plain(v) => v.span(),
            Self::Policy(v) => v.span(),
        }
    }
}

/// Concrete alternatives of the Statement production.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    /// Include definitions from a referenced source.
    Include(Include),

    /// Extend a referenced parent configuration.
    Extend(Extend),

    /// Select document inheritance behavior.
    Inherit(Inherit),

    /// Declare a nominal domain.
    Domain(Domain),

    /// Declare a transparent type alias.
    Type(TypeAlias),

    /// Declare a typed immutable constant.
    Constant(Constant),

    /// Declare a reusable rule expression.
    Rule(Rule),

    /// Initialize a root configuration field.
    Field(Field),
}

impl Spanned for Statement {
    #[inline]
    fn span(&self) -> Span {
        match self {
            Self::Include(value) => value.span(),
            Self::Extend(value) => value.span(),
            Self::Inherit(value) => value.span(),
            Self::Domain(value) => value.span(),
            Self::Type(value) => value.span(),
            Self::Constant(value) => value.span(),
            Self::Rule(value) => value.span(),
            Self::Field(value) => value.span(),
        }
    }
}
node! {
    /// A typed field binding, without an optional policy wrapper.
    pub struct Binding {
        /// The field's identifier or quoted data key.
        key: Key,

        /// The explicit type separator.
        colon: punct::Colon,

        /// The field's syntactic type.
        ty: Type,

        /// The initializer separator.
        equals: punct::Eq,

        /// The field's initializer expression.
        value: Expr,
    }

    /// A field whose composition policy is explicitly wrapped around its binding.
    pub struct PolicyField {
        /// The field-level policy keyword.
        policy: FieldPolicy,

        /// The parenthesized typed binding.
        binding: Parenthesized<Binding>,
    }

    /// A source-definition inclusion directive.
    pub struct Include {
        /// The reserved directive keyword.
        keyword: keyword::Include,

        /// The single source-reference expression and its parentheses.
        argument: Parenthesized<SourceArgument>,
    }

    /// A parent configuration directive.
    pub struct Extend {
        /// The reserved directive keyword.
        keyword: keyword::Extend,

        /// The single source-reference expression and its parentheses.
        argument: Parenthesized<SourceArgument>,
    }

    /// A document inheritance policy directive.
    pub struct Inherit {
        /// The reserved directive keyword.
        keyword: keyword::Inherit,

        /// The dedicated policy keyword and its parentheses.
        argument: Parenthesized<InheritancePolicy>,
    }

    /// A nominal domain declaration before category and property checking.
    pub struct Domain {
        /// The reserved declaration keyword.
        keyword: keyword::Domain,

        /// The newly declared domain identifier.
        name: ident::Ident,

        /// The category separator.
        colon: punct::Colon,

        /// The domain category identifier.
        category: ident::Ident,

        /// The properties initializer separator.
        equals: punct::Eq,

        /// The syntactic properties record.
        properties: Record,
    }

    /// A transparent type alias declaration.
    pub struct TypeAlias {
        /// The reserved declaration keyword.
        keyword: keyword::Type,

        /// The newly declared type identifier.
        name: ident::Ident,

        /// The alias definition separator.
        equals: punct::Eq,

        /// The alias's syntactic type expression.
        value: Type,
    }

    /// An explicitly typed immutable constant declaration.
    pub struct Constant {
        /// The reserved declaration keyword.
        keyword: keyword::Const,

        /// The newly declared constant identifier.
        name: ident::Ident,

        /// The explicit type separator.
        colon: punct::Colon,

        /// The constant's declared type.
        ty: Type,

        /// The initializer separator.
        equals: punct::Eq,

        /// The unified initializer expression.
        value: Expr,
    }

    /// A named reusable predicate declaration.
    pub struct Rule {
        /// The reserved declaration keyword.
        keyword: keyword::Rule,

        /// The newly declared rule identifier.
        name: ident::Ident,

        /// The initializer separator.
        equals: punct::Eq,

        /// The unified expression, before predicate typing.
        value: Expr,
    }
}

/// Expectations owned by declaration productions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Assert)]
#[assert(input(TokenTree))]
pub enum Fragment {
    /// An identifier or quoted data key.
    #[assert(matches(TokenTree::Ident(_) | TokenTree::String(_)) as "an identifier or quoted key")]
    Key,

    /// A symbolic declaration name.
    #[assert(matches(TokenTree::Ident(_)) as "an identifier")]
    Ident,

    /// A type separator.
    #[assert(matches(TokenTree::Punctuation(punct::Punctuation::Colon(_))) as "`:`")]
    Colon,

    /// An initializer separator.
    #[assert(matches(TokenTree::Punctuation(punct::Punctuation::Eq(_))) as "`=`")]
    Equals,

    /// A field composition policy.
    #[assert(
        matches(
            TokenTree::Keyword(
                keyword::Keyword::Merge(_)
                | keyword::Keyword::Join(_)
                | keyword::Keyword::Admerge(_)
                | keyword::Keyword::Adjoin(_),
            )
        ) as "a field policy keyword"
    )]
    FieldPolicy,

    /// A document inheritance policy.
    #[assert(
        matches(
            TokenTree::Keyword(
                keyword::Keyword::Merge(_)
                | keyword::Keyword::Join(_)
                | keyword::Keyword::Admerge(_)
                | keyword::Keyword::Adjoin(_)
                | keyword::Keyword::Forbid(_),
            )
        ) as "an inheritance policy keyword"
    )]
    InheritancePolicy,

    /// The `include` introducer.
    #[assert(matches(TokenTree::Keyword(keyword::Keyword::Include(_))) as "`include`")]
    Include,

    /// The `extend` introducer.
    #[assert(matches(TokenTree::Keyword(keyword::Keyword::Extend(_))) as "`extend`")]
    Extend,

    /// The `inherit` introducer.
    #[assert(matches(TokenTree::Keyword(keyword::Keyword::Inherit(_))) as "`inherit`")]
    Inherit,

    /// The `domain` introducer.
    #[assert(matches(TokenTree::Keyword(keyword::Keyword::Domain(_))) as "`domain`")]
    Domain,

    /// The `type` introducer.
    #[assert(matches(TokenTree::Keyword(keyword::Keyword::Type(_))) as "`type`")]
    Type,

    /// The `const` introducer.
    #[assert(matches(TokenTree::Keyword(keyword::Keyword::Const(_))) as "`const`")]
    Const,

    /// The `rule` introducer.
    #[assert(matches(TokenTree::Keyword(keyword::Keyword::Rule(_))) as "`rule`")]
    Rule,
}

/// Failure of an identifier or quoted data key.
#[derive(Debug, Clone, Report)]
#[report(forward)]
pub enum KeyError {
    /// A data key was missing.
    Expected(KeyExpectation),

    /// Source lexing failed.
    Source(tree::TokenError),
}

impl fmt::Display for KeyError {
    #[inline]
    fn fmt(&self, writer: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expected(error) => fmt::Display::fmt(error, writer),
            Self::Source(error) => fmt::Display::fmt(error, writer),
        }
    }
}

impl Error for KeyError {
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Expected(error) => Some(error),
            Self::Source(error) => Some(error),
        }
    }
}

/// A data key was required here.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("expected a data key")]
#[error(source(cause))]
#[report(title = "expected a data key", annotations = labels)]
// NOTE(invariant): None.
pub struct KeyExpectation {
    /// Grammar expectation and the token or boundary found at failure.
    cause: Expected<TokenTree, Fragment>,

    /// Primary failure location followed by related source context.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

impl KeyExpectation {
    /// Retrieve the syntax fragment expected at the failure location.
    #[inline]
    pub const fn cause(&self) -> &Expected<TokenTree, Fragment> {
        let Self { cause, .. } = self;

        cause
    }
}

/// Failure of a declaration or intrinsic directive.
#[derive(Debug, Clone, Report)]
#[report(forward)]
pub enum DeclarationError {
    /// The initializer is missing after its assignment token.
    Initializer(Initializer),

    /// Required declaration syntax was absent.
    Expected(Expectation),

    /// Source lexing failed.
    Source(tree::TokenError),

    /// A declared key was invalid.
    Key(KeyError),

    /// A type production failed.
    Type(types::TypeError),

    /// An initializer failed.
    Expression(expression::ExpressionError),

    /// A field-policy wrapper failed.
    BindingGroup(super::group::GroupError<DeclarationError>),

    /// A source directive's argument group failed.
    ArgumentGroup(super::group::GroupError<DeclarationError>),

    /// An inheritance policy group failed.
    PolicyGroup(super::group::GroupError<DeclarationError>),

    /// Domain properties failed.
    RecordGroup(super::group::GroupError<super::list::ListError<expression::ExpressionError>>),

    /// A directive did not contain exactly one argument.
    Arity(Arity),
}

impl fmt::Display for DeclarationError {
    #[inline]
    fn fmt(&self, writer: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Initializer(error) => fmt::Display::fmt(error, writer),
            Self::Expected(error) => fmt::Display::fmt(error, writer),
            Self::Source(error) => fmt::Display::fmt(error, writer),
            Self::Key(error) => fmt::Display::fmt(error, writer),
            Self::Type(error) => fmt::Display::fmt(error, writer),
            Self::Expression(error) => fmt::Display::fmt(error, writer),
            Self::BindingGroup(error) => fmt::Display::fmt(error, writer),
            Self::ArgumentGroup(error) => fmt::Display::fmt(error, writer),
            Self::PolicyGroup(error) => fmt::Display::fmt(error, writer),
            Self::RecordGroup(error) => fmt::Display::fmt(error, writer),
            Self::Arity(error) => fmt::Display::fmt(error, writer),
        }
    }
}

impl Error for DeclarationError {
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Initializer(error) => Some(error),
            Self::Expected(error) => Some(error),
            Self::Source(error) => Some(error),
            Self::Key(error) => Some(error),
            Self::Type(error) => Some(error),
            Self::Expression(error) => Some(error),
            Self::BindingGroup(error) => Some(error),
            Self::ArgumentGroup(error) => Some(error),
            Self::PolicyGroup(error) => Some(error),
            Self::RecordGroup(error) => Some(error),
            Self::Arity(error) => Some(error),
        }
    }
}

/// A declaration component was expected.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("expected declaration syntax")]
#[error(source(cause))]
#[report(title = "expected declaration syntax", annotations = labels)]
// NOTE(invariant): The related annotation locates the declaration requiring this component.
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

/// An assignment token was not followed by an expression.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("missing initializer expression")]
#[error(source(cause))]
#[report(title = "missing initializer expression", annotations = labels)]
// NOTE(invariant): The related label identifies the assignment requiring this expression.
pub struct Initializer {
    /// Grammar expectation and the token or boundary found at failure.
    cause: Expected<TokenTree, expression::Fragment>,

    /// The missing expression and its assignment occurrence.
    labels: InlineAnnotations<Label<&'static str>, 1>,
}

impl Initializer {
    /// Retrieve the syntax fragment expected at the failure location.
    #[inline]
    pub const fn cause(&self) -> &Expected<TokenTree, expression::Fragment> {
        let Self { cause, .. } = self;

        cause
    }
}

/// A source directive's argument count was invalid.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("invalid directive argument count")]
#[report(title = "invalid directive argument count", annotations = labels)]
// NOTE(invariant): Related context identifies the accepted argument or its missing boundary.
pub struct Arity {
    /// Primary failure location followed by related source context.
    labels: InlineAnnotations<Label<&'static str>, 1>,
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Key {
    type Context = ();
    type Error = KeyError;
    type Input<'i>
        = TokenTree
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, KeyError>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();
        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::Key))
        }
        .map_err(KeyError::Source)?
        .map_err(|cause| {
            let labels = InlineAnnotations::new(
                Label::new(
                    "expected an identifier or quoted string as the key",
                    Annotated::target(&cause),
                ),
                [],
            );

            KeyError::Expected(KeyExpectation { cause, labels })
        })?;

        b0.backport();

        Ok(token)
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, KeyError>
    where
        't: 'i,
    {
        Ok(match input {
            TokenTree::Ident(v) => Self::Name(v),
            TokenTree::String(v) => Self::Quoted(v),
            _ => Ice::impossible(None),
        })
    }
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for FieldPolicy {
    type Context = ();
    type Error = DeclarationError;
    type Input<'i>
        = TokenTree
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, DeclarationError>
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
        .map_err(DeclarationError::Source)?;
        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::FieldPolicy))
        }
        .map_err(DeclarationError::Source)?
        .map_err(|cause| {
            let target = Annotated::target(&cause);
            let labels = InlineAnnotations::new(
                Label::new("expected an unquoted policy keyword", target),
                [Label::new("this declaration starts here", start)],
            );

            DeclarationError::Expected(Expectation { cause, labels })
        })?;

        b0.backport();

        Ok(token)
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, DeclarationError>
    where
        't: 'i,
    {
        Ok(match input {
            TokenTree::Keyword(keyword::Keyword::Merge(v)) => Self::Merge(v),
            TokenTree::Keyword(keyword::Keyword::Join(v)) => Self::Join(v),
            TokenTree::Keyword(keyword::Keyword::Admerge(v)) => Self::Admerge(v),
            TokenTree::Keyword(keyword::Keyword::Adjoin(v)) => Self::Adjoin(v),
            _ => Ice::impossible(None),
        })
    }
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for InheritancePolicy {
    type Context = ();
    type Error = DeclarationError;
    type Input<'i>
        = TokenTree
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, DeclarationError>
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
        .map_err(DeclarationError::Source)?;
        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::InheritancePolicy))
        }
        .map_err(DeclarationError::Source)?
        .map_err(|cause| {
            let target = Annotated::target(&cause);
            let labels = InlineAnnotations::new(
                Label::new("expected an unquoted policy keyword", target),
                [Label::new("this declaration starts here", start)],
            );

            DeclarationError::Expected(Expectation { cause, labels })
        })?;

        b0.backport();

        Ok(token)
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, DeclarationError>
    where
        't: 'i,
    {
        Ok(match input {
            TokenTree::Keyword(keyword::Keyword::Merge(v)) => Self::Field(FieldPolicy::Merge(v)),
            TokenTree::Keyword(keyword::Keyword::Join(v)) => Self::Field(FieldPolicy::Join(v)),
            TokenTree::Keyword(keyword::Keyword::Admerge(v)) => Self::Field(FieldPolicy::Admerge(v)),
            TokenTree::Keyword(keyword::Keyword::Adjoin(v)) => Self::Field(FieldPolicy::Adjoin(v)),
            TokenTree::Keyword(keyword::Keyword::Forbid(v)) => Self::Forbid(v),
            _ => Ice::impossible(None),
        })
    }
}

/// The single source-reference expression accepted by an include or extend directive.
#[derive(Debug, Clone, PartialEq, Eq)]
// NOTE(invariant): The directive argument contains exactly one expression.
pub struct SourceArgument {
    /// Syntax associated with this source occurrence.
    value: Expr,
}

impl SourceArgument {
    /// Retrieve the source-reference expression.
    #[inline]
    pub const fn value(&self) -> &Expr {
        let Self { value } = self;

        value
    }
}

impl Spanned for SourceArgument {
    #[inline]
    fn span(&self) -> Span {
        let Self { value } = self;

        value.span()
    }
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for SourceArgument {
    type Context = ();
    type Error = DeclarationError;
    type Input<'i>
        = Expr
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, DeclarationError>
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
        .map_err(DeclarationError::Source)?;

        if {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.peek())
        }
        .map_err(DeclarationError::Source)?
        .is_none()
        {
            let labels = InlineAnnotations::new(
                Label::new("this source directive requires exactly one expression", start),
                [Label::new(
                    "this source argument group starts here",
                    b0.source()
                        .stream()
                        .scope()
                        .map_or(start, |group| group.delimiter().open()),
                )],
            );

            return Err(DeclarationError::Arity(Arity { labels }));
        }

        let value = Expr::lex(&mut b0).map_err(DeclarationError::Expression)?;

        if let Some(token) = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.peek())
        }
        .map_err(DeclarationError::Source)?
        {
            let labels = InlineAnnotations::new(
                Label::new("source directives accept exactly one expression", token.span()),
                [Label::new("the accepted argument is here", value.span())],
            );

            return Err(DeclarationError::Arity(Arity { labels }));
        }

        b0.backport();

        Ok(value)
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, DeclarationError>
    where
        't: 'i,
    {
        Ok(Self { value: input })
    }
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Binding {
    type Context = ();
    type Error = DeclarationError;
    type Input<'i>
        = (Key, punct::Colon, Type, punct::Eq, Expr)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, DeclarationError>
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
        .map_err(DeclarationError::Source)?;
        let key = Key::lex(&mut b0).map_err(DeclarationError::Key)?;
        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::Colon))
        }
        .map_err(DeclarationError::Source)?
        .map_err(|cause| {
            let target = Annotated::target(&cause);
            let labels = InlineAnnotations::new(
                Label::new("fields and constants require an explicit `: type`", target),
                [Label::new("this declaration starts here", start)],
            );

            DeclarationError::Expected(Expectation { cause, labels })
        })?;

        let TokenTree::Punctuation(punct::Punctuation::Colon(colon)) = token else {
            Ice::impossible(None)
        };

        let ty = Type::lex(&mut b0).map_err(DeclarationError::Type)?;
        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::Equals))
        }
        .map_err(DeclarationError::Source)?
        .map_err(|cause| {
            let target = Annotated::target(&cause);
            let labels = InlineAnnotations::new(
                Label::new("expected `=` before the definition", target),
                [Label::new("this declaration starts here", start)],
            );

            DeclarationError::Expected(Expectation { cause, labels })
        })?;

        let TokenTree::Punctuation(punct::Punctuation::Eq(equals)) = token else {
            Ice::impossible(None)
        };

        let readiness = {
            let mut b1 = b0.branch();
            let trivia = b1.source().trivia();

            b1.ignore_while(trivia);

            b1.source().checked(b1.expect_is_next(expression::Fragment::Start))
        }
        .map_err(DeclarationError::Source)?;

        if let Err(cause) = readiness {
            let target = Annotated::target(&cause);
            let labels = InlineAnnotations::new(
                Label::new("expected an initializer expression after `=`", target),
                [Label::new("this initializer starts here", equals.span())],
            );

            return Err(DeclarationError::Initializer(Initializer { cause, labels }));
        }

        let value = Expr::lex(&mut b0).map_err(DeclarationError::Expression)?;

        b0.backport();

        Ok((key, colon, ty, equals, value))
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, DeclarationError>
    where
        't: 'i,
    {
        let (key, colon, ty, equals, value) = input;

        let span = key.span().superset(value.span());

        Ok(Self {
            key,
            colon,
            ty,
            equals,
            value,
            span,
        })
    }
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for PolicyField {
    type Context = ();
    type Error = DeclarationError;
    type Input<'i>
        = (FieldPolicy, Parenthesized<Binding>)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, DeclarationError>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();
        let policy = FieldPolicy::lex(&mut b0)?;
        let binding = Parenthesized::<Binding>::lex(&mut b0).map_err(DeclarationError::BindingGroup)?;

        b0.backport();

        Ok((policy, binding))
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, DeclarationError>
    where
        't: 'i,
    {
        let (policy, binding) = input;

        let span = policy.span().superset(binding.span());

        Ok(Self { policy, binding, span })
    }
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Include {
    type Context = ();
    type Error = DeclarationError;
    type Input<'i>
        = (keyword::Include, Parenthesized<SourceArgument>)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, DeclarationError>
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
        .map_err(DeclarationError::Source)?;
        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::Include))
        }
        .map_err(DeclarationError::Source)?
        .map_err(|cause| {
            let target = Annotated::target(&cause);
            let labels = InlineAnnotations::new(
                Label::new("expected `include`", target),
                [Label::new("this declaration starts here", start)],
            );

            DeclarationError::Expected(Expectation { cause, labels })
        })?;

        let TokenTree::Keyword(keyword::Keyword::Include(keyword)) = token else {
            Ice::impossible(None)
        };

        let argument = Parenthesized::<SourceArgument>::lex(&mut b0).map_err(DeclarationError::ArgumentGroup)?;

        b0.backport();

        Ok((keyword, argument))
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, DeclarationError>
    where
        't: 'i,
    {
        let (keyword, argument) = input;

        let span = keyword.span().superset(argument.span());

        Ok(Self {
            keyword,
            argument,
            span,
        })
    }
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Extend {
    type Context = ();
    type Error = DeclarationError;
    type Input<'i>
        = (keyword::Extend, Parenthesized<SourceArgument>)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, DeclarationError>
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
        .map_err(DeclarationError::Source)?;
        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::Extend))
        }
        .map_err(DeclarationError::Source)?
        .map_err(|cause| {
            let target = Annotated::target(&cause);
            let labels = InlineAnnotations::new(
                Label::new("expected `extend`", target),
                [Label::new("this declaration starts here", start)],
            );

            DeclarationError::Expected(Expectation { cause, labels })
        })?;

        let TokenTree::Keyword(keyword::Keyword::Extend(keyword)) = token else {
            Ice::impossible(None)
        };

        let argument = Parenthesized::<SourceArgument>::lex(&mut b0).map_err(DeclarationError::ArgumentGroup)?;

        b0.backport();

        Ok((keyword, argument))
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, DeclarationError>
    where
        't: 'i,
    {
        let (keyword, argument) = input;

        let span = keyword.span().superset(argument.span());

        Ok(Self {
            keyword,
            argument,
            span,
        })
    }
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Inherit {
    type Context = ();
    type Error = DeclarationError;
    type Input<'i>
        = (keyword::Inherit, Parenthesized<InheritancePolicy>)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, DeclarationError>
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
        .map_err(DeclarationError::Source)?;
        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::Inherit))
        }
        .map_err(DeclarationError::Source)?
        .map_err(|cause| {
            let target = Annotated::target(&cause);
            let labels = InlineAnnotations::new(
                Label::new("expected `inherit`", target),
                [Label::new("this declaration starts here", start)],
            );

            DeclarationError::Expected(Expectation { cause, labels })
        })?;

        let TokenTree::Keyword(keyword::Keyword::Inherit(keyword)) = token else {
            Ice::impossible(None)
        };

        let argument = Parenthesized::<InheritancePolicy>::lex(&mut b0).map_err(DeclarationError::PolicyGroup)?;

        b0.backport();

        Ok((keyword, argument))
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, DeclarationError>
    where
        't: 'i,
    {
        let (keyword, argument) = input;

        let span = keyword.span().superset(argument.span());

        Ok(Self {
            keyword,
            argument,
            span,
        })
    }
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Domain {
    type Context = ();
    type Error = DeclarationError;
    type Input<'i>
        = (
        keyword::Domain,
        ident::Ident,
        punct::Colon,
        ident::Ident,
        punct::Eq,
        Record,
    )
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, DeclarationError>
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
        .map_err(DeclarationError::Source)?;
        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::Domain))
        }
        .map_err(DeclarationError::Source)?
        .map_err(|cause| {
            let target = Annotated::target(&cause);
            let labels = InlineAnnotations::new(
                Label::new("expected `domain`", target),
                [Label::new("this declaration starts here", start)],
            );

            DeclarationError::Expected(Expectation { cause, labels })
        })?;

        let TokenTree::Keyword(keyword::Keyword::Domain(keyword)) = token else {
            Ice::impossible(None)
        };

        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::Ident))
        }
        .map_err(DeclarationError::Source)?
        .map_err(|cause| {
            let target = Annotated::target(&cause);
            let labels = InlineAnnotations::new(
                Label::new("expected a non-reserved symbolic name", target),
                [Label::new("this declaration starts here", start)],
            );

            DeclarationError::Expected(Expectation { cause, labels })
        })?;

        let TokenTree::Ident(name) = token else {
            Ice::impossible(None)
        };

        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::Colon))
        }
        .map_err(DeclarationError::Source)?
        .map_err(|cause| {
            let target = Annotated::target(&cause);
            let labels = InlineAnnotations::new(
                Label::new("fields and constants require an explicit `: type`", target),
                [Label::new("this declaration starts here", start)],
            );

            DeclarationError::Expected(Expectation { cause, labels })
        })?;

        let TokenTree::Punctuation(punct::Punctuation::Colon(colon)) = token else {
            Ice::impossible(None)
        };

        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::Ident))
        }
        .map_err(DeclarationError::Source)?
        .map_err(|cause| {
            let target = Annotated::target(&cause);
            let labels = InlineAnnotations::new(
                Label::new("expected a non-reserved symbolic name", target),
                [Label::new("this declaration starts here", start)],
            );

            DeclarationError::Expected(Expectation { cause, labels })
        })?;

        let TokenTree::Ident(category) = token else {
            Ice::impossible(None)
        };

        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::Equals))
        }
        .map_err(DeclarationError::Source)?
        .map_err(|cause| {
            let target = Annotated::target(&cause);
            let labels = InlineAnnotations::new(
                Label::new("expected `=` before the definition", target),
                [Label::new("this declaration starts here", start)],
            );

            DeclarationError::Expected(Expectation { cause, labels })
        })?;

        let TokenTree::Punctuation(punct::Punctuation::Eq(equals)) = token else {
            Ice::impossible(None)
        };

        let properties = Record::lex(&mut b0).map_err(DeclarationError::RecordGroup)?;

        b0.backport();

        Ok((keyword, name, colon, category, equals, properties))
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, DeclarationError>
    where
        't: 'i,
    {
        let (keyword, name, colon, category, equals, properties) = input;

        let span = keyword.span().superset(properties.span());

        Ok(Self {
            keyword,
            name,
            colon,
            category,
            equals,
            properties,
            span,
        })
    }
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for TypeAlias {
    type Context = ();
    type Error = DeclarationError;
    type Input<'i>
        = (keyword::Type, ident::Ident, punct::Eq, Type)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, DeclarationError>
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
        .map_err(DeclarationError::Source)?;
        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::Type))
        }
        .map_err(DeclarationError::Source)?
        .map_err(|cause| {
            let target = Annotated::target(&cause);
            let labels = InlineAnnotations::new(
                Label::new("expected `type`", target),
                [Label::new("this declaration starts here", start)],
            );

            DeclarationError::Expected(Expectation { cause, labels })
        })?;

        let TokenTree::Keyword(keyword::Keyword::Type(keyword)) = token else {
            Ice::impossible(None)
        };

        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::Ident))
        }
        .map_err(DeclarationError::Source)?
        .map_err(|cause| {
            let target = Annotated::target(&cause);
            let labels = InlineAnnotations::new(
                Label::new("expected a non-reserved symbolic name", target),
                [Label::new("this declaration starts here", start)],
            );

            DeclarationError::Expected(Expectation { cause, labels })
        })?;

        let TokenTree::Ident(name) = token else {
            Ice::impossible(None)
        };

        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::Equals))
        }
        .map_err(DeclarationError::Source)?
        .map_err(|cause| {
            let target = Annotated::target(&cause);
            let labels = InlineAnnotations::new(
                Label::new("expected `=` before the definition", target),
                [Label::new("this declaration starts here", start)],
            );

            DeclarationError::Expected(Expectation { cause, labels })
        })?;

        let TokenTree::Punctuation(punct::Punctuation::Eq(equals)) = token else {
            Ice::impossible(None)
        };

        let value = Type::lex(&mut b0).map_err(DeclarationError::Type)?;

        b0.backport();

        Ok((keyword, name, equals, value))
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, DeclarationError>
    where
        't: 'i,
    {
        let (keyword, name, equals, value) = input;

        let span = keyword.span().superset(value.span());

        Ok(Self {
            keyword,
            name,
            equals,
            value,
            span,
        })
    }
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Constant {
    type Context = ();
    type Error = DeclarationError;
    type Input<'i>
        = (keyword::Const, ident::Ident, punct::Colon, Type, punct::Eq, Expr)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, DeclarationError>
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
        .map_err(DeclarationError::Source)?;
        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::Const))
        }
        .map_err(DeclarationError::Source)?
        .map_err(|cause| {
            let target = Annotated::target(&cause);
            let labels = InlineAnnotations::new(
                Label::new("expected `const`", target),
                [Label::new("this declaration starts here", start)],
            );

            DeclarationError::Expected(Expectation { cause, labels })
        })?;

        let TokenTree::Keyword(keyword::Keyword::Const(keyword)) = token else {
            Ice::impossible(None)
        };

        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::Ident))
        }
        .map_err(DeclarationError::Source)?
        .map_err(|cause| {
            let target = Annotated::target(&cause);
            let labels = InlineAnnotations::new(
                Label::new("expected a non-reserved symbolic name", target),
                [Label::new("this declaration starts here", start)],
            );

            DeclarationError::Expected(Expectation { cause, labels })
        })?;

        let TokenTree::Ident(name) = token else {
            Ice::impossible(None)
        };

        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::Colon))
        }
        .map_err(DeclarationError::Source)?
        .map_err(|cause| {
            let target = Annotated::target(&cause);
            let labels = InlineAnnotations::new(
                Label::new("fields and constants require an explicit `: type`", target),
                [Label::new("this declaration starts here", start)],
            );

            DeclarationError::Expected(Expectation { cause, labels })
        })?;

        let TokenTree::Punctuation(punct::Punctuation::Colon(colon)) = token else {
            Ice::impossible(None)
        };

        let ty = Type::lex(&mut b0).map_err(DeclarationError::Type)?;
        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::Equals))
        }
        .map_err(DeclarationError::Source)?
        .map_err(|cause| {
            let target = Annotated::target(&cause);
            let labels = InlineAnnotations::new(
                Label::new("expected `=` before the definition", target),
                [Label::new("this declaration starts here", start)],
            );

            DeclarationError::Expected(Expectation { cause, labels })
        })?;

        let TokenTree::Punctuation(punct::Punctuation::Eq(equals)) = token else {
            Ice::impossible(None)
        };

        let readiness = {
            let mut b1 = b0.branch();
            let trivia = b1.source().trivia();

            b1.ignore_while(trivia);

            b1.source().checked(b1.expect_is_next(expression::Fragment::Start))
        }
        .map_err(DeclarationError::Source)?;

        if let Err(cause) = readiness {
            let target = Annotated::target(&cause);
            let labels = InlineAnnotations::new(
                Label::new("expected an initializer expression after `=`", target),
                [Label::new("this initializer starts here", equals.span())],
            );

            return Err(DeclarationError::Initializer(Initializer { cause, labels }));
        }

        let value = Expr::lex(&mut b0).map_err(DeclarationError::Expression)?;

        b0.backport();

        Ok((keyword, name, colon, ty, equals, value))
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, DeclarationError>
    where
        't: 'i,
    {
        let (keyword, name, colon, ty, equals, value) = input;

        let span = keyword.span().superset(value.span());

        Ok(Self {
            keyword,
            name,
            colon,
            ty,
            equals,
            value,
            span,
        })
    }
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Rule {
    type Context = ();
    type Error = DeclarationError;
    type Input<'i>
        = (keyword::Rule, ident::Ident, punct::Eq, Expr)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, DeclarationError>
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
        .map_err(DeclarationError::Source)?;
        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::Rule))
        }
        .map_err(DeclarationError::Source)?
        .map_err(|cause| {
            let target = Annotated::target(&cause);
            let labels = InlineAnnotations::new(
                Label::new("expected `rule`", target),
                [Label::new("this declaration starts here", start)],
            );

            DeclarationError::Expected(Expectation { cause, labels })
        })?;

        let TokenTree::Keyword(keyword::Keyword::Rule(keyword)) = token else {
            Ice::impossible(None)
        };

        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::Ident))
        }
        .map_err(DeclarationError::Source)?
        .map_err(|cause| {
            let target = Annotated::target(&cause);
            let labels = InlineAnnotations::new(
                Label::new("expected a non-reserved symbolic name", target),
                [Label::new("this declaration starts here", start)],
            );

            DeclarationError::Expected(Expectation { cause, labels })
        })?;

        let TokenTree::Ident(name) = token else {
            Ice::impossible(None)
        };

        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::Equals))
        }
        .map_err(DeclarationError::Source)?
        .map_err(|cause| {
            let target = Annotated::target(&cause);
            let labels = InlineAnnotations::new(
                Label::new("expected `=` before the definition", target),
                [Label::new("this declaration starts here", start)],
            );

            DeclarationError::Expected(Expectation { cause, labels })
        })?;

        let TokenTree::Punctuation(punct::Punctuation::Eq(equals)) = token else {
            Ice::impossible(None)
        };

        let readiness = {
            let mut b1 = b0.branch();
            let trivia = b1.source().trivia();

            b1.ignore_while(trivia);

            b1.source().checked(b1.expect_is_next(expression::Fragment::Start))
        }
        .map_err(DeclarationError::Source)?;

        if let Err(cause) = readiness {
            let target = Annotated::target(&cause);
            let labels = InlineAnnotations::new(
                Label::new("expected an initializer expression after `=`", target),
                [Label::new("this initializer starts here", equals.span())],
            );

            return Err(DeclarationError::Initializer(Initializer { cause, labels }));
        }

        let value = Expr::lex(&mut b0).map_err(DeclarationError::Expression)?;

        b0.backport();

        Ok((keyword, name, equals, value))
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, DeclarationError>
    where
        't: 'i,
    {
        let (keyword, name, equals, value) = input;

        let span = keyword.span().superset(value.span());

        Ok(Self {
            keyword,
            name,
            equals,
            value,
            span,
        })
    }
}

/// A plain binding or a policy-wrapped binding awaiting field construction.
#[derive(Debug)]
pub enum FieldInput {
    /// A plain binding.
    Plain(Binding),

    /// An explicit policy wrapper.
    Policy(PolicyField),
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Field {
    type Context = ();
    type Error = DeclarationError;
    type Input<'i>
        = FieldInput
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, DeclarationError>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();
        let input = if {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.is_next(Fragment::FieldPolicy))
        }
        .map_err(DeclarationError::Source)?
        {
            FieldInput::Policy(PolicyField::lex(&mut b0)?)
        } else {
            FieldInput::Plain(Binding::lex(&mut b0)?)
        };

        b0.backport();

        Ok(input)
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, DeclarationError>
    where
        't: 'i,
    {
        Ok(match input {
            FieldInput::Plain(v) => Self::Plain(v),
            FieldInput::Policy(v) => Self::Policy(v),
        })
    }
}

/// Recognized statement alternatives awaiting document construction.
#[derive(Debug)]
pub enum StatementInput {
    /// The include production.
    Include(Include),

    /// The extend production.
    Extend(Extend),

    /// The inherit production.
    Inherit(Inherit),

    /// The domain production.
    Domain(Domain),

    /// The type production.
    Type(TypeAlias),

    /// The constant production.
    Constant(Constant),

    /// The rule production.
    Rule(Rule),

    /// The field production.
    Field(Field),
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Statement {
    type Context = ();
    type Error = DeclarationError;
    type Input<'i>
        = StatementInput
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, DeclarationError>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();
        let input = match {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.peek())
        }
        .map_err(DeclarationError::Source)?
        {
            Some(TokenTree::Keyword(keyword::Keyword::Include(_))) => StatementInput::Include(Include::lex(&mut b0)?),
            Some(TokenTree::Keyword(keyword::Keyword::Extend(_))) => StatementInput::Extend(Extend::lex(&mut b0)?),
            Some(TokenTree::Keyword(keyword::Keyword::Inherit(_))) => StatementInput::Inherit(Inherit::lex(&mut b0)?),
            Some(TokenTree::Keyword(keyword::Keyword::Domain(_))) => StatementInput::Domain(Domain::lex(&mut b0)?),
            Some(TokenTree::Keyword(keyword::Keyword::Type(_))) => StatementInput::Type(TypeAlias::lex(&mut b0)?),
            Some(TokenTree::Keyword(keyword::Keyword::Const(_))) => StatementInput::Constant(Constant::lex(&mut b0)?),
            Some(TokenTree::Keyword(keyword::Keyword::Rule(_))) => StatementInput::Rule(Rule::lex(&mut b0)?),
            _ => StatementInput::Field(Field::lex(&mut b0)?),
        };

        b0.backport();

        Ok(input)
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, DeclarationError>
    where
        't: 'i,
    {
        Ok(match input {
            StatementInput::Include(v) => Self::Include(v),
            StatementInput::Extend(v) => Self::Extend(v),
            StatementInput::Inherit(v) => Self::Inherit(v),
            StatementInput::Domain(v) => Self::Domain(v),
            StatementInput::Type(v) => Self::Type(v),
            StatementInput::Constant(v) => Self::Constant(v),
            StatementInput::Rule(v) => Self::Rule(v),
            StatementInput::Field(v) => Self::Field(v),
        })
    }
}

#[cfg(test)]
mod tests {
    use core::error::Error;

    use aldebaran::{
        prelude::{Annotated, Annotations, Cursor},
        text::cursor::error::Expected,
    };

    use super::*;
    use crate::scan::{
        context::Shared,
        tree::{Stream, source::Tokens},
    };

    #[test]
    fn missing_initializers_retain_the_expression_expectation_and_assignment() {
        for source in ["x: u8 =", "const x: u8 =", "rule x ="] {
            let stream = Stream::new(Cursor::create(source), Shared::new(), None);

            let tokens = Tokens::new(stream);
            let tokens_ref = &tokens;
            let mut c0 = Cursor::at(&tokens_ref, tokens.stream().position());

            let DeclarationError::Initializer(error) = Statement::lex(&mut c0).expect_err("missing initializer") else {
                unreachable!("an initializer has its own expression expectation")
            };

            let labels = error.annotations().list().expect("initializer labels");
            let cause = Error::source(&error).expect("typed cursor failure");

            assert!(
                cause
                    .downcast_ref::<Expected<TokenTree, expression::Fragment>>()
                    .is_some()
            );

            assert_eq!(Annotated::target(error.cause()).start(), source.len());

            assert_eq!(labels.first().target().start(), source.len());

            assert_eq!(source.get(labels.rest()[0].target().range()), Some("="));

            assert_eq!(c0.index(), 0);
        }
    }
}

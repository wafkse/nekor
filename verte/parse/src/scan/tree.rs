//! Structural lexical occurrences and lazy streams over the original text.

use core::fmt;

use aldebaran::{
    grammar::stream::TokenStream,
    logic::connective::OneOf,
    prelude::{Assert, Cursor, Lex, Report, Span, Spanned},
    print::visualize::Visualize,
    report::annotated::{InlineAnnotations, label::Label},
    text::cursor::{internment::Internment, stream::LexStream},
};
use fack::prelude::Error;

use super::{context::ScanContext, ident, keyword, number, punct, string, trivia};

pub mod group;
pub mod source;

/// Source components selecting a lexical construct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Assert)]
pub enum Fragment {
    /// A symbolic word.
    #[assert(use ident::Fragment::Start type ident::Fragment)]
    Word,

    /// A numeric occurrence.
    #[assert(use number::Fragment::Decimal type number::Fragment)]
    Number,

    /// A quoted string.
    #[assert(use string::Fragment::Quote type string::Fragment)]
    String,

    /// Horizontal spacing, a comment, or a physical ending.
    #[assert(
        use OneOf::these([
            trivia::Fragment::Space,
            trivia::Fragment::Newline,
            trivia::Fragment::Comment,
        ])
        type OneOf<trivia::Fragment, 3>
    )]
    Trivia,

    /// A group opener.
    #[assert(use group::Fragment::Open type group::Fragment)]
    Group,

    /// A closer outside its owning group.
    #[assert(use group::Fragment::Close type group::Fragment)]
    Close,
}

/// A concrete lexical occurrence or a bounded group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TokenTree {
    /// A symbolic name.
    Ident(ident::Ident),

    /// A dedicated keyword occurrence.
    Keyword(keyword::Keyword),

    /// An exact numeric occurrence.
    Number(number::Number),

    /// A decoded quoted literal.
    String(string::Literal),

    /// A maximal punctuation occurrence.
    Punctuation(punct::Punctuation),

    /// Horizontal spacing, a comment, or a physical line ending.
    Trivia(trivia::Trivia),

    /// A matched delimiter pair with source-backed content.
    Group(group::Group),

    /// A lexical failure recorded by the owning token source.
    Invalid(source::Invalid),
}

impl Spanned for TokenTree {
    #[inline]
    fn span(&self) -> Span {
        match self {
            Self::Ident(v) => v.span(),
            Self::Keyword(v) => v.span(),
            Self::Number(v) => v.span(),
            Self::String(v) => v.span(),
            Self::Punctuation(v) => v.span(),
            Self::Trivia(v) => v.span(),
            Self::Group(v) => v.span(),
            Self::Invalid(v) => v.span(),
        }
    }
}

impl Visualize for TokenTree {
    #[inline]
    fn visualize<W>(&self, writer: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        match self {
            Self::Keyword(value) => value.visualize(writer),
            Self::Punctuation(value) => value.visualize(writer),
            Self::Ident(_) => writer.write_str("identifier"),
            Self::Number(_) => writer.write_str("number"),
            Self::String(_) => writer.write_str("quoted string"),
            Self::Trivia(_) => writer.write_str("trivia"),
            Self::Invalid(_) => writer.write_str("invalid lexical input"),
            Self::Group(group) => writer.write_str(match group.delimiter() {
                group::Delimiter::Parenthesis(..) => "parenthesized group",
                group::Delimiter::Bracket(..) => "bracketed group",
                group::Delimiter::Brace(..) => "braced group",
            }),
        }
    }
}

/// Failure in the selected lexical construct.
#[derive(Debug, Clone, Report)]
#[report(forward)]
pub enum TokenError {
    /// The lexical storage domain is already borrowed.
    StorageBorrow(StorageBorrow),

    /// Invalid symbolic name or storage failure.
    Ident(ident::IdentError),

    /// Invalid keyword.
    Keyword(keyword::KeywordError),

    /// Invalid numeric literal.
    Number(number::NumberError),

    /// Invalid quoted literal.
    String(string::StringError),

    /// Unknown source component.
    Unexpected(Unexpected),

    /// Invalid trivia.
    Trivia(trivia::TriviaError),

    /// Punctuation construction failed.
    Punctuation(punct::PunctuationError),

    /// Invalid delimiter structure.
    Group(group::DelimiterError),
}

impl fmt::Display for TokenError {
    #[inline]
    fn fmt(&self, writer: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StorageBorrow(error) => fmt::Display::fmt(error, writer),
            Self::Ident(error) => fmt::Display::fmt(error, writer),
            Self::Keyword(error) => fmt::Display::fmt(error, writer),
            Self::Number(error) => fmt::Display::fmt(error, writer),
            Self::String(error) => fmt::Display::fmt(error, writer),
            Self::Unexpected(error) => fmt::Display::fmt(error, writer),
            Self::Trivia(error) => fmt::Display::fmt(error, writer),
            Self::Punctuation(error) => fmt::Display::fmt(error, writer),
            Self::Group(error) => fmt::Display::fmt(error, writer),
        }
    }
}

impl Error for TokenError {
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::StorageBorrow(error) => Some(error),
            Self::Ident(error) => Some(error),
            Self::Keyword(error) => Some(error),
            Self::Number(error) => Some(error),
            Self::String(error) => Some(error),
            Self::Unexpected(error) => Some(error),
            Self::Trivia(error) => Some(error),
            Self::Punctuation(error) => Some(error),
            Self::Group(error) => Some(error),
        }
    }
}

/// A component that cannot begin a Verte lexical occurrence.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("unexpected source character")]
#[error(source(cause))]
#[report(title = "unexpected source character", annotations = labels)]
// NOTE(invariant): None.
pub struct Unexpected {
    /// The rejected punctuation expectation when classification reached that family.
    cause: Option<punct::PunctuationError>,

    /// Primary failure location followed by related source context.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

/// Recognized source awaiting lexical construction.
#[derive(Debug)]
pub enum Input<'s> {
    /// An identifier proof.
    Ident(ident::Input<'s>),

    /// A keyword proof.
    Keyword(keyword::Input),

    /// A numeric proof.
    Number(number::Input),

    /// A quoted literal proof.
    String(string::Input<'s>),

    /// A punctuation proof.
    Punctuation(punct::Input),

    /// A trivia proof.
    Trivia(trivia::Input<'s>),

    /// A group proof.
    Group(group::Input<'s>),
}

impl<'s> Lex<'s, str> for TokenTree {
    type Context = ScanContext<'s>;
    type Error = TokenError;
    type Input<'i>
        = Input<'s>
    where
        's: 'i;

    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "Keep substantial recognition or construction out of line across grammar entry points."
    )]
    fn pass<'i>(c0: &'i mut Cursor<'s, str>) -> Result<Input<'s>, TokenError>
    where
        's: 'i,
    {
        if c0.is_next(Fragment::Trivia) {
            return trivia::Trivia::pass(c0).map(Input::Trivia).map_err(TokenError::Trivia);
        }
        if c0.is_next(Fragment::String) {
            return <string::Literal as Lex<'s, str>>::pass(c0)
                .map(Input::String)
                .map_err(TokenError::String);
        }
        if c0.is_next(Fragment::Number) {
            return number::Number::pass(c0).map(Input::Number).map_err(TokenError::Number);
        }
        if c0.is_next(Fragment::Word) {
            if let Ok(input) = keyword::Keyword::pass(c0) {
                return Ok(Input::Keyword(input));
            }
            return ident::Ident::pass(c0).map(Input::Ident).map_err(TokenError::Ident);
        }
        if c0.is_next(Fragment::Group) {
            return group::Group::pass(c0).map(Input::Group).map_err(TokenError::Group);
        }
        if c0.is_next(Fragment::Close) {
            let index = c0.index();
            let target = (*c0).any_spanned().map_or(Span::unit(index), |(_, span)| span);
            let labels =
                InlineAnnotations::new(Label::new("this closing delimiter has no matching opener", target), []);

            return Err(TokenError::Unexpected(Unexpected { cause: None, labels }));
        }
        punct::Punctuation::pass(c0).map(Input::Punctuation).map_err(|cause| {
            let index = c0.index();
            let target = (*c0).any_spanned().map_or(Span::unit(index), |(_, span)| span);
            let labels = InlineAnnotations::new(
                Label::new(
                    "expected a Verte lexical occurrence; unsupported line separators are not whitespace",
                    target,
                ),
                [],
            );

            TokenError::Unexpected(Unexpected {
                cause: Some(cause),
                labels,
            })
        })
    }

    #[inline]
    fn build_with_context<'i>(input: Input<'s>, context: &mut Self::Context) -> Result<Self, TokenError>
    where
        's: 'i,
    {
        match input {
            Input::Ident(input) => ident::Ident::build_with_context(input, context)
                .map(Self::Ident)
                .map_err(TokenError::Ident),
            Input::Keyword(input) => keyword::Keyword::build(input)
                .map(Self::Keyword)
                .map_err(TokenError::Keyword),
            Input::Number(input) => number::Number::build(input)
                .map(Self::Number)
                .map_err(TokenError::Number),
            Input::String(input) => <string::Literal as Lex<'s, str>>::build_with_context(input, context)
                .map(Self::String)
                .map_err(TokenError::String),
            Input::Punctuation(input) => punct::Punctuation::build(input)
                .map(Self::Punctuation)
                .map_err(TokenError::Punctuation),
            Input::Trivia(input) => trivia::Trivia::build_with_context(input, context)
                .map(Self::Trivia)
                .map_err(TokenError::Trivia),
            Input::Group(input) => group::Group::build_with_context(input, context)
                .map(Self::Group)
                .map_err(TokenError::Group),
        }
    }

    #[inline]
    fn lex_with_context(c0: &mut Cursor<'s, str>, context: &mut Self::Context) -> Result<Self, TokenError> {
        let mut b0 = c0.branch();

        if b0.is_next(Fragment::Group) {
            let value = group::Group::lex_with_context(&mut b0, context).map_err(TokenError::Group)?;

            b0.backport();
            return Ok(Self::Group(value));
        }

        let input = Self::pass(&mut b0)?;
        let value = Self::build_with_context(input, context)?;

        b0.backport();

        Ok(value)
    }
}

/// A lazy lexical stream sharing storage with its bounded child streams.
// NOTE(invariant): `cursor` remains in the same source coordinate space throughout traversal.
pub struct Stream<'s> {
    /// Source traversal for this scope.
    cursor: Cursor<'s, str>,

    /// Storage domain shared with nested scopes.
    context: super::context::Shared<'s>,

    /// The enclosing group, absent for an outer source stream.
    scope: Option<group::Group>,
}

impl<'s> Stream<'s> {
    /// Traverse a source scope in the supplied storage domain.
    #[inline]
    pub const fn new(
        cursor: Cursor<'s, str>,
        context: super::context::Shared<'s>,
        scope: Option<group::Group>,
    ) -> Self {
        Self { cursor, context, scope }
    }

    /// Retrieve the initial byte position of this lexical scope.
    #[inline]
    pub const fn position(&self) -> usize {
        let Self { cursor, .. } = self;

        cursor.index()
    }

    /// Retrieve the group owning this bounded stream, if any.
    #[inline]
    pub const fn scope(&self) -> Option<group::Group> {
        let &Self { scope, .. } = self;

        scope
    }

    /// Retrieve this scope's source prefix in original byte coordinates.
    #[inline]
    pub const fn source(&self) -> &'s str {
        let Self { cursor, .. } = self;

        cursor.source()
    }

    /// Borrow the storage domain shared by this scope and its children.
    #[inline]
    pub const fn context(&self) -> &super::context::Shared<'s> {
        let Self { context, .. } = self;

        context
    }
}

impl TokenStream for Stream<'_> {
    type Error = TokenError;
    type Token = TokenTree;

    #[inline]
    fn next(&mut self) -> Result<Option<TokenTree>, TokenError> {
        let Self { cursor, context, .. } = self;

        let mut context = context.borrow_mut().map_err(|_borrow| {
            let labels = InlineAnnotations::new(
                Label::new(
                    "release the storage borrow before advancing this stream",
                    Span::unit(cursor.index()),
                ),
                [],
            );

            TokenError::StorageBorrow(StorageBorrow { labels })
        })?;
        let mut state = Internment::new(cursor.source(), &mut *context);

        let (c0, _) = state.parts_mut();

        *c0 = *cursor;

        let mut stream = LexStream::<str, TokenTree>::new(state);
        let result = stream.next();

        let LexStream(mut state) = stream;

        let (c0, _) = state.parts_mut();

        *cursor = *c0;
        result
    }
}

/// The storage domain is borrowed by another operation.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("lexical storage is already borrowed")]
#[report(title = "lexical storage is already borrowed", annotations = labels)]
// NOTE(invariant): None.
pub struct StorageBorrow {
    /// Primary failure location followed by related source context.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

/// The ordinary Aldebaran cursor over a borrowed token source.
pub type SyntaxCursor<'s, 't> = Cursor<'t, &'t source::Tokens<'s>>;

/// Structural trivia expectations for stream filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Assert)]
#[assert(input(TokenTree))]
pub enum TriviaFragment {
    /// Ignore spaces and comments at document level.
    #[assert(
        matches(
            TokenTree::Trivia(trivia::Trivia::Whitespace(_) | trivia::Trivia::Comment(_))
        ) as "horizontal trivia"
    )]
    Horizontal,

    /// Ignore all physical trivia inside groups.
    #[assert(matches(TokenTree::Trivia(_)) as "group trivia")]
    All,

    /// A physical statement terminator.
    #[assert(matches(TokenTree::Trivia(trivia::Trivia::LineBreak(_))) as "a physical line ending")]
    Newline,
}

impl Unexpected {
    /// Retrieve the punctuation failure when that family rejected the component.
    #[inline]
    pub const fn cause(&self) -> Option<&punct::PunctuationError> {
        let Self { cause, .. } = self;

        cause.as_ref()
    }
}

#[cfg(test)]
mod tests {

    use aldebaran::text::cursor::{internment::Internment, stream::LexStream};

    use super::*;

    #[test]
    fn stream_is_demand_driven() {
        let mut context = ScanContext::new();
        let internment = Internment::new("name \\", &mut context);
        let mut stream = LexStream::<str, TokenTree>::new(internment);

        assert!(matches!(stream.next(), Ok(Some(TokenTree::Ident(_)))));

        assert!(matches!(stream.next(), Ok(Some(TokenTree::Trivia(_)))));

        assert!(stream.next().is_err());
    }
}

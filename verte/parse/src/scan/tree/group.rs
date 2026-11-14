//! Matched delimiters and source-bounded group content.

use alloc::{boxed::Box, collections::BTreeMap, vec::Vec};
use core::{fmt, ptr};

use aldebaran::{
    ice::Ice,
    logic::connective::OneOf,
    prelude::{Annotated, Assert, Cursor, Lex, Report, Span, Spanned},
    report::annotated::{InlineAnnotations, label::Label},
    text::{builtin::Char, cursor::error::Expected},
};
use fack::prelude::Error;

use super::{TokenError as TreeError, TokenTree};
use crate::scan::{context::ScanContext, punct};

/// Group delimiter component expectations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Assert)]
pub enum Fragment {
    /// Any opening delimiter.
    #[assert(use OneOf::these([Char('('), Char('['), Char('{')]) type OneOf<Char, 3>)]
    Open,

    /// Any closing delimiter.
    #[assert(use OneOf::these([Char(')'), Char(']'), Char('}')]) type OneOf<Char, 3>)]
    Close,
}

pub mod syntax;

/// A matching concrete delimiter pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Delimiter {
    /// Parentheses.
    Parenthesis(punct::LeftParen, punct::RightParen),

    /// Brackets.
    Bracket(punct::LeftBracket, punct::RightBracket),

    /// Braces.
    Brace(punct::LeftBrace, punct::RightBrace),
}

impl Spanned for Delimiter {
    #[inline]
    fn span(&self) -> Span {
        match self {
            Self::Parenthesis(l, r) => l.span().superset(r.span()),
            Self::Bracket(l, r) => l.span().superset(r.span()),
            Self::Brace(l, r) => l.span().superset(r.span()),
        }
    }
}

impl Delimiter {
    /// Retrieve the opening delimiter's source extent.
    #[inline]
    pub fn open(&self) -> Span {
        match self {
            Self::Parenthesis(l, _) => l.span(),
            Self::Bracket(l, _) => l.span(),
            Self::Brace(l, _) => l.span(),
        }
    }

    /// Retrieve the closing delimiter's source extent.
    #[inline]
    pub fn close(&self) -> Span {
        match self {
            Self::Parenthesis(_, r) => r.span(),
            Self::Bracket(_, r) => r.span(),
            Self::Brace(_, r) => r.span(),
        }
    }
}

/// Matched delimiters whose enclosed syntax is parsed when a consumer enters the group.
///
/// Recognition scans the contents to validate lexical forms and find matching
/// delimiters. It leaves construction of the enclosed syntax to its consumer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
// NOTE(invariant): `start..end` is exactly the content between `delimiter`'s matching occurrences.
pub struct Group {
    /// Matching concrete delimiter occurrences.
    delimiter: Delimiter,

    /// Absolute content boundary after the opener.
    start: usize,

    /// Absolute content boundary before the closer.
    end: usize,
}

impl Group {
    /// Retrieve the matching delimiter occurrences.
    #[inline]
    pub const fn delimiter(&self) -> Delimiter {
        let &Self { delimiter, .. } = self;

        delimiter
    }

    /// Retrieve the absolute content byte boundaries, including an empty body.
    #[inline]
    pub const fn content(&self) -> (usize, usize) {
        let &Self { start, end, .. } = self;

        (start, end)
    }
}

impl Spanned for Group {
    #[inline]
    fn span(&self) -> Span {
        let Self { delimiter, .. } = self;

        delimiter.span()
    }
}

/// A matched pair awaiting group construction.
#[derive(Debug)]
// NOTE(invariant): The opening and closing punctuation match.
pub struct Input<'s> {
    /// Immutable source that established the delimiter boundaries.
    source: &'s str,

    /// Nested matched pairs in closing order, including the outer pair last.
    pairs: Vec<(punct::Punctuation, punct::Punctuation)>,

    /// The outer opening delimiter occurrence.
    open: punct::Punctuation,

    /// The matching outer closing delimiter occurrence.
    close: punct::Punctuation,
}

/// Invalid lexical content or delimiter structure.
#[derive(Debug, Clone, Report)]
#[report(forward)]
pub enum DelimiterError {
    /// An opening delimiter was required.
    Expected(Opening),

    /// Mismatched or missing delimiters.
    Delimiter(Mismatch),

    /// A delimiter could not be recognized.
    Punctuation(punct::PunctuationError),

    /// A nested lexical construct failed.
    Lexical(Box<TreeError>),
}

impl fmt::Display for DelimiterError {
    #[inline]
    fn fmt(&self, writer: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expected(error) => fmt::Display::fmt(error, writer),
            Self::Delimiter(error) => fmt::Display::fmt(error, writer),
            Self::Punctuation(error) => fmt::Display::fmt(error, writer),
            Self::Lexical(error) => fmt::Display::fmt(error, writer),
        }
    }
}

impl Error for DelimiterError {
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Expected(error) => Some(error),
            Self::Delimiter(error) => Some(error),
            Self::Punctuation(error) => Some(error),
            Self::Lexical(error) => Some(error.as_ref()),
        }
    }
}

/// A group opener was required at this source location.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("expected an opening delimiter")]
#[error(source(cause))]
#[report(title = "expected an opening delimiter", annotations = labels)]
// NOTE(invariant): None.
pub struct Opening {
    /// Required opener and the scalar or boundary that was found.
    cause: Expected<char, Fragment>,

    /// The location at which a group opener was required.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

impl Opening {
    /// Retrieve the failed opener expectation.
    #[inline]
    pub const fn cause(&self) -> &Expected<char, Fragment> {
        let Self { cause, .. } = self;

        cause
    }
}

/// A group delimiter failed to match its opener.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("unclosed delimiter group")]
#[report(title = "unclosed delimiter group", annotations = labels)]
// NOTE(invariant): The related label identifies the unmatched opener.
pub struct Mismatch {
    /// Primary failure location followed by related source context.
    labels: InlineAnnotations<Label<&'static str>, 1>,
}

impl<'s> Lex<'s, str> for Group {
    type Context = ScanContext<'s>;
    type Error = DelimiterError;
    type Input<'i>
        = Input<'s>
    where
        's: 'i;

    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "Keep substantial recognition or construction out of line across grammar entry points."
    )]
    fn pass<'i>(c0: &'i mut Cursor<'s, str>) -> Result<Input<'s>, DelimiterError>
    where
        's: 'i,
    {
        let mut b0 = c0.branch();

        b0.validate_is_next(Fragment::Open).map_err(|cause| {
            let labels = InlineAnnotations::new(Label::new("expected `(`, `[`, or `{`", Annotated::target(&cause)), []);

            DelimiterError::Expected(Opening { cause, labels })
        })?;

        let open = punct::Punctuation::lex(&mut b0).map_err(DelimiterError::Punctuation)?;
        let mut frames = Vec::new();
        let mut pairs = Vec::new();

        frames.push(open);

        let close = loop {
            let opener = Ice::some(frames.last().copied());

            if b0.is_eof() {
                let message = match opener {
                    punct::Punctuation::LeftParen(_) => "expected `)` to close this parenthesized group",
                    punct::Punctuation::LeftBracket(_) => "expected `]` to close this bracketed group",
                    punct::Punctuation::LeftBrace(_) => "expected `}` to close this braced group",
                    _ => Ice::impossible(None),
                };
                let labels = InlineAnnotations::new(
                    Label::new(message, Span::unit(b0.index())),
                    [Label::new("this group starts here", opener.span())],
                );

                return Err(DelimiterError::Delimiter(Mismatch { labels }));
            }
            if b0.is_next(Fragment::Open) {
                frames.push(punct::Punctuation::lex(&mut b0).map_err(DelimiterError::Punctuation)?);
            } else if b0.is_next(Fragment::Close) {
                let close = punct::Punctuation::lex(&mut b0).map_err(DelimiterError::Punctuation)?;

                match (opener, close) {
                    (punct::Punctuation::LeftParen(_), punct::Punctuation::RightParen(_))
                    | (punct::Punctuation::LeftBracket(_), punct::Punctuation::RightBracket(_))
                    | (punct::Punctuation::LeftBrace(_), punct::Punctuation::RightBrace(_)) => {
                        frames.pop();
                        pairs.push((opener, close));
                    },
                    _ => {
                        let labels = InlineAnnotations::new(
                            Label::new("this delimiter does not close the active group", close.span()),
                            [Label::new("the unmatched group starts here", opener.span())],
                        );

                        return Err(DelimiterError::Delimiter(Mismatch { labels }));
                    },
                }
                if frames.is_empty() {
                    break close;
                }
            } else {
                TokenTree::pass(&mut b0).map_err(|error| DelimiterError::Lexical(Box::new(error)))?;
            }
        };
        let source = b0.source();

        b0.backport();

        Ok(Input {
            source,
            pairs,
            open,
            close,
        })
    }

    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "Keep substantial recognition or construction out of line across grammar entry points."
    )]
    fn build_with_context<'i>(
        Input { source, pairs, .. }: Input<'s>,
        context: &mut Self::Context,
    ) -> Result<Self, DelimiterError>
    where
        's: 'i,
    {
        let mut outer = None;

        for (open, close) in pairs {
            let delimiter = Delimiter::build(PairInput { open, close })?;
            let start = delimiter.open().end();
            let end = delimiter.close().start();
            let group = Self { delimiter, start, end };

            context.boundaries_mut().insert(source, group);
            outer = Some(group);
        }
        Ok(Ice::some(outer))
    }

    #[inline]
    fn lex_with_context(c0: &mut Cursor<'s, str>, context: &mut Self::Context) -> Result<Self, DelimiterError> {
        let mut b0 = c0.branch();

        if let Some(group) = context.boundaries().get(b0.source(), b0.index()) {
            let source = b0.source();

            *b0 = Cursor::at(source, group.span().end());
            b0.backport();
            return Ok(group);
        }

        let input = Self::pass(&mut b0)?;
        let value = Self::build_with_context(input, context)?;

        b0.backport();

        Ok(value)
    }
}

/// A matched punctuation pair awaiting delimiter construction.
#[derive(Debug)]
// NOTE(invariant): `open` and `close` are a matching pair established by group recognition.
pub struct PairInput {
    /// Opening delimiter occurrence.
    open: punct::Punctuation,

    /// Matching closing delimiter occurrence.
    close: punct::Punctuation,
}

impl<'s> Lex<'s, str> for Delimiter {
    type Context = ();
    type Error = DelimiterError;
    type Input<'i>
        = PairInput
    where
        's: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut Cursor<'s, str>) -> Result<PairInput, DelimiterError>
    where
        's: 'i,
    {
        let Input { open, close, .. } = Group::pass(c0)?;

        Ok(PairInput { open, close })
    }

    #[inline]
    fn build_with_context<'i>(PairInput { open, close }: PairInput, _: &mut ()) -> Result<Self, DelimiterError>
    where
        's: 'i,
    {
        Ok(match (open, close) {
            (punct::Punctuation::LeftParen(l), punct::Punctuation::RightParen(r)) => Self::Parenthesis(l, r),
            (punct::Punctuation::LeftBracket(l), punct::Punctuation::RightBracket(r)) => Self::Bracket(l, r),
            (punct::Punctuation::LeftBrace(l), punct::Punctuation::RightBrace(r)) => Self::Brace(l, r),
            _ => Ice::impossible(None),
        })
    }
}

/// Recognized delimiter boundaries tied to the borrowed source allocation.
#[derive(Debug)]
// NOTE(invariant): Each entry holds an immutable source borrow, preventing address reuse while
// cached.
pub struct Cache<'s> {
    /// Groups indexed by source base address and opening byte position.
    entries: BTreeMap<(usize, usize), Entry<'s>>,
}

/// A matched group and the source that established its boundaries.
#[derive(Debug)]
// NOTE(invariant): `group` is contained in `source` in original byte coordinates.
struct Entry<'s> {
    /// Source borrow keeping this allocation alive.
    source: &'s str,

    /// Already recognized delimiters and content boundaries.
    group: Group,
}

impl<'s> Cache<'s> {
    /// Create an empty boundary cache.
    #[inline]
    pub const fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Retrieve a group only when it fits the requested source prefix.
    #[inline]
    pub fn get(&self, source: &str, index: usize) -> Option<Group> {
        let Self { entries } = self;

        let key = (source.as_ptr() as usize, index);

        match entries.get(&key) {
            Some(Entry {
                source: original,
                group,
            }) if ptr::eq(original.as_ptr(), source.as_ptr()) && group.span().end() <= source.len() => Some(*group),
            _ => None,
        }
    }

    /// Cache the boundaries established by successful group recognition.
    #[inline]
    fn insert(&mut self, source: &'s str, group: Group) {
        let Self { entries } = self;

        let key = (source.as_ptr() as usize, group.span().start());

        entries.insert(key, Entry { source, group });
    }
}

impl Default for Cache<'_> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {

    use aldebaran::report::annotated::Annotations as _;

    use super::*;

    #[test]
    fn lexical_content_does_not_close_groups() {
        let source = r#"( ["}\""] # )
 {}) tail"#;
        let mut c0 = Cursor::create(source);
        let group = Group::lex(&mut c0).expect("matched group");

        assert!(c0.index() < source.len());
        let (start, _) = group.content();

        assert_eq!(start, 1);
    }
    #[test]
    fn mismatch_and_eof_retain_both_locations() {
        for source in ["([)]", "(", "{[x}"] {
            let mut c0 = Cursor::create(source);
            let error = Group::lex(&mut c0).expect_err("unclosed");

            assert_eq!(error.annotations().list().expect("labels").rest().len(), 1);

            assert_eq!(c0.index(), 0);
        }
    }
    #[test]
    fn nested_boundaries_are_cached_and_respect_scope_ends() {
        let source = "( [name] {key = \"text\"} )";
        let mut context = ScanContext::new();
        let mut c0 = Cursor::create(source);
        let outer = Group::lex_with_context(&mut c0, &mut context).expect("outer group");
        let nested = context
            .boundaries()
            .get(source, 2)
            .expect("nested boundary was discovered");

        let (start, end) = outer.content();

        let prefix = source.get(..end).expect("bounded source");
        let mut c1 = Cursor::at(prefix, 2);
        let reused = Group::lex_with_context(&mut c1, &mut context).expect("cached group");

        assert_eq!(reused, nested);

        assert_eq!(c1.index(), nested.span().end());

        assert_eq!(start, 1);
        let truncated = source.get(..nested.span().end() - 1).expect("truncated scope");

        assert!(context.boundaries().get(truncated, 2).is_none());
    }
}

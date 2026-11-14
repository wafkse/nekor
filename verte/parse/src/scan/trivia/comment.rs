//! Comments found inside Verte source text.

use core::fmt;

use aldebaran::{
    baseline::span::{Span, Spanned},
    id::ident::TaggedId,
    interner::storage::StorageId,
    logic::connective::{Not, Or},
    prelude::{Annotated, Assert, Cursor, Report},
    report::annotated::{InlineAnnotations, label::Label},
    source::terminate::{Single, TerminatorCompose as _},
    text::{
        builtin::{Ascii, Char},
        cursor::{consume::Lex, error::Expected},
    },
};
use fack::prelude::Error;

use crate::scan::context::ScanContext;

/// A textual fragment of a comment.
///
/// This is an [`Assert`] that is used to match the inline comment.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Assert)]
pub enum Fragment {
    /// A `#` character that starts an inline comment.
    #[assert(use Ascii::Hash type Ascii)]
    Hash,

    /// A fragment that determines the rest of the comment.
    ///
    /// This represents the inline (as in same-line) content of the comment.
    #[assert(use Not::that(Or::one(Char('\r'), Char('\n'))) type Not<Or<Char, Char>>)]
    Content,
}

/// A type alias for a tagged, interned [`str`].
type InternedStr = TaggedId<str, StorageId>;

/// An inline comment.
///
/// This is an inline comment inside a `Verte` source code: `# This is an inline comment.`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
// NOTE(invariant): The interned text is the exact source slice covered by the span.
pub struct Comment(InternedStr, Span);

impl Comment {
    /// Determine the interned contents of this inline comment.
    ///
    /// This contains the comment wholesale, including the associated comment `#`.
    // FIXME: This is ugly, but it needs extra post-processing steps to handle comment content properly, as well as with
    // its associated Span. Will implement once needed, as right now it bloats the `Comment` struct.
    #[inline]
    pub const fn content(&self) -> InternedStr {
        let &Self(target_content, ..) = self;

        target_content
    }

    /// Determine the span of this inline comment.
    #[inline]
    pub const fn span(&self) -> Span {
        let &Self(.., s0) = self;

        s0
    }
}

impl Spanned for Comment {
    #[inline]
    fn span(&self) -> Span {
        Self::span(self)
    }
}

/// Comment text and source extent established by recognition.
#[derive(Debug)]
// NOTE(invariant): The text is the complete comment slice covered by the span.
pub struct Input<'source>(&'source str, Span);

impl<'source> Lex<'source, str> for Comment {
    type Context = ScanContext<'source>;
    type Error = CommentError;
    type Input<'input>
        = Input<'source>
    where
        'source: 'input;

    #[inline]
    fn pass<'input>(c0: &'input mut Cursor<'source, str>) -> Result<Self::Input<'input>, Self::Error>
    where
        'source: 'input,
    {
        let mut b0 = c0.branch();

        let (.., s0) = b0.expect_is_next_spanned(Fragment::Hash).map_err(|cause| {
            let labels = InlineAnnotations::new(
                Label::new("expected a comment beginning with `#`", Annotated::target(&cause)),
                [],
            );

            CommentError::Expected(Expectation { cause, labels })
        })?;

        let mut terminator = Single::<char>::CR.either(Single::<char>::LF);

        let (content, _) = b0.line_with(&mut terminator);

        let s0 = content.map_or(s0, |content| s0.superset(content));
        let source = b0.source();

        *b0 = Cursor::at(source, s0.end());

        let text = b0.slice(s0).source();

        b0.backport();

        Ok(Input(text, s0))
    }

    #[inline]
    fn build_with_context<'input>(
        Input(text, s0): Self::Input<'input>,
        context: &mut Self::Context,
    ) -> Result<Self, Self::Error>
    where
        'source: 'input,
        Self: Sized,
    {
        context
            .storage_mut()
            .try_store_tagged(text)
            .map(|id| Self(id, s0))
            .ok_or_else(|| {
                let labels =
                    InlineAnnotations::new(Label::new("cannot retain this comment in the storage domain", s0), []);

                CommentError::Storage(Storage { labels })
            })
    }

    #[inline]
    fn lex_with_context(c0: &mut Cursor<'source, str>, context: &mut Self::Context) -> Result<Self, Self::Error> {
        let mut b0 = c0.branch();
        let input = Self::pass(&mut b0)?;
        let value = Self::build_with_context(input, context)?;

        b0.backport();

        Ok(value)
    }
}

/// A comment expectation or storage failure.
#[derive(Debug, Clone, Report)]
#[report(forward)]
pub enum CommentError {
    /// The comment introducer was absent.
    Expected(Expectation),

    /// The comment could not be interned.
    Storage(Storage),
}

impl fmt::Display for CommentError {
    #[inline]
    fn fmt(&self, writer: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expected(error) => fmt::Display::fmt(error, writer),
            Self::Storage(error) => fmt::Display::fmt(error, writer),
        }
    }
}

impl Error for CommentError {
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Expected(error) => Some(error),
            Self::Storage(error) => Some(error),
        }
    }
}

/// A missing comment introducer.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("expected a comment")]
#[error(source(cause))]
#[report(title = "expected a comment", annotations = labels)]
// NOTE(invariant): None.
pub struct Expectation {
    /// Typed expectation and observed source component.
    cause: Expected<char, Fragment>,

    /// Primary failure location followed by related source context.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

/// Storage could not save the original comment text.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("comment storage exhausted")]
#[report(title = "comment storage exhausted", annotations = labels)]
// NOTE(invariant): None.
pub struct Storage {
    /// Primary failure location followed by related source context.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

impl Expectation {
    /// Retrieve the failed lexical assertion and its observed component.
    #[inline]
    pub const fn cause(&self) -> &Expected<char, Fragment> {
        let Self { cause, .. } = self;

        cause
    }
}

#[cfg(test)]
mod preservation_tests {

    use aldebaran::prelude::{Cursor, Lex};

    use crate::scan::{context::ScanContext, trivia::comment::Comment};

    #[test]
    fn comments_reuse_line_terminators_without_absorbing_the_break() {
        for (source, end) in [
            ("#", 1),
            ("# note", 6),
            ("#λ\r\n", 3),
            ("#x\ry", 2),
            ("#\u{2028}x\n", 5),
        ] {
            let mut c0 = Cursor::create(source);
            let mut context = ScanContext::new();
            let comment = Comment::lex_with_context(&mut c0, &mut context).expect("comment");

            assert_eq!(c0.index(), end, "comment must end before CR or LF");
            let text = context
                .storage()
                .try_resolve_tagged(comment.content())
                .expect("comment identity");

            assert_eq!(
                Some(text),
                source.get(comment.span().range()),
                "comment interning must preserve the exact source text"
            );
        }
    }
}

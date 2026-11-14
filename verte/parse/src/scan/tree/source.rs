//! Source-backed token components for Aldebaran's ordinary cursor.

use core::{cell::RefCell, num::NonZero};

use aldebaran::{
    grammar::stream::TokenStream,
    prelude::{Cursor, Span, Spanned},
    source::{component::Component, dissect::SourceDissect, iter::SourceIter, source::Source},
};

use super::{Stream, TokenError, TokenTree, TriviaFragment};

/// Token widths cover the original bytes, including each trivia occurrence.
impl Component for TokenTree {
    #[inline]
    fn size(&self) -> NonZero<usize> {
        self.span().length()
    }
}

/// A lexical failure occupying the remaining source extent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
// NOTE(invariant): The span extends from the failed recognition to the source end.
pub struct Invalid(Span);

impl Spanned for Invalid {
    #[inline]
    fn span(&self) -> Span {
        let &Self(span) = self;

        span
    }
}

/// A token source sharing its text and interning domain with nested scopes.
// NOTE(invariant): All tokens and any recorded lexical failure belong to `stream`'s text.
pub struct Tokens<'s> {
    /// The original source and its lexical storage domain.
    stream: Stream<'s>,

    /// The first lexical failure encountered by a demanded read.
    error: RefCell<Option<TokenError>>,
}

impl<'s> Tokens<'s> {
    /// Expose a lexical scope without recognizing its contents in advance.
    #[inline]
    pub const fn new(stream: Stream<'s>) -> Self {
        Self {
            stream,
            error: RefCell::new(None),
        }
    }

    /// Borrow the lexical scope and its storage domain.
    #[inline]
    pub const fn stream(&self) -> &Stream<'s> {
        let Self { stream, .. } = self;

        stream
    }

    /// Trivia ignored by the syntax view of this scope.
    #[inline]
    pub const fn trivia(&self) -> TriviaFragment {
        match self.stream().scope() {
            Some(_) => TriviaFragment::All,
            None => TriviaFragment::Horizontal,
        }
    }

    /// Preserve a demanded lexical failure when transporting a cursor result.
    #[inline]
    pub fn checked<T>(&self, value: T) -> Result<T, TokenError> {
        let Self { error, .. } = self;

        match &*error.borrow() {
            Some(error) => Err(error.clone()),
            None => Ok(value),
        }
    }
}

impl<'s: 'a, 'a> Source<'a> for Tokens<'s> {
    type Component = TokenTree;

    #[inline]
    fn footprint(&self) -> Option<Span> {
        NonZero::new(self.stream().source().len()).map(|size| Span::new(0, size))
    }
}

/// A UTF-8 byte window whose tokens are recognized within that window.
///
/// A window can split an outer token at a scalar boundary. Its contents are then
/// lexed independently, with byte spans measured from the start of the original text.
// NOTE(invariant): `start..end` lies in `source` and both endpoints are UTF-8 boundaries.
pub struct View<'a, 's> {
    /// The source owning the text and any lexical failure.
    source: &'a Tokens<'s>,

    /// First byte of this window.
    start: usize,

    /// Exclusive byte boundary of this window.
    end: usize,
}

impl<'s: 'a, 'a> SourceDissect<'a> for Tokens<'s> {
    type View<'b>
        = View<'b, 's>
    where
        Self: 'b;

    /// Select an independently lexed window in original UTF-8 byte coordinates.
    ///
    /// # Panics
    ///
    /// Panics when the window is outside the text or splits a UTF-8 scalar.
    #[inline]
    fn dissect(&self, span: Span) -> Self::View<'_> {
        let start = span.start();
        let end = span.end();
        let text = self.stream().source();

        assert!(
            text.get(start..end).is_some(),
            "token source windows require valid UTF-8 byte boundaries"
        );

        View {
            source: self,
            start,
            end,
        }
    }
}

impl<'s: 'a, 'a> SourceIter<'a> for Tokens<'s> {
    type Iterator<'b>
        = Iter<'b, 's>
    where
        Self: 'b;

    #[inline]
    fn iter(&self) -> Self::Iterator<'_> {
        View {
            source: self,
            start: 0,
            end: self.stream().source().len(),
        }
        .iter()
    }
}

impl<'a: 'b, 's: 'a, 'b> Source<'b> for View<'a, 's> {
    type Component = TokenTree;

    #[inline]
    fn footprint(&self) -> Option<Span> {
        let Self { start, end, .. } = self;

        NonZero::new(end - start).map(|size| Span::new(0, size))
    }
}

impl<'a: 'b, 's: 'a, 'b> SourceIter<'b> for View<'a, 's> {
    type Iterator<'c>
        = Iter<'a, 's>
    where
        Self: 'c;

    #[inline]
    fn iter(&self) -> Self::Iterator<'_> {
        let &Self { source, start, end } = self;

        let stream = source.stream();
        let text = &stream.source()[..end];
        let stream = Stream::new(Cursor::at(text, start), stream.context().clone(), stream.scope());

        Iter {
            source,
            stream,
            end,
            failed: false,
        }
    }
}

/// Lazy lexical traversal of a source window.
// NOTE(invariant): The stream remains within `end`; a failed read yields the remaining extent once.
pub struct Iter<'a, 's> {
    /// Source that records the first lexical failure.
    source: &'a Tokens<'s>,

    /// Lexical producer for this window.
    stream: Stream<'s>,

    /// Exclusive byte boundary.
    end: usize,

    /// Whether this iterator already exposed a lexical failure.
    failed: bool,
}

impl Iterator for Iter<'_, '_> {
    type Item = TokenTree;

    #[inline]
    fn next(&mut self) -> Option<TokenTree> {
        let Self {
            source,
            stream,
            end,
            failed,
        } = self;

        if *failed {
            return None;
        }

        let start = stream.cursor.index();

        match stream.next() {
            Ok(token) => token,
            Err(error) => {
                let size = NonZero::new(*end - start)?;
                let span = Span::new(start, size);

                source.error.borrow_mut().get_or_insert(error);
                *failed = true;

                Some(TokenTree::Invalid(Invalid(span)))
            },
        }
    }
}

impl<'s, 'a> SourceDissect<'a> for &'a Tokens<'s> {
    type View<'b>
        = View<'b, 's>
    where
        Self: 'b;

    #[inline]
    fn dissect(&self, span: Span) -> Self::View<'_> {
        Tokens::dissect(self, span)
    }
}

#[cfg(test)]
mod tests {
    use aldebaran::{
        prelude::{Cursor, Span, Spanned},
        source::{component::Component, dissect::SourceDissect, iter::SourceIter, source::Source},
    };

    use super::Tokens;
    use crate::scan::{
        context::Shared,
        tree::{Stream, TokenTree},
    };

    #[test]
    fn tokens_and_windows_preserve_the_original_utf8_byte_plane() {
        let text = "name # λ\r\n\"β\"";
        let tokens = Tokens::new(Stream::new(Cursor::create(text), Shared::new(), None));
        let width = tokens.iter().map(|token| token.size().get()).sum::<usize>();

        assert_eq!(width, tokens.size());
        assert_eq!(width, text.len());

        let start = text.find('"').expect("quoted occurrence");
        let span = Span::unit(start).superset(Span::unit(text.len() - 1));
        let view = tokens.dissect(span);
        let literal = view.iter().next().expect("literal component");

        assert!(matches!(literal, TokenTree::String(_)));
        assert_eq!(literal.span(), span);
        assert_eq!(view.size(), span.length().get());
    }

    #[test]
    fn ordinary_cursor_branches_restore_byte_positions() {
        let text = "one two";
        let tokens = Tokens::new(Stream::new(Cursor::create(text), Shared::new(), None));
        let source = &tokens;
        let mut c0 = Cursor::create(&source);

        {
            let mut b0 = c0.branch();
            let first = b0.any().expect("first identifier");

            assert_eq!(first.span().range(), 0..3);

            let mut b1 = b0.branch();

            b1.any();
            b1.any();
            b1.backport();
        }

        assert_eq!(c0.index(), 0);
        assert_eq!(c0.any().expect("replayed identifier").span().range(), 0..3);
        assert_eq!(c0.index(), 3);
    }

    #[test]
    fn lexical_failures_are_demanded_and_retain_the_remaining_footprint() {
        let text = "name \\";
        let tokens = Tokens::new(Stream::new(Cursor::create(text), Shared::new(), None));
        let source = &tokens;
        let mut c0 = Cursor::create(&source);

        assert!(matches!(c0.any(), Some(TokenTree::Ident(_))));
        assert!(tokens.checked(()).is_ok());
        assert!(matches!(c0.any(), Some(TokenTree::Trivia(_))));
        assert!(tokens.checked(()).is_ok());

        let failure = c0.any().expect("failed lexical component");

        assert!(matches!(failure, TokenTree::Invalid(_)));
        assert_eq!(failure.span().range(), 5..text.len());
        assert_eq!(c0.index(), tokens.size());
        assert!(tokens.checked(()).is_err());
    }

    #[test]
    fn later_windows_preserve_the_first_lexical_failure() {
        let text = "\\ 1e";
        let tokens = Tokens::new(Stream::new(Cursor::create(text), Shared::new(), None));

        assert!(matches!(tokens.iter().next(), Some(TokenTree::Invalid(_))));
        assert!(matches!(tokens.checked(()), Err(super::TokenError::Unexpected(_))));

        let tail = tokens.dissect(Span::unit(2).superset(Span::unit(3)));

        assert!(matches!(tail.iter().next(), Some(TokenTree::Invalid(_))));
        assert!(matches!(tokens.checked(()), Err(super::TokenError::Unexpected(_))));
    }

    #[test]
    fn scalar_aligned_windows_are_lexed_independently() {
        let tokens = Tokens::new(Stream::new(Cursor::create("name"), Shared::new(), None));
        let window = tokens.dissect(Span::unit(1));
        let token = window.iter().next().expect("identifier in the selected byte window");

        assert!(matches!(token, TokenTree::Ident(_)));
        assert_eq!(token.span(), Span::unit(1));
        assert!(tokens.checked(()).is_ok());
    }

    #[test]
    #[should_panic(expected = "token source windows require valid UTF-8 byte boundaries")]
    fn windows_cannot_split_a_utf8_scalar() {
        let tokens = Tokens::new(Stream::new(Cursor::create("β"), Shared::new(), None));

        tokens.dissect(Span::unit(1));
    }

    #[test]
    #[should_panic(expected = "token source windows require valid UTF-8 byte boundaries")]
    fn windows_cannot_extend_beyond_the_source() {
        let tokens = Tokens::new(Stream::new(Cursor::create("name"), Shared::new(), None));

        tokens.dissect(Span::unit(4));
    }
}

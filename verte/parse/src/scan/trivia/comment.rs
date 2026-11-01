//! Comments found inside Verte source text.

use aldebaran::{
    baseline::span::{Span, Spanned},
    ice::Ice,
    id::ident::TaggedId,
    interner::storage::StorageId,
    logic::connective::{Connective, Not, OneOf},
    prelude::{Assert, Cursor},
    text::{
        builtin::Ascii,
        cursor::{consume::Lex, error::Expected},
    },
};

use crate::scan::context::ScanContext;

/// A textual fragment of a comment.
///
/// This is an [`Assert`] that is used to match the inline comment.
#[derive(Debug, Copy, Clone, Assert)]
pub enum Fragment {
    /// A `#` character that starts an inline comment.
    #[assert(use Ascii::Hash type Ascii)]
    Hash,

    /// A fragment that that determines the rest of the comment.
    ///
    /// This represents anything, except a newline character or a carriage return.
    ///
    /// This way, the inline contents of the comment is a single line without any line
    /// delimiter.
    #[assert(use OneOf::these([Ascii::LineFeed, Ascii::CarriageReturn]).not() type Not<OneOf<Ascii, 2>>)]
    Rest,
}

/// A type alias for an tagged interned [`str`].
type InternedStr = TaggedId<str, StorageId>;

/// An inline comment.
///
/// This is an inline comment inside a `Verte` source code: `# This is an inline comment.`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct Comment(InternedStr, Span);

impl Comment {
    /// Determine the interned contents of this inline comments.
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

impl<'source> Lex<'source, str> for Comment {
    type Context = ScanContext<'source>;
    type Error = Expected<char, Fragment>;
    type Input<'input>
        = (&'input Cursor<'source, str>, Span)
    where
        'source: 'input;

    #[inline]
    fn pass<'input>(c0: &'input mut Cursor<'source, str>) -> Result<Self::Input<'input>, Self::Error>
    where
        'source: 'input,
    {
        let mut b0 = c0.branch();

        let (.., s0) = b0.expect_is_next_spanned(Fragment::Hash)?;

        let s0 = b0.ignore_while_spanned(Fragment::Rest).map_or(s0, |s1| s0.superset(s1));

        b0.backport();

        Ok((c0, s0))
    }

    #[inline]
    fn build_with_context<'input>(
        (c0, s0): Self::Input<'input>,
        context: &mut Self::Context,
    ) -> Result<Self, Self::Error>
    where
        'source: 'input,
        Self: Sized,
    {
        Ok(Self(
            Ice::some(context.storage_mut().try_store_tagged(c0.slice(s0).source())),
            s0,
        ))
    }
}

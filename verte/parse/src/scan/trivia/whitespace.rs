//! Handling of whitespace inside the lexical scanner.

use aldebaran::{
    prelude::{Cursor, Lex, Span, Spanned},
    text::{builtin::property, cursor::error::Expected},
};

/// A whitespace token.
///
/// This is a contiguous run of Unicode Whitespace characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
// NOTE(invariant): The cursor run over the `Span` was composed of Unicode Whitespace
// codepoints.
pub struct Whitespace(Span);

impl<'source> Lex<'source, str> for Whitespace {
    type Context = ();
    type Error = Expected<char, property::Whitespace>;
    type Input<'input>
        = Span
    where
        'source: 'input;

    fn pass<'input>(c0: &'input mut Cursor<'source, str>) -> Result<Self::Input<'input>, Self::Error>
    where
        'source: 'input,
    {
        let (.., s0) = c0.expect_is_next_spanned(property::Whitespace)?;

        Ok(c0
            .ignore_while_spanned(property::Whitespace)
            .map_or(s0, |s1| s0.superset(s1)))
    }

    fn build_with_context<'input>(target_value: Self::Input<'input>, _: &mut Self::Context) -> Result<Self, Self::Error>
    where
        'source: 'input,
        Self: Sized,
    {
        Ok(Self(target_value))
    }
}

impl Spanned for Whitespace {
    #[inline]
    fn span(&self) -> Span {
        let &Self(target_value) = self;

        target_value
    }
}

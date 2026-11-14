//! Physical LF and indivisible CRLF line endings.

use aldebaran::{
    logic::connective::Or,
    prelude::{Annotated, Assert, Cursor, Lex, Report, Span, Spanned},
    report::annotated::{InlineAnnotations, label::Label},
    source::terminate::Standard,
    text::{builtin::Char, cursor::error::Expected},
};
use fack::prelude::Error;

/// Physical line-ending component expectations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Assert)]
pub enum Fragment {
    /// The start of LF or CRLF.
    #[assert(use Or::one(Char('\n'), Char('\r')) type Or<Char, Char>)]
    Start,
}

/// A source-backed line break, including both bytes when spelled CRLF.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
// NOTE(invariant): The span covers exactly LF or the entire CRLF sequence.
pub struct LineBreak(Span);

impl Spanned for LineBreak {
    #[inline]
    fn span(&self) -> Span {
        let &Self(span) = self;

        span
    }
}

/// Source extent established by successful recognition of [`LineBreak`].
#[derive(Debug)]
// NOTE(invariant): The span was recognized by `LineBreak::pass`.
pub struct Input(Span);

impl<'source> Lex<'source, str> for LineBreak {
    type Context = ();
    type Error = LineBreakError;
    type Input<'input>
        = Input
    where
        'source: 'input;

    #[inline]
    fn pass<'input>(c0: &'input mut Cursor<'source, str>) -> Result<Input, LineBreakError>
    where
        'source: 'input,
    {
        let mut b0 = c0.branch();

        b0.validate_is_next(Fragment::Start).map_err(|cause| {
            let labels = InlineAnnotations::new(Label::new("expected LF or CRLF", Annotated::target(&cause)), []);

            LineBreakError::Expected(Expectation { cause, labels })
        })?;

        let start = b0.index();
        let mut terminator = Standard::<char>::new();
        let span = match b0.line_with(&mut terminator) {
            (None, Some(line_break)) => line_break.span(),
            _ => {
                let labels = InlineAnnotations::new(
                    Label::new("a lone CR is not a line break; use LF or CRLF", Span::unit(start)),
                    [],
                );

                return Err(LineBreakError::LoneCr(LoneCr { labels }));
            },
        };

        b0.backport();

        Ok(Input(span))
    }

    #[inline]
    fn build_with_context<'input>(Input(span): Input, _: &mut ()) -> Result<Self, LineBreakError>
    where
        'source: 'input,
    {
        Ok(Self(span))
    }
}

/// A missing or malformed physical line ending.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[report(forward)]
pub enum LineBreakError {
    /// A physical line ending was required.
    #[error(transparent(0))]
    Expected(Expectation),

    /// A carriage return was not followed immediately by LF.
    #[error(transparent(0))]
    LoneCr(LoneCr),
}

/// A physical line ending was required here.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("expected a physical line ending")]
#[error(source(cause))]
#[report(title = "expected a physical line ending", annotations = labels)]
// NOTE(invariant): None.
pub struct Expectation {
    /// Required line-ending start and the scalar or boundary found instead.
    cause: Expected<char, Fragment>,

    /// The location requiring a physical line ending.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

impl Expectation {
    /// Retrieve the failed physical-line-ending expectation.
    #[inline]
    pub const fn cause(&self) -> &Expected<char, Fragment> {
        let Self { cause, .. } = self;

        cause
    }
}

/// A carriage return that cannot form an LF or CRLF line ending.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("invalid line ending")]
#[report(title = "invalid line ending", annotations = labels)]
// NOTE(invariant): None.
pub struct LoneCr {
    /// The carriage return requiring an immediately following LF.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

#[cfg(test)]
mod tests {
    use core::error::Error;

    use aldebaran::prelude::Annotated;

    use super::*;

    #[test]
    fn physical_endings_and_rollback() {
        for (text, end) in [("\n", 1), ("\r\n", 2)] {
            let mut c0 = Cursor::create(text);

            assert_eq!(LineBreak::lex(&mut c0).expect("line ending").span().range(), 0..end);
        }

        let mut c0 = Cursor::create("\rx");

        assert!(LineBreak::lex(&mut c0).is_err());

        assert_eq!(c0.index(), 0);
    }
    #[test]
    fn missing_endings_and_lone_carriage_returns_are_distinct() {
        let mut c0 = Cursor::create("x");

        let LineBreakError::Expected(error) = LineBreak::lex(&mut c0).expect_err("a physical ending was required")
        else {
            unreachable!("the initial line-ending assertion must be retained")
        };

        assert_eq!(Annotated::target(error.cause()).range(), 0..1);

        assert!(Error::source(&error).is_some());

        assert_eq!(c0.index(), 0);

        let mut c0 = Cursor::create("\r");

        let LineBreakError::LoneCr(error) = LineBreak::lex(&mut c0).expect_err("a lone CR is unsupported") else {
            unreachable!("a lone CR differs from a missing line-ending start")
        };

        assert_eq!(error.annotations().primary().target().range(), 0..1);

        assert_eq!(c0.index(), 0);
    }
}

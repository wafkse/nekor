//! Escape sequences that denote a Unicode scalar.

use core::fmt;

use aldebaran::{
    ice::Ice,
    logic::connective::OneOf,
    prelude::{Annotated, Assert, Cursor, Lex, Report, Span, Spanned},
    report::annotated::{InlineAnnotations, label::Label},
    text::{
        builtin::{Char, property},
        cursor::error::Expected,
    },
};
use fack::prelude::Error;

/// Escape component expectations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Assert)]
pub enum Fragment {
    /// The backslash introducer.
    #[assert(use Char('\\') type Char)]
    Start,

    /// A supported escape selector.
    #[assert(
        use OneOf::these([
            Char('"'),
            Char('\\'),
            Char('n'),
            Char('r'),
            Char('t'),
            Char('0'),
            Char('u'),
        ])
        type OneOf<Char, 7>
    )]
    Selector,

    /// Opening brace of a scalar escape.
    #[assert(use Char('{') type Char)]
    Open,

    /// A hexadecimal scalar digit.
    #[assert(use property::AsciiHexadecimal type property::AsciiHexadecimal)]
    Digit,

    /// Closing brace of a scalar escape.
    #[assert(use Char('}') type Char)]
    Close,
}

/// A scalar encoded by an escape sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
// NOTE(invariant): `scalar` is the decoding of the escape covered by `span`.
pub struct Escape {
    /// Unicode scalar denoted by this escape.
    scalar: char,

    /// Original source byte extent.
    span: Span,
}

impl Escape {
    /// Retrieve the denoted scalar.
    #[inline]
    pub const fn scalar(&self) -> char {
        let &Self { scalar, .. } = self;

        scalar
    }
}

impl Spanned for Escape {
    #[inline]
    fn span(&self) -> Span {
        let &Self { span, .. } = self;

        span
    }
}

/// Invalid escape syntax or scalar value.
#[derive(Debug, Clone, Report)]
#[report(forward)]
pub enum EscapeError {
    /// A required component was absent.
    Expected(Expectation),

    /// The hexadecimal value is not a Unicode scalar.
    Scalar(Scalar),
}

impl fmt::Display for EscapeError {
    #[inline]
    fn fmt(&self, writer: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expected(error) => fmt::Display::fmt(error, writer),
            Self::Scalar(error) => fmt::Display::fmt(error, writer),
        }
    }
}

impl Error for EscapeError {
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Expected(error) => Some(error),
            Self::Scalar(error) => Some(error),
        }
    }
}

/// An unmet escape component expectation.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("invalid escape sequence")]
#[error(source(cause))]
#[report(title = "invalid escape sequence", annotations = labels)]
// NOTE(invariant): Both annotations belong to the same escape.
pub struct Expectation {
    /// Typed expectation and observed source component.
    cause: Expected<char, Fragment>,

    /// Primary failure location followed by related source context.
    labels: InlineAnnotations<Label<&'static str>, 1>,
}

/// A surrogate or value above the Unicode scalar range.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("invalid Unicode scalar escape")]
#[report(title = "invalid Unicode scalar escape", annotations = labels)]
// NOTE(invariant): None.
pub struct Scalar {
    /// Primary failure location followed by related source context.
    labels: InlineAnnotations<Label<&'static str>, 1>,
}

/// An escape's decoded scalar and extent established by recognition.
#[derive(Debug)]
// NOTE(invariant): The scalar is the decoding of the escape covered by the span.
pub struct Input {
    /// Decoded Unicode scalar.
    scalar: char,

    /// Absolute source extent of the escape.
    span: Span,
}

impl<'s> Lex<'s, str> for Escape {
    type Context = ();
    type Error = EscapeError;
    type Input<'i>
        = Input
    where
        's: 'i;

    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "Keep substantial recognition or construction out of line across grammar entry points."
    )]
    fn pass<'i>(c0: &'i mut Cursor<'s, str>) -> Result<Self::Input<'i>, EscapeError>
    where
        's: 'i,
    {
        let mut b0 = c0.branch();
        let start = Span::unit(b0.index());

        b0.expect_is_next(Fragment::Start).map_err(|cause| {
            let labels = InlineAnnotations::new(
                Label::new("expected `\\` to introduce an escape", Annotated::target(&cause)),
                [Label::new("escape starts here", start)],
            );

            EscapeError::Expected(Expectation { cause, labels })
        })?;

        let selector = b0.expect_is_next(Fragment::Selector).map_err(|cause| {
            let labels = InlineAnnotations::new(
                Label::new(
                    "supported escapes are quote, backslash, `n`, `r`, `t`, `0`, and `u{hex}`",
                    Annotated::target(&cause),
                ),
                [Label::new("escape starts here", start)],
            );

            EscapeError::Expected(Expectation { cause, labels })
        })?;
        let scalar = match selector {
            '"' => '"',
            '\\' => '\\',
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            '0' => '\0',
            'u' => {
                b0.expect_is_next(Fragment::Open).map_err(|cause| {
                    let labels = InlineAnnotations::new(
                        Label::new("expected `{` after `\\u`", Annotated::target(&cause)),
                        [Label::new("Unicode escape starts here", start)],
                    );

                    EscapeError::Expected(Expectation { cause, labels })
                })?;

                let first = b0.expect_is_next(Fragment::Digit).map_err(|cause| {
                    let labels = InlineAnnotations::new(
                        Label::new(
                            "a Unicode escape requires at least one hexadecimal digit",
                            Annotated::target(&cause),
                        ),
                        [Label::new("Unicode escape starts here", start)],
                    );

                    EscapeError::Expected(Expectation { cause, labels })
                })?;
                let mut magnitude = Ice::some(first.to_digit(16));

                while let Some(digit) = b0.optional(Fragment::Digit) {
                    let digit = Ice::some(digit.to_digit(16));

                    magnitude = magnitude
                        .checked_mul(16)
                        .and_then(|value| value.checked_add(digit))
                        .ok_or_else(|| {
                            let target = start.superset(Span::unit(b0.index() - 1));
                            let labels = InlineAnnotations::new(
                                Label::new("this value exceeds U+10FFFF", target),
                                [Label::new("Unicode escape starts here", start)],
                            );

                            EscapeError::Scalar(Scalar { labels })
                        })?;
                }
                b0.expect_is_next(Fragment::Close).map_err(|cause| {
                    let labels = InlineAnnotations::new(
                        Label::new("expected `}` after the hexadecimal digits", Annotated::target(&cause)),
                        [Label::new("Unicode escape starts here", start)],
                    );

                    EscapeError::Expected(Expectation { cause, labels })
                })?;
                char::from_u32(magnitude).ok_or_else(|| {
                    let target = start.superset(Span::unit(b0.index() - 1));
                    let labels = InlineAnnotations::new(
                        Label::new("surrogates and values above U+10FFFF are not Unicode scalars", target),
                        [Label::new("Unicode escape starts here", start)],
                    );

                    EscapeError::Scalar(Scalar { labels })
                })?
            },
            _ => Ice::impossible(None),
        };
        let span = start.superset(Span::unit(b0.index() - 1));

        b0.backport();

        Ok(Input { scalar, span })
    }

    #[inline]
    fn build_with_context<'i>(Input { scalar, span }: Self::Input<'i>, _: &mut ()) -> Result<Self, EscapeError>
    where
        's: 'i,
    {
        Ok(Self { scalar, span })
    }
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
mod tests {
    use super::*;

    #[test]
    fn unicode_and_failures() {
        for (source, expected) in [("\\n", '\n'), ("\\u{1f408}", '🐈'), ("\\u{00000000000041}", 'A')] {
            let mut c0 = Cursor::create(source);

            assert_eq!(Escape::lex(&mut c0).expect("escape").scalar(), expected);
        }
        for source in ["\\q", "\\u{}", "\\u{d800}", "\\u{110000}", "\\u{ffffffffff}", "\\u{a"] {
            let mut c0 = Cursor::create(source);

            assert!(Escape::lex(&mut c0).is_err());

            assert_eq!(c0.index(), 0);
        }
    }
}

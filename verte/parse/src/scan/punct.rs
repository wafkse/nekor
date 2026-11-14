//! Punctuation tokens and their lexical family.

use core::fmt;

use aldebaran::{
    ice::Ice,
    logic::connective::{Not, Or},
    prelude::{Annotated, Assert, Cursor, Lex, Report, Span, Spanned},
    print::visualize::Visualize,
    report::annotated::{InlineAnnotations, label::Label},
    text::{builtin::Char, cursor::error::Expected},
};
use fack::prelude::Error;

/// Character and boundary expectations for punctuation tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Assert)]
pub enum Fragment {
    /// The `.` character.
    #[assert(use Char('.') type Char)]
    Dot,

    /// The `=` character.
    #[assert(use Char('=') type Char)]
    Equals,

    /// The `+` character.
    #[assert(use Char('+') type Char)]
    Plus,

    /// The `!` character.
    #[assert(use Char('!') type Char)]
    Bang,

    /// The `<` character.
    #[assert(use Char('<') type Char)]
    Less,

    /// The `>` character.
    #[assert(use Char('>') type Char)]
    Greater,

    /// The `*` character.
    #[assert(use Char('*') type Char)]
    Star,

    /// The `&` character.
    #[assert(use Char('&') type Char)]
    Ampersand,

    /// The `|` character.
    #[assert(use Char('|') type Char)]
    Pipe,

    /// The `(` character.
    #[assert(use Char('(') type Char)]
    LeftParen,

    /// The `)` character.
    #[assert(use Char(')') type Char)]
    RightParen,

    /// The `[` character.
    #[assert(use Char('[') type Char)]
    LeftBracket,

    /// The `]` character.
    #[assert(use Char(']') type Char)]
    RightBracket,

    /// The `{` character.
    #[assert(use Char('{') type Char)]
    LeftBrace,

    /// The `}` character.
    #[assert(use Char('}') type Char)]
    RightBrace,

    /// The `:` character.
    #[assert(use Char(':') type Char)]
    Colon,

    /// The `,` character.
    #[assert(use Char(',') type Char)]
    Comma,

    /// The `;` character.
    #[assert(use Char(';') type Char)]
    Semicolon,

    /// The `-` character.
    #[assert(use Char('-') type Char)]
    Minus,

    /// The `/` character.
    #[assert(use Char('/') type Char)]
    Slash,

    /// The `%` character.
    #[assert(use Char('%') type Char)]
    Percent,

    /// The `^` character.
    #[assert(use Char('^') type Char)]
    Caret,

    /// The `~` character.
    #[assert(use Char('~') type Char)]
    Tilde,

    /// The `@` character.
    #[assert(use Char('@') type Char)]
    At,

    /// The maximal `Dot` token boundary.
    #[assert(use Not::that(Char('.')) type Not<Char>)]
    DotBoundary,

    /// The maximal `DotDot` token boundary.
    #[assert(use Not::that(Or::one(Char('='), Char('+'))) type Not<Or<Char, Char>>)]
    DotDotBoundary,

    /// The maximal `Eq` token boundary.
    #[assert(use Not::that(Char('=')) type Not<Char>)]
    EqBoundary,

    /// The maximal `Bang` token boundary.
    #[assert(use Not::that(Char('=')) type Not<Char>)]
    BangBoundary,

    /// The maximal `Lt` token boundary.
    #[assert(use Not::that(Or::one(Char('<'), Char('='))) type Not<Or<Char, Char>>)]
    LtBoundary,

    /// The maximal `Gt` token boundary.
    #[assert(use Not::that(Or::one(Char('>'), Char('='))) type Not<Or<Char, Char>>)]
    GtBoundary,

    /// The maximal `Star` token boundary.
    #[assert(use Not::that(Char('*')) type Not<Char>)]
    StarBoundary,

    /// The maximal `Amp` token boundary.
    #[assert(use Not::that(Char('&')) type Not<Char>)]
    AmpBoundary,

    /// The maximal `Pipe` token boundary.
    #[assert(use Not::that(Char('|')) type Not<Char>)]
    PipeBoundary,
}

/// A punctuation character or token boundary was invalid.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("expected punctuation")]
#[error(source(cause))]
#[report(title = "expected punctuation", annotations = labels)]
// NOTE(invariant): Related context identifies the start of the attempted punctuation token.
pub struct PunctuationError {
    /// Expected component and observed input.
    cause: Expected<char, Fragment>,

    /// Failure location and punctuation context.
    labels: InlineAnnotations<Label<&'static str>, 1>,
}

/// Declare fixed punctuation tokens and their lexical family.
macro_rules! fixed {
    () => {};
    ($(
        $(#[$meta:meta])*

        $vis:vis struct $name:ident = $text:literal => [$($component:ident),+] $(/ $boundary:ident)?;
    )+) => {
        tokel::stream! {
            $(
                $(#[$meta])*
                #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
                // NOTE: The span covers the complete declared token text.
                $vis struct $name(Span);

                impl Spanned for $name {
                    #[inline]
                    fn span(&self) -> Span {
                        let &Self(span) = self;

                        span
                    }
                }

                impl<'s> Lex<'s, str> for $name {
                    type Context = ();
                    type Error = PunctuationError;
                    type Input<'i> = Span where 's: 'i;

                    #[inline]
                    fn pass<'i>(c0: &'i mut Cursor<'s, str>) -> Result<Span, PunctuationError>
                    where
                        's: 'i,
                    {
                        let mut b0 = c0.branch();
                        let start = Span::unit(b0.index());

                        for fragment in [$(Fragment::$component),+] {
                            b0.expect_is_next(fragment).map_err(|cause| {
                                let target = Annotated::target(&cause);
                                let primary = Label::new(concat!("expected `", $text, "`"), target);
                                let labels = InlineAnnotations::new(
                                    primary,
                                    [Label::new("this punctuation starts here", start)],
                                );

                                PunctuationError { cause, labels }
                            })?;
                        }
                        $(
                            b0.validate_is_next_or_eof(Fragment::$boundary).map_err(|cause| {
                                let target = Annotated::target(&cause);
                                let primary = Label::new(
                                    concat!("expected the boundary after `", $text, "`"),
                                    target,
                                );
                                let labels = InlineAnnotations::new(
                                    primary,
                                    [Label::new("this punctuation starts here", start)],
                                );

                                PunctuationError { cause, labels }
                            })?;
                        )?
                        let span = start.superset(Span::unit(b0.index() - 1));

                        b0.backport();

                        Ok(span)
                    }

                    #[inline]
                    fn build_with_context<'i>(span: Span, _: &mut ()) -> Result<Self, PunctuationError>
                    where
                        's: 'i,
                    {
                        Ok(Self(span))
                    }
                }
            )+

            /// Concrete punctuation alternatives.
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
            pub enum Punctuation {
                $(
                    $(#[$meta])*
                    $name($name),
                )+
            }

            impl Spanned for Punctuation {
                #[inline]
                fn span(&self) -> Span {
                    match self {
                        $(Self::$name(value) => value.span(),)+
                    }
                }
            }

            impl Visualize for Punctuation {
                #[inline]
                fn visualize<W>(&self, writer: &mut W) -> fmt::Result
                where
                    W: fmt::Write,
                {
                    match self {
                        $(Self::$name(_) => writer.write_str($text),)+
                    }
                }
            }

            /// A recognized punctuation token awaiting construction.
            #[derive(Debug)]
            pub enum Input {
                $(
                    $(#[$meta])*
                    $name(Span),
                )+
            }

            impl<'s> Lex<'s, str> for Punctuation {
                type Context = ();
                type Error = PunctuationError;
                type Input<'i> = Input where 's: 'i;

                #[inline]
                fn pass<'i>(c0: &'i mut Cursor<'s, str>) -> Result<Input, PunctuationError>
                where
                    's: 'i,
                {
                    // NOTE: Alternatives are declared longest first. Each failed pass rolls back.
                    let mut failure = None;

                    $(
                        match $name::pass(c0) {
                            Ok(span) => return Ok(Input::$name(span)),
                            Err(error) => {
                                if failure.is_none() {
                                    failure = Some(error);
                                }
                            },
                        }
                    )+
                    Err(Ice::some(failure))
                }

                #[inline]
                fn build_with_context<'i>(input: Input, _: &mut ()) -> Result<Self, PunctuationError>
                where
                    's: 'i,
                {
                    Ok(match input {
                        $(Input::$name(span) => Self::$name($name(span)),)+
                    })
                }
            }
        }
    };
}
fixed! {
    /// The `..=` occurrence.
    pub struct DotDotEq = "..=" => [Dot,Dot,Equals];

    /// The `..+` occurrence.
    pub struct DotDotPlus = "..+" => [Dot,Dot,Plus];

    /// The `..` occurrence.
    pub struct DotDot = ".." => [Dot,Dot] / DotDotBoundary;

    /// The `==` occurrence.
    pub struct EqEq = "==" => [Equals,Equals];

    /// The `!=` occurrence.
    pub struct BangEq = "!=" => [Bang,Equals];

    /// The `<=` occurrence.
    pub struct LtEq = "<=" => [Less,Equals];

    /// The `>=` occurrence.
    pub struct GtEq = ">=" => [Greater,Equals];

    /// The `<<` occurrence.
    pub struct LtLt = "<<" => [Less,Less];

    /// The `>>` occurrence.
    pub struct GtGt = ">>" => [Greater,Greater];

    /// The `**` occurrence.
    pub struct StarStar = "**" => [Star,Star];

    /// The `&&` occurrence.
    pub struct AmpAmp = "&&" => [Ampersand,Ampersand];

    /// The `||` occurrence.
    pub struct PipePipe = "||" => [Pipe,Pipe];

    /// The `(` occurrence.
    pub struct LeftParen = "(" => [LeftParen];

    /// The `)` occurrence.
    pub struct RightParen = ")" => [RightParen];

    /// The `[` occurrence.
    pub struct LeftBracket = "[" => [LeftBracket];

    /// The `]` occurrence.
    pub struct RightBracket = "]" => [RightBracket];

    /// The `{` occurrence.
    pub struct LeftBrace = "{" => [LeftBrace];

    /// The `}` occurrence.
    pub struct RightBrace = "}" => [RightBrace];

    /// The `.` occurrence.
    pub struct Dot = "." => [Dot] / DotBoundary;

    /// The `:` occurrence.
    pub struct Colon = ":" => [Colon];

    /// The `,` occurrence.
    pub struct Comma = "," => [Comma];

    /// The `;` occurrence.
    pub struct Semicolon = ";" => [Semicolon];

    /// The `=` occurrence.
    pub struct Eq = "=" => [Equals] / EqBoundary;

    /// The `<` occurrence.
    pub struct Lt = "<" => [Less] / LtBoundary;

    /// The `>` occurrence.
    pub struct Gt = ">" => [Greater] / GtBoundary;

    /// The `+` occurrence.
    pub struct Plus = "+" => [Plus];

    /// The `-` occurrence.
    pub struct Minus = "-" => [Minus];

    /// The `*` occurrence.
    pub struct Star = "*" => [Star] / StarBoundary;

    /// The `/` occurrence.
    pub struct Slash = "/" => [Slash];

    /// The `%` occurrence.
    pub struct Percent = "%" => [Percent];

    /// The `^` occurrence.
    pub struct Caret = "^" => [Caret];

    /// The `!` occurrence.
    pub struct Bang = "!" => [Bang] / BangBoundary;

    /// The `~` occurrence.
    pub struct Tilde = "~" => [Tilde];

    /// The `&` occurrence.
    pub struct Amp = "&" => [Ampersand] / AmpBoundary;

    /// The `|` occurrence.
    pub struct Pipe = "|" => [Pipe] / PipeBoundary;

    /// The `@` occurrence.
    pub struct At = "@" => [At];
}

impl PunctuationError {
    /// Retrieve the failed lexical assertion and observed component.
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
    fn punctuation_is_maximal() {
        for source in ["..", "..=", "..+"] {
            let mut c0 = Cursor::create(source);

            assert!(Dot::lex(&mut c0).is_err());

            assert_eq!(c0.index(), 0);
        }
        for source in ["..+", "..=", "..", ">=", ">>", "**", "&&", "||", "!="] {
            let mut c0 = Cursor::create(source);

            assert_eq!(
                Punctuation::lex(&mut c0).expect("punctuation").span().range(),
                0..source.len()
            );
        }
    }
}

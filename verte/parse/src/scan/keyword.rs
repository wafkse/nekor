//! Keyword tokens and their lexical family.

use core::fmt;

use aldebaran::{
    ice::Ice,
    logic::connective::Not,
    prelude::{Annotated, Assert, Cursor, Lex, Report, Span, Spanned},
    print::visualize::Visualize,
    report::annotated::{InlineAnnotations, label::Label},
    text::{builtin::Char, cursor::error::Expected},
};
use fack::prelude::Error;

/// Character and boundary expectations for keyword tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Assert)]
pub enum Fragment {
    /// The `i` character.
    #[assert(use Char('i') type Char)]
    LowerI,

    /// The `n` character.
    #[assert(use Char('n') type Char)]
    LowerN,

    /// The `c` character.
    #[assert(use Char('c') type Char)]
    LowerC,

    /// The `l` character.
    #[assert(use Char('l') type Char)]
    LowerL,

    /// The `u` character.
    #[assert(use Char('u') type Char)]
    LowerU,

    /// The `d` character.
    #[assert(use Char('d') type Char)]
    LowerD,

    /// The `e` character.
    #[assert(use Char('e') type Char)]
    LowerE,

    /// The `h` character.
    #[assert(use Char('h') type Char)]
    LowerH,

    /// The `r` character.
    #[assert(use Char('r') type Char)]
    LowerR,

    /// The `t` character.
    #[assert(use Char('t') type Char)]
    LowerT,

    /// The `a` character.
    #[assert(use Char('a') type Char)]
    LowerA,

    /// The `m` character.
    #[assert(use Char('m') type Char)]
    LowerM,

    /// The `g` character.
    #[assert(use Char('g') type Char)]
    LowerG,

    /// The `x` character.
    #[assert(use Char('x') type Char)]
    LowerX,

    /// The `o` character.
    #[assert(use Char('o') type Char)]
    LowerO,

    /// The `j` character.
    #[assert(use Char('j') type Char)]
    LowerJ,

    /// The `f` character.
    #[assert(use Char('f') type Char)]
    LowerF,

    /// The `b` character.
    #[assert(use Char('b') type Char)]
    LowerB,

    /// The `s` character.
    #[assert(use Char('s') type Char)]
    LowerS,

    /// The `y` character.
    #[assert(use Char('y') type Char)]
    LowerY,

    /// The `p` character.
    #[assert(use Char('p') type Char)]
    LowerP,

    /// End of a reserved identifier.
    #[assert(use Not::that(super::ident::Fragment::Rest) type Not<super::ident::Fragment>)]
    Boundary,
}

/// A keyword character or token boundary was invalid.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("expected a reserved keyword")]
#[error(source(cause))]
#[report(title = "expected a reserved keyword", annotations = labels)]
// NOTE(invariant): Related context identifies the start of the attempted keyword token.
pub struct KeywordError {
    /// Expected component and observed input.
    cause: Expected<char, Fragment>,

    /// Failure location and keyword context.
    labels: InlineAnnotations<Label<&'static str>, 1>,
}

/// Declare fixed keyword tokens and their lexical family.
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
                    type Error = KeywordError;
                    type Input<'i> = Span where 's: 'i;

                    #[inline]
                    fn pass<'i>(c0: &'i mut Cursor<'s, str>) -> Result<Span, KeywordError>
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
                                    [Label::new("this keyword starts here", start)],
                                );

                                KeywordError { cause, labels }
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
                                    [Label::new("this keyword starts here", start)],
                                );

                                KeywordError { cause, labels }
                            })?;
                        )?
                        let span = start.superset(Span::unit(b0.index() - 1));

                        b0.backport();

                        Ok(span)
                    }

                    #[inline]
                    fn build_with_context<'i>(span: Span, _: &mut ()) -> Result<Self, KeywordError>
                    where
                        's: 'i,
                    {
                        Ok(Self(span))
                    }
                }
            )+

            /// Concrete keyword alternatives.
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
            pub enum Keyword {
                $(
                    $(#[$meta])*
                    $name($name),
                )+
            }

            impl Spanned for Keyword {
                #[inline]
                fn span(&self) -> Span {
                    match self {
                        $(Self::$name(value) => value.span(),)+
                    }
                }
            }

            impl Visualize for Keyword {
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

            /// A recognized keyword awaiting token construction.
            #[derive(Debug)]
            pub enum Input {
                $(
                    $(#[$meta])*
                    $name(Span),
                )+
            }

            impl<'s> Lex<'s, str> for Keyword {
                type Context = ();
                type Error = KeywordError;
                type Input<'i> = Input where 's: 'i;

                #[inline]
                fn pass<'i>(c0: &'i mut Cursor<'s, str>) -> Result<Input, KeywordError>
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
                fn build_with_context<'i>(input: Input, _: &mut ()) -> Result<Self, KeywordError>
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
    /// The `include` occurrence.
    pub struct Include = "include" => [LowerI,LowerN,LowerC,LowerL,LowerU,LowerD,LowerE] / Boundary;

    /// The `inherit` occurrence.
    pub struct Inherit = "inherit" => [LowerI,LowerN,LowerH,LowerE,LowerR,LowerI,LowerT] / Boundary;

    /// The `admerge` occurrence.
    pub struct Admerge = "admerge" => [LowerA,LowerD,LowerM,LowerE,LowerR,LowerG,LowerE] / Boundary;

    /// The `extend` occurrence.
    pub struct Extend = "extend" => [LowerE,LowerX,LowerT,LowerE,LowerN,LowerD] / Boundary;

    /// The `domain` occurrence.
    pub struct Domain = "domain" => [LowerD,LowerO,LowerM,LowerA,LowerI,LowerN] / Boundary;

    /// The `adjoin` occurrence.
    pub struct Adjoin = "adjoin" => [LowerA,LowerD,LowerJ,LowerO,LowerI,LowerN] / Boundary;

    /// The `forbid` occurrence.
    pub struct Forbid = "forbid" => [LowerF,LowerO,LowerR,LowerB,LowerI,LowerD] / Boundary;

    /// The `const` occurrence.
    pub struct Const = "const" => [LowerC,LowerO,LowerN,LowerS,LowerT] / Boundary;

    /// The `merge` occurrence.
    pub struct Merge = "merge" => [LowerM,LowerE,LowerR,LowerG,LowerE] / Boundary;

    /// The `false` occurrence.
    pub struct False = "false" => [LowerF,LowerA,LowerL,LowerS,LowerE] / Boundary;

    /// The `type` occurrence.
    pub struct Type = "type" => [LowerT,LowerY,LowerP,LowerE] / Boundary;

    /// The `rule` occurrence.
    pub struct Rule = "rule" => [LowerR,LowerU,LowerL,LowerE] / Boundary;

    /// The `enum` occurrence.
    pub struct Enum = "enum" => [LowerE,LowerN,LowerU,LowerM] / Boundary;

    /// The `join` occurrence.
    pub struct Join = "join" => [LowerJ,LowerO,LowerI,LowerN] / Boundary;

    /// The `true` occurrence.
    pub struct True = "true" => [LowerT,LowerR,LowerU,LowerE] / Boundary;
}

impl KeywordError {
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
    fn exact_words_and_boundaries() {
        for source in ["type_name", "types", "Type", "ty"] {
            let mut c0 = Cursor::create(source);

            assert!(Type::lex(&mut c0).is_err());

            assert_eq!(c0.index(), 0);
        }

        let mut c0 = Cursor::create("include(");

        assert_eq!(Include::lex(&mut c0).expect("keyword").span().range(), 0..7);
    }
}

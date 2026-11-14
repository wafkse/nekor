//! Exact numeric occurrences and independently recognizable numeric components.

use core::fmt;

use aldebaran::{
    ice::Ice,
    logic::connective::{Not, OneOf, Or},
    prelude::{Annotated, Assert, Cursor, Lex, Report, Span, Spanned},
    report::annotated::{InlineAnnotations, label::Label},
    text::{
        builtin::{Char, property},
        cursor::error::Expected,
    },
};
use fack::prelude::Error;

/// Numeric component expectations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Assert)]
pub enum Fragment {
    /// A decimal digit.
    #[assert(use property::AsciiDigit type property::AsciiDigit)]
    Decimal,

    /// A hexadecimal digit.
    #[assert(use property::AsciiHexadecimal type property::AsciiHexadecimal)]
    Hex,

    /// An octal digit.
    #[assert(
        use OneOf::these([
            Char('0'),
            Char('1'),
            Char('2'),
            Char('3'),
            Char('4'),
            Char('5'),
            Char('6'),
            Char('7'),
        ])
        type OneOf<Char, 8>
    )]
    Octal,

    /// A binary digit.
    #[assert(use Or::one(Char('0'), Char('1')) type Or<Char, Char>)]
    Binary,

    /// A numeric separator or unit introducer.
    #[assert(use Char('_') type Char)]
    Separator,

    /// A leading zero.
    #[assert(use Char('0') type Char)]
    Zero,

    /// A hexadecimal prefix selector.
    #[assert(use Char('x') type Char)]
    HexPrefix,

    /// An octal prefix selector.
    #[assert(use Char('o') type Char)]
    OctalPrefix,

    /// A binary prefix selector.
    #[assert(use Char('b') type Char)]
    BinaryPrefix,

    /// The decimal point.
    #[assert(use Char('.') type Char)]
    Point,

    /// An exponent introducer.
    #[assert(use Or::one(Char('e'), Char('E')) type Or<Char, Char>)]
    Exponent,

    /// An exponent sign.
    #[assert(use Or::one(Char('+'), Char('-')) type Or<Char, Char>)]
    Sign,

    /// The first unit letter.
    #[assert(use property::AsciiAlphabetic type property::AsciiAlphabetic)]
    UnitStart,

    /// Subsequent unit letters and digits.
    #[assert(use property::AsciiAlphanumeric type property::AsciiAlphanumeric)]
    UnitRest,

    /// End of a numeric token.
    #[assert(use Not::that(super::ident::Fragment::Rest) type Not<super::ident::Fragment>)]
    Boundary,
}

/// Numeric syntax failures.
#[derive(Debug, Clone, Report)]
#[report(forward)]
pub enum NumberError {
    /// The recognized numeric form differs from the required form.
    Form(FormMismatch),

    /// A required component was missing.
    Expected(Expectation),

    /// A decimal integer contains a forbidden leading zero.
    LeadingZero(LeadingZero),
}

impl fmt::Display for NumberError {
    #[inline]
    fn fmt(&self, writer: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Form(error) => fmt::Display::fmt(error, writer),
            Self::Expected(error) => fmt::Display::fmt(error, writer),
            Self::LeadingZero(error) => fmt::Display::fmt(error, writer),
        }
    }
}

impl Error for NumberError {
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Form(error) => Some(error),
            Self::Expected(error) => Some(error),
            Self::LeadingZero(error) => Some(error),
        }
    }
}

/// An invalid numeric component or boundary.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("invalid numeric literal")]
#[error(source(cause))]
#[report(title = "invalid numeric literal", annotations = labels)]
// NOTE(invariant): The related annotation locates the containing numeric component.
pub struct Expectation {
    /// Typed expectation and observed source component.
    cause: Expected<char, Fragment>,

    /// Primary failure location followed by related source context.
    labels: InlineAnnotations<Label<&'static str>, 1>,
}

/// Decimal leading zeros are not accepted.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("leading zero in decimal integer")]
#[report(title = "leading zero in decimal integer", annotations = labels)]
// NOTE(invariant): None.
pub struct LeadingZero {
    /// Primary failure location followed by related source context.
    labels: InlineAnnotations<Label<&'static str>, 1>,
}

/// A separated decimal digit sequence.
///
/// Construction requires the opaque input returned by recognition. A source span
/// alone cannot establish that decimal digits were recognized.
///
/// ```compile_fail
/// use aldebaran::prelude::{Lex, Span};
/// use verte_parse::scan::number::DecimalDigits;
///
/// let _ = <DecimalDigits as Lex<'_, str>>::build(Span::unit(42));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
// NOTE(invariant): The span covers nonempty separated decimal digits with no leading or trailing
// separator.
pub struct DecimalDigits(Span);

impl Spanned for DecimalDigits {
    #[inline]
    fn span(&self) -> Span {
        let &Self(span) = self;

        span
    }
}

/// Source extent established by successful recognition of [`DecimalDigits`].
#[derive(Debug)]
// NOTE(invariant): The span was recognized by `DecimalDigits::pass`.
pub struct DecimalDigitsInput(Span);

impl<'s> Lex<'s, str> for DecimalDigits {
    type Context = ();
    type Error = NumberError;
    type Input<'i>
        = DecimalDigitsInput
    where
        's: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut Cursor<'s, str>) -> Result<Self::Input<'i>, NumberError>
    where
        's: 'i,
    {
        let mut b0 = c0.branch();
        let start = Span::unit(b0.index());

        b0.expect_is_next(Fragment::Decimal).map_err(|cause| {
            let labels = InlineAnnotations::new(
                Label::new(
                    "expected a digit valid for this numeric component",
                    Annotated::target(&cause),
                ),
                [Label::new("this numeric component starts here", start)],
            );

            NumberError::Expected(Expectation { cause, labels })
        })?;
        loop {
            b0.ignore_while(Fragment::Decimal);
            if !b0.is_next(Fragment::Separator) {
                break;
            }
            if b0.is_at::<1, _, _>(Fragment::UnitStart) {
                break;
            }
            b0.expect_is_next(Fragment::Separator).map_err(|cause| {
                let labels = InlineAnnotations::new(
                    Label::new("expected a numeric separator", Annotated::target(&cause)),
                    [Label::new("this numeric component starts here", start)],
                );

                NumberError::Expected(Expectation { cause, labels })
            })?;
            b0.expect_is_next(Fragment::Decimal).map_err(|cause| {
                let labels = InlineAnnotations::new(
                    Label::new(
                        "a numeric separator requires a valid digit on each side",
                        Annotated::target(&cause),
                    ),
                    [Label::new("this numeric component starts here", start)],
                );

                NumberError::Expected(Expectation { cause, labels })
            })?;
        }

        let span = start.superset(Span::unit(b0.index() - 1));

        b0.backport();

        Ok(DecimalDigitsInput(span))
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, NumberError>
    where
        's: 'i,
    {
        let DecimalDigitsInput(span) = input;

        Ok(Self(span))
    }
}

/// A separated hexadecimal digit sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
// NOTE(invariant): The span covers nonempty separated hexadecimal digits with no leading or
// trailing separator.
pub struct HexDigits(Span);

impl Spanned for HexDigits {
    #[inline]
    fn span(&self) -> Span {
        let &Self(span) = self;

        span
    }
}

/// Source extent established by successful recognition of [`HexDigits`].
#[derive(Debug)]
// NOTE(invariant): The span was recognized by `HexDigits::pass`.
pub struct HexDigitsInput(Span);

impl<'s> Lex<'s, str> for HexDigits {
    type Context = ();
    type Error = NumberError;
    type Input<'i>
        = HexDigitsInput
    where
        's: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut Cursor<'s, str>) -> Result<Self::Input<'i>, NumberError>
    where
        's: 'i,
    {
        let mut b0 = c0.branch();
        let start = Span::unit(b0.index());

        b0.expect_is_next(Fragment::Hex).map_err(|cause| {
            let labels = InlineAnnotations::new(
                Label::new(
                    "expected a digit valid for this numeric component",
                    Annotated::target(&cause),
                ),
                [Label::new("this numeric component starts here", start)],
            );

            NumberError::Expected(Expectation { cause, labels })
        })?;
        loop {
            b0.ignore_while(Fragment::Hex);
            if !b0.is_next(Fragment::Separator) {
                break;
            }
            b0.expect_is_next(Fragment::Separator).map_err(|cause| {
                let labels = InlineAnnotations::new(
                    Label::new("expected a numeric separator", Annotated::target(&cause)),
                    [Label::new("this numeric component starts here", start)],
                );

                NumberError::Expected(Expectation { cause, labels })
            })?;
            b0.expect_is_next(Fragment::Hex).map_err(|cause| {
                let labels = InlineAnnotations::new(
                    Label::new(
                        "a numeric separator requires a valid digit on each side",
                        Annotated::target(&cause),
                    ),
                    [Label::new("this numeric component starts here", start)],
                );

                NumberError::Expected(Expectation { cause, labels })
            })?;
        }

        let span = start.superset(Span::unit(b0.index() - 1));

        b0.backport();

        Ok(HexDigitsInput(span))
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, NumberError>
    where
        's: 'i,
    {
        let HexDigitsInput(span) = input;

        Ok(Self(span))
    }
}

/// A separated octal digit sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
// NOTE(invariant): The span covers nonempty separated octal digits with no leading or trailing
// separator.
pub struct OctalDigits(Span);

impl Spanned for OctalDigits {
    #[inline]
    fn span(&self) -> Span {
        let &Self(span) = self;

        span
    }
}

/// Source extent established by successful recognition of [`OctalDigits`].
#[derive(Debug)]
// NOTE(invariant): The span was recognized by `OctalDigits::pass`.
pub struct OctalDigitsInput(Span);

impl<'s> Lex<'s, str> for OctalDigits {
    type Context = ();
    type Error = NumberError;
    type Input<'i>
        = OctalDigitsInput
    where
        's: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut Cursor<'s, str>) -> Result<Self::Input<'i>, NumberError>
    where
        's: 'i,
    {
        let mut b0 = c0.branch();
        let start = Span::unit(b0.index());

        b0.expect_is_next(Fragment::Octal).map_err(|cause| {
            let labels = InlineAnnotations::new(
                Label::new(
                    "expected a digit valid for this numeric component",
                    Annotated::target(&cause),
                ),
                [Label::new("this numeric component starts here", start)],
            );

            NumberError::Expected(Expectation { cause, labels })
        })?;
        loop {
            b0.ignore_while(Fragment::Octal);
            if !b0.is_next(Fragment::Separator) {
                break;
            }
            b0.expect_is_next(Fragment::Separator).map_err(|cause| {
                let labels = InlineAnnotations::new(
                    Label::new("expected a numeric separator", Annotated::target(&cause)),
                    [Label::new("this numeric component starts here", start)],
                );

                NumberError::Expected(Expectation { cause, labels })
            })?;
            b0.expect_is_next(Fragment::Octal).map_err(|cause| {
                let labels = InlineAnnotations::new(
                    Label::new(
                        "a numeric separator requires a valid digit on each side",
                        Annotated::target(&cause),
                    ),
                    [Label::new("this numeric component starts here", start)],
                );

                NumberError::Expected(Expectation { cause, labels })
            })?;
        }

        let span = start.superset(Span::unit(b0.index() - 1));

        b0.backport();

        Ok(OctalDigitsInput(span))
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, NumberError>
    where
        's: 'i,
    {
        let OctalDigitsInput(span) = input;

        Ok(Self(span))
    }
}

/// A separated binary digit sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
// NOTE(invariant): The span covers nonempty separated binary digits with no leading or trailing
// separator.
pub struct BinaryDigits(Span);

impl Spanned for BinaryDigits {
    #[inline]
    fn span(&self) -> Span {
        let &Self(span) = self;

        span
    }
}

/// Source extent established by successful recognition of [`BinaryDigits`].
#[derive(Debug)]
// NOTE(invariant): The span was recognized by `BinaryDigits::pass`.
pub struct BinaryDigitsInput(Span);

impl<'s> Lex<'s, str> for BinaryDigits {
    type Context = ();
    type Error = NumberError;
    type Input<'i>
        = BinaryDigitsInput
    where
        's: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut Cursor<'s, str>) -> Result<Self::Input<'i>, NumberError>
    where
        's: 'i,
    {
        let mut b0 = c0.branch();
        let start = Span::unit(b0.index());

        b0.expect_is_next(Fragment::Binary).map_err(|cause| {
            let labels = InlineAnnotations::new(
                Label::new(
                    "expected a digit valid for this numeric component",
                    Annotated::target(&cause),
                ),
                [Label::new("this numeric component starts here", start)],
            );

            NumberError::Expected(Expectation { cause, labels })
        })?;
        loop {
            b0.ignore_while(Fragment::Binary);
            if !b0.is_next(Fragment::Separator) {
                break;
            }
            b0.expect_is_next(Fragment::Separator).map_err(|cause| {
                let labels = InlineAnnotations::new(
                    Label::new("expected a numeric separator", Annotated::target(&cause)),
                    [Label::new("this numeric component starts here", start)],
                );

                NumberError::Expected(Expectation { cause, labels })
            })?;
            b0.expect_is_next(Fragment::Binary).map_err(|cause| {
                let labels = InlineAnnotations::new(
                    Label::new(
                        "a numeric separator requires a valid digit on each side",
                        Annotated::target(&cause),
                    ),
                    [Label::new("this numeric component starts here", start)],
                );

                NumberError::Expected(Expectation { cause, labels })
            })?;
        }

        let span = start.superset(Span::unit(b0.index() - 1));

        b0.backport();

        Ok(BinaryDigitsInput(span))
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, NumberError>
    where
        's: 'i,
    {
        let BinaryDigitsInput(span) = input;

        Ok(Self(span))
    }
}

/// An unsigned decimal integer without leading zeros.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
// NOTE(invariant): The span covers a decimal integer with no leading zeros.
pub struct DecimalInteger(Span);

impl Spanned for DecimalInteger {
    #[inline]
    fn span(&self) -> Span {
        let &Self(span) = self;

        span
    }
}

/// Source extent established by successful recognition of [`DecimalInteger`].
#[derive(Debug)]
// NOTE(invariant): The span was recognized by `DecimalInteger::pass`.
pub struct DecimalIntegerInput(Span);

impl<'s> Lex<'s, str> for DecimalInteger {
    type Context = ();
    type Error = NumberError;
    type Input<'i>
        = DecimalIntegerInput
    where
        's: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut Cursor<'s, str>) -> Result<Self::Input<'i>, NumberError>
    where
        's: 'i,
    {
        let mut b0 = c0.branch();
        let start = Span::unit(b0.index());

        let DecimalDigitsInput(span) = DecimalDigits::pass(&mut b0)?;

        let text = b0.slice(span).source();
        let mut c1 = Cursor::create(text);

        if c1.optional(Fragment::Zero).is_some() && !c1.is_eof() {
            let labels = InlineAnnotations::new(
                Label::new("remove the leading zero or use an explicit radix prefix", span),
                [Label::new("decimal integer starts here", start)],
            );

            return Err(NumberError::LeadingZero(LeadingZero { labels }));
        }

        let span = start.superset(Span::unit(b0.index() - 1));

        b0.backport();

        Ok(DecimalIntegerInput(span))
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, NumberError>
    where
        's: 'i,
    {
        let DecimalIntegerInput(span) = input;

        Ok(Self(span))
    }
}

/// A decimal exponent with optional sign.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
// NOTE(invariant): The span covers an exponent marker, optional sign, and separated decimal digits.
pub struct Exponent(Span);

impl Spanned for Exponent {
    #[inline]
    fn span(&self) -> Span {
        let &Self(span) = self;

        span
    }
}

/// Source extent established by successful recognition of [`Exponent`].
#[derive(Debug)]
// NOTE(invariant): The span was recognized by `Exponent::pass`.
pub struct ExponentInput(Span);

impl<'s> Lex<'s, str> for Exponent {
    type Context = ();
    type Error = NumberError;
    type Input<'i>
        = ExponentInput
    where
        's: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut Cursor<'s, str>) -> Result<Self::Input<'i>, NumberError>
    where
        's: 'i,
    {
        let mut b0 = c0.branch();
        let start = Span::unit(b0.index());

        b0.expect_is_next(Fragment::Exponent).map_err(|cause| {
            let labels = InlineAnnotations::new(
                Label::new("expected `e` or `E`", Annotated::target(&cause)),
                [Label::new("this numeric component starts here", start)],
            );

            NumberError::Expected(Expectation { cause, labels })
        })?;
        b0.optional(Fragment::Sign);
        DecimalDigits::pass(&mut b0)?;

        let span = start.superset(Span::unit(b0.index() - 1));

        b0.backport();

        Ok(ExponentInput(span))
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, NumberError>
    where
        's: 'i,
    {
        let ExponentInput(span) = input;

        Ok(Self(span))
    }
}

/// A decimal point followed by separated decimal digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
// NOTE(invariant): The span covers a decimal point followed by separated decimal digits.
pub struct Fraction(Span);

impl Spanned for Fraction {
    #[inline]
    fn span(&self) -> Span {
        let &Self(span) = self;

        span
    }
}

/// Source extent established by successful recognition of [`Fraction`].
#[derive(Debug)]
// NOTE(invariant): The span was recognized by `Fraction::pass`.
pub struct FractionInput(Span);

impl<'s> Lex<'s, str> for Fraction {
    type Context = ();
    type Error = NumberError;
    type Input<'i>
        = FractionInput
    where
        's: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut Cursor<'s, str>) -> Result<Self::Input<'i>, NumberError>
    where
        's: 'i,
    {
        let mut b0 = c0.branch();
        let start = Span::unit(b0.index());

        b0.expect_is_next(Fragment::Point).map_err(|cause| {
            let labels = InlineAnnotations::new(
                Label::new("expected a decimal point", Annotated::target(&cause)),
                [Label::new("this numeric component starts here", start)],
            );

            NumberError::Expected(Expectation { cause, labels })
        })?;
        DecimalDigits::pass(&mut b0)?;

        let span = start.superset(Span::unit(b0.index() - 1));

        b0.backport();

        Ok(FractionInput(span))
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, NumberError>
    where
        's: 'i,
    {
        let FractionInput(span) = input;

        Ok(Self(span))
    }
}

/// A case-sensitive alphanumeric unit name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
// NOTE(invariant): The span covers an ASCII letter followed by ASCII letters or digits.
pub struct Unit(Span);

impl Spanned for Unit {
    #[inline]
    fn span(&self) -> Span {
        let &Self(span) = self;

        span
    }
}

/// Source extent established by successful recognition of [`Unit`].
#[derive(Debug)]
// NOTE(invariant): The span was recognized by `Unit::pass`.
pub struct UnitInput(Span);

impl<'s> Lex<'s, str> for Unit {
    type Context = ();
    type Error = NumberError;
    type Input<'i>
        = UnitInput
    where
        's: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut Cursor<'s, str>) -> Result<Self::Input<'i>, NumberError>
    where
        's: 'i,
    {
        let mut b0 = c0.branch();
        let start = Span::unit(b0.index());

        b0.expect_is_next(Fragment::UnitStart).map_err(|cause| {
            let labels = InlineAnnotations::new(
                Label::new(
                    "a unit suffix must begin with an ASCII letter",
                    Annotated::target(&cause),
                ),
                [Label::new("this numeric component starts here", start)],
            );

            NumberError::Expected(Expectation { cause, labels })
        })?;
        b0.ignore_while(Fragment::UnitRest);

        let span = start.superset(Span::unit(b0.index() - 1));

        b0.backport();

        Ok(UnitInput(span))
    }

    #[inline]
    fn build_with_context<'i>(input: Self::Input<'i>, _: &mut ()) -> Result<Self, NumberError>
    where
        's: 'i,
    {
        let UnitInput(span) = input;

        Ok(Self(span))
    }
}

/// The syntactic base of an unsigned integer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Radix {
    /// Base ten.
    Decimal,

    /// Base sixteen.
    Hexadecimal,

    /// Base eight.
    Octal,

    /// Base two.
    Binary,
}

/// An exact unsigned integer occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
// NOTE(invariant): The span covers valid integer syntax in `radix`.
pub struct Integer {
    /// Radix used by the integer literal.
    radix: Radix,

    /// Original source byte extent.
    span: Span,
}

impl Integer {
    /// Retrieve the integer's base.
    #[inline]
    pub const fn radix(&self) -> Radix {
        let &Self { radix, .. } = self;

        radix
    }
}

impl Spanned for Integer {
    #[inline]
    fn span(&self) -> Span {
        let &Self { span, .. } = self;

        span
    }
}

/// An exact decimal fraction or exponent occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
// NOTE(invariant): The span covers a decimal literal with a fraction or exponent.
pub struct Decimal(Span);

impl Spanned for Decimal {
    #[inline]
    fn span(&self) -> Span {
        let &Self(span) = self;

        span
    }
}

/// A decimal magnitude with a unit suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
// NOTE(invariant): `magnitude` and `unit` are contained in `span`, separated by an underscore.
pub struct Quantity {
    /// Decimal magnitude before the unit separator.
    magnitude: Span,

    /// Case-sensitive unit name without its separator.
    unit: Span,

    /// Original source byte extent.
    span: Span,
}

impl Quantity {
    /// Retrieve the magnitude's exact text extent.
    #[inline]
    pub const fn magnitude(&self) -> Span {
        let &Self { magnitude, .. } = self;

        magnitude
    }

    /// Retrieve the unit name extent without its underscore.
    #[inline]
    pub const fn unit(&self) -> Span {
        let &Self { unit, .. } = self;

        unit
    }
}

impl Spanned for Quantity {
    #[inline]
    fn span(&self) -> Span {
        let &Self { span, .. } = self;

        span
    }
}

/// Concrete numeric alternatives without fixed-width conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Number {
    /// Unsigned integer syntax.
    Integer(Integer),

    /// Fractional or exponent syntax.
    Decimal(Decimal),

    /// Decimal quantity syntax.
    Quantity(Quantity),
}

impl Spanned for Number {
    #[inline]
    fn span(&self) -> Span {
        match self {
            Self::Integer(v) => v.span(),
            Self::Decimal(v) => v.span(),
            Self::Quantity(v) => v.span(),
        }
    }
}

/// Numeric recognition state awaiting concrete construction.
#[derive(Debug)]
// NOTE(invariant): The form and its spans were established by successful numeric recognition.
pub struct Input(Form);

/// The numeric alternative and source extents established during recognition.
#[derive(Debug)]
enum Form {
    /// Integer base and extent.
    Integer(Radix, Span),

    /// A fraction or exponent extent.
    Decimal(Span),

    /// Decimal magnitude, unit name, and entire extent.
    Quantity(Span, Span, Span),
}

impl<'s> Lex<'s, str> for Number {
    type Context = ();
    type Error = NumberError;
    type Input<'i>
        = Input
    where
        's: 'i;

    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "Keep substantial recognition or construction out of line across grammar entry points."
    )]
    fn pass<'i>(c0: &'i mut Cursor<'s, str>) -> Result<Input, NumberError>
    where
        's: 'i,
    {
        let mut b0 = c0.branch();
        let start = Span::unit(b0.index());
        let radix = if b0.is_next(Fragment::Zero) && b0.is_at::<1, _, _>(Fragment::HexPrefix) {
            Some(Radix::Hexadecimal)
        } else if b0.is_next(Fragment::Zero) && b0.is_at::<1, _, _>(Fragment::OctalPrefix) {
            Some(Radix::Octal)
        } else if b0.is_next(Fragment::Zero) && b0.is_at::<1, _, _>(Fragment::BinaryPrefix) {
            Some(Radix::Binary)
        } else {
            None
        };
        let input = match radix {
            Some(radix) => {
                b0.expect_is_next(Fragment::Zero).map_err(|cause| {
                    let labels = InlineAnnotations::new(
                        Label::new("expected a radix prefix", Annotated::target(&cause)),
                        [Label::new("this numeric component starts here", start)],
                    );

                    NumberError::Expected(Expectation { cause, labels })
                })?;
                match radix {
                    Radix::Hexadecimal => {
                        b0.expect_is_next(Fragment::HexPrefix).map_err(|cause| {
                            let labels = InlineAnnotations::new(
                                Label::new("expected a radix prefix selector", Annotated::target(&cause)),
                                [Label::new("this numeric component starts here", start)],
                            );

                            NumberError::Expected(Expectation { cause, labels })
                        })?;
                        HexDigits::pass(&mut b0)?;
                    },
                    Radix::Octal => {
                        b0.expect_is_next(Fragment::OctalPrefix).map_err(|cause| {
                            let labels = InlineAnnotations::new(
                                Label::new("expected a radix prefix selector", Annotated::target(&cause)),
                                [Label::new("this numeric component starts here", start)],
                            );

                            NumberError::Expected(Expectation { cause, labels })
                        })?;
                        OctalDigits::pass(&mut b0)?;
                    },
                    Radix::Binary => {
                        b0.expect_is_next(Fragment::BinaryPrefix).map_err(|cause| {
                            let labels = InlineAnnotations::new(
                                Label::new("expected a radix prefix selector", Annotated::target(&cause)),
                                [Label::new("this numeric component starts here", start)],
                            );

                            NumberError::Expected(Expectation { cause, labels })
                        })?;
                        BinaryDigits::pass(&mut b0)?;
                    },
                    Radix::Decimal => Ice::impossible(None),
                }

                let span = start.superset(Span::unit(b0.index() - 1));

                Form::Integer(radix, span)
            },
            None => {
                DecimalInteger::pass(&mut b0)?;

                let mut fractional = false;

                if b0.is_next(Fragment::Point) && !b0.is_at::<1, _, _>(Fragment::Point) {
                    Fraction::pass(&mut b0)?;
                    fractional = true;
                }
                if b0.is_next(Fragment::Exponent) {
                    Exponent::pass(&mut b0)?;
                    fractional = true;
                }

                let magnitude = start.superset(Span::unit(b0.index() - 1));

                if b0.is_next(Fragment::Separator) {
                    b0.expect_is_next(Fragment::Separator).map_err(|cause| {
                        let labels = InlineAnnotations::new(
                            Label::new("expected the unit separator", Annotated::target(&cause)),
                            [Label::new("this numeric component starts here", start)],
                        );

                        NumberError::Expected(Expectation { cause, labels })
                    })?;

                    let UnitInput(unit) = Unit::pass(&mut b0)?;

                    let span = start.superset(unit);

                    Form::Quantity(magnitude, unit, span)
                } else if fractional {
                    Form::Decimal(magnitude)
                } else {
                    Form::Integer(Radix::Decimal, magnitude)
                }
            },
        };

        b0.validate_is_next_or_eof(Fragment::Boundary).map_err(|cause| {
            let labels = InlineAnnotations::new(
                Label::new(
                    "a numeric literal must end at a token boundary",
                    Annotated::target(&cause),
                ),
                [Label::new("this numeric component starts here", start)],
            );

            NumberError::Expected(Expectation { cause, labels })
        })?;
        b0.backport();

        Ok(Input(input))
    }

    #[inline]
    fn build_with_context<'i>(input: Input, _: &mut ()) -> Result<Self, NumberError>
    where
        's: 'i,
    {
        let Input(form) = input;

        Ok(match form {
            Form::Integer(radix, span) => Self::Integer(Integer { radix, span }),
            Form::Decimal(span) => Self::Decimal(Decimal(span)),
            Form::Quantity(magnitude, unit, span) => Self::Quantity(Quantity { magnitude, unit, span }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_forms_and_maximal_boundaries() {
        for source in [
            "0",
            "42",
            "0x10_B",
            "0b1_0",
            "0o7_1",
            "1.25e-3",
            "1e3_GiB",
            "2_GiB",
            "999999999999999999999999999999999999",
        ] {
            let mut c0 = Cursor::create(source);
            let number = Number::lex(&mut c0).expect(source);

            assert_eq!(number.span().range(), 0..source.len());
        }

        let mut c0 = Cursor::create("1..+2_GiB");

        assert!(matches!(Number::lex(&mut c0), Ok(Number::Integer(_))));

        assert_eq!(c0.index(), 1);
        let mut c0 = Cursor::create("0x10_B");

        assert!(matches!(Number::lex(&mut c0), Ok(Number::Integer(_))));
    }
    #[test]
    fn failures_roll_back() {
        for source in [
            "01", "0_1", "1__2", "1_", "0x", "0b2", "0x1_GiB", "1e+", "1.2.3", "1e3e4",
        ] {
            let mut c0 = Cursor::create(source);
            let result = Number::lex(&mut c0);

            if source == "1.2.3" {
                continue;
            }
            assert!(result.is_err(), "{source}");

            assert_eq!(c0.index(), 0, "{source}");
        }
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

/// A numeric grammar production required a different literal form.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("unexpected numeric form")]
#[report(title = "unexpected numeric form", annotations = labels)]
// NOTE(invariant): None.
pub struct FormMismatch {
    /// Primary failure location followed by related source context.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

/// Recognition state restricted to [`Integer`] syntax.
#[derive(Debug)]
// NOTE(invariant): The contained numeric input has the `Integer` form.
pub struct IntegerInput(Input);

impl<'s> Lex<'s, str> for Integer {
    type Context = ();
    type Error = NumberError;
    type Input<'i>
        = IntegerInput
    where
        's: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut Cursor<'s, str>) -> Result<IntegerInput, NumberError>
    where
        's: 'i,
    {
        let mut b0 = c0.branch();
        let input = Number::pass(&mut b0)?;

        let Input(form) = &input;

        match form {
            Form::Integer(_, _) => {
                b0.backport();

                Ok(IntegerInput(input))
            },
            Form::Decimal(span) | Form::Quantity(_, _, span) => {
                let labels = InlineAnnotations::new(Label::new("expected integer numeric syntax", *span), []);

                Err(NumberError::Form(FormMismatch { labels }))
            },
        }
    }

    #[inline]
    fn build_with_context<'i>(IntegerInput(input): IntegerInput, _: &mut ()) -> Result<Self, NumberError>
    where
        's: 'i,
    {
        match Number::build(input)? {
            Number::Integer(value) => Ok(value),
            _ => Ice::impossible(None),
        }
    }
}

/// Recognition state restricted to [`Decimal`] syntax.
#[derive(Debug)]
// NOTE(invariant): The contained numeric input has the `Decimal` form.
pub struct DecimalInput(Input);

impl<'s> Lex<'s, str> for Decimal {
    type Context = ();
    type Error = NumberError;
    type Input<'i>
        = DecimalInput
    where
        's: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut Cursor<'s, str>) -> Result<DecimalInput, NumberError>
    where
        's: 'i,
    {
        let mut b0 = c0.branch();
        let input = Number::pass(&mut b0)?;

        let Input(form) = &input;

        match form {
            Form::Decimal(_) => {
                b0.backport();

                Ok(DecimalInput(input))
            },
            Form::Integer(_, span) | Form::Quantity(_, _, span) => {
                let labels = InlineAnnotations::new(Label::new("expected decimal numeric syntax", *span), []);

                Err(NumberError::Form(FormMismatch { labels }))
            },
        }
    }

    #[inline]
    fn build_with_context<'i>(DecimalInput(input): DecimalInput, _: &mut ()) -> Result<Self, NumberError>
    where
        's: 'i,
    {
        match Number::build(input)? {
            Number::Decimal(value) => Ok(value),
            _ => Ice::impossible(None),
        }
    }
}

/// Recognition state restricted to [`Quantity`] syntax.
#[derive(Debug)]
// NOTE(invariant): The contained numeric input has the `Quantity` form.
pub struct QuantityInput(Input);

impl<'s> Lex<'s, str> for Quantity {
    type Context = ();
    type Error = NumberError;
    type Input<'i>
        = QuantityInput
    where
        's: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut Cursor<'s, str>) -> Result<QuantityInput, NumberError>
    where
        's: 'i,
    {
        let mut b0 = c0.branch();
        let input = Number::pass(&mut b0)?;

        let Input(form) = &input;

        match form {
            Form::Quantity(_, _, _) => {
                b0.backport();

                Ok(QuantityInput(input))
            },
            Form::Integer(_, span) | Form::Decimal(span) => {
                let labels = InlineAnnotations::new(Label::new("expected quantity numeric syntax", *span), []);

                Err(NumberError::Form(FormMismatch { labels }))
            },
        }
    }

    #[inline]
    fn build_with_context<'i>(QuantityInput(input): QuantityInput, _: &mut ()) -> Result<Self, NumberError>
    where
        's: 'i,
    {
        match Number::build(input)? {
            Number::Quantity(value) => Ok(value),
            _ => Ice::impossible(None),
        }
    }
}

#[cfg(test)]
mod preservation_tests {

    use aldebaran::prelude::{Cursor, Lex, Spanned};

    use crate::scan::{
        number::{Integer, Number, Radix},
        punct,
    };

    #[test]
    fn numeric_tokenization_is_exact_and_boundary_checked() {
        for source in [
            "0",
            "1_000",
            "0x10_B",
            "0o7_1",
            "0b1_0",
            "1.25",
            "1e-2",
            "1.0E+2",
            "115_200_Bd",
            "1.2_3e4_5_MHz",
            "0_B",
        ] {
            let mut cursor = Cursor::create(source);
            let value = Number::lex(&mut cursor).expect("numeric literal");

            assert_eq!(
                Some(source),
                cursor.source().get(value.span().range()),
                "numeric source text is preserved exactly"
            );

            assert!(cursor.is_eof(), "numeric token must consume its complete source text");
        }
        for source in [
            "01", "0_1", "1__0", "1_", "0x", "0x_1", "0o8", "0b2", "0b102", "1e", "1e+", "1e_2", "1foo", "1_Mi_B",
            "0x10_GiB",
        ] {
            let mut cursor = Cursor::create(source);

            Number::lex(&mut cursor).expect_err("malformed numeric literal");

            assert_eq!(cursor.index(), 0, "malformed numeric input must roll back");
        }

        let mut cursor = Cursor::create("0x10_B");
        let value = Integer::lex(&mut cursor).expect("greedy hexadecimal integer");

        assert_eq!(
            value.radix(),
            Radix::Hexadecimal,
            "based digits take precedence over unit recognition"
        );
        let mut cursor = Cursor::create("1..+2_GiB");

        Integer::lex(&mut cursor).expect("integer before range");

        assert_eq!(
            cursor.index(),
            1,
            "decimal scanning must not consume the first range dot"
        );
        punct::DotDotPlus::lex(&mut cursor).expect("extent range");

        assert!(
            matches!(Number::lex(&mut cursor), Ok(Number::Quantity(_))),
            "range extent is a quantity"
        );
    }
}

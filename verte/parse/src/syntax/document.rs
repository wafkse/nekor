//! Source-ordered documents, physical statement boundaries, and section placement.

use alloc::{boxed::Box, vec::Vec};
use core::fmt;

use aldebaran::{
    ice::Ice,
    prelude::{Annotated, Assert, Cursor, Lex, Report, Span, Spanned},
    report::annotated::{InlineAnnotations, label::Label},
    source::source::Source as _,
    text::cursor::error::Expected,
};
use fack::prelude::Error;

use super::declaration::{self, Field, Key, KeyError, Statement};
use crate::{
    scan::{
        context::{ScanContext, Shared},
        keyword, punct,
        tree::{self, Stream, SyntaxCursor, TokenTree, TriviaFragment, source::Tokens},
        trivia,
    },
    source::Source,
};

node! {
    /// A nonempty absolute table path.
    pub struct Path {
        /// The first key component.
        first:Key,

        /// Subsequent dotted components.
        rest:Vec<PathTail>,
    }

    /// A path separator and its following key.
    pub struct PathTail {
        /// Concrete path punctuation.
        dot:punct::Dot,

        /// The following path component.
        key:Key,
    }

    /// A bracketed absolute table path.
    pub struct Header {
        /// The opening table bracket.
        open:punct::LeftBracket,

        /// The absolute path syntax.
        path:Path,

        /// The matching closing bracket.
        close:punct::RightBracket,
    }

    /// A table header followed by source-ordered fields.
    pub struct Section {
        /// The table header and its physical boundary.
        header:Line<Header>,

        /// Fields and their physical boundaries.
        fields:Vec<Line<Field>>,
    }
}

/// A statement followed by its physical terminator, if present.
#[derive(Debug, Clone, PartialEq, Eq)]
// NOTE(invariant): `ending`, when present, immediately follows `value` apart from horizontal
// trivia.
pub struct Line<T> {
    /// The terminated production.
    value: T,

    /// A physical line ending, absent only at EOF.
    ending: Option<trivia::line_break::LineBreak>,
}

impl<T> Line<T> {
    /// Retrieve the production before its terminator.
    #[inline]
    pub const fn value(&self) -> &T {
        let Self { value, .. } = self;

        value
    }

    /// Retrieve the physical ending without synthesizing one at EOF.
    #[inline]
    pub const fn ending(&self) -> Option<trivia::line_break::LineBreak> {
        let &Self { ending, .. } = self;

        ending
    }
}

impl<T: Spanned> Spanned for Line<T> {
    #[inline]
    fn span(&self) -> Span {
        let Self { value, ending } = self;

        ending
            .as_ref()
            .map_or(value.span(), |ending| value.span().superset(ending.span()))
    }
}

/// Invalid statement syntax or physical termination.
#[derive(Debug, Clone, Report)]
#[report(forward)]
pub enum LineError<E> {
    /// A physical statement boundary was missing.
    Boundary(Boundary),

    /// The terminated production failed.
    Content(Box<E>),

    /// Source lexing failed.
    Source(tree::TokenError),
}

impl<E> fmt::Display for LineError<E>
where
    E: fmt::Display,
{
    #[inline]
    fn fmt(&self, writer: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Boundary(error) => fmt::Display::fmt(error, writer),
            Self::Content(error) => fmt::Display::fmt(error, writer),
            Self::Source(error) => fmt::Display::fmt(error, writer),
        }
    }
}

impl<E> Error for LineError<E>
where
    E: Error + 'static,
{
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Boundary(error) => Some(error),
            Self::Content(error) => Some(error.as_ref()),
            Self::Source(error) => Some(error),
        }
    }
}

/// A statement was followed by more syntax on the same physical line.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("expected a physical statement boundary")]
#[report(title = "expected a physical statement boundary", annotations = labels)]
// NOTE(invariant): The related label identifies the statement preceding the unexpected input.
pub struct Boundary {
    /// Primary failure location followed by related source context.
    labels: InlineAnnotations<Label<&'static str>, 1>,
}

impl<'s, 't, T> Lex<'t, &'t Tokens<'s>> for Line<T>
where
    T: Lex<'t, &'t Tokens<'s>, Context = ()> + Spanned,
{
    type Context = ();
    type Error = LineError<T::Error>;
    type Input<'i>
        = (T, Option<trivia::line_break::LineBreak>)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, Self::Error>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();
        let value = T::lex(&mut b0).map_err(|error| LineError::Content(Box::new(error)))?;
        let ending = match {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.peek())
        }
        .map_err(LineError::Source)?
        {
            None => None,
            Some(token) if TriviaFragment::Newline.assert(token) => {
                let Some(TokenTree::Trivia(trivia::Trivia::LineBreak(ending))) = {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source().checked(b0.any())
                }
                .map_err(LineError::Source)?
                else {
                    Ice::impossible(None)
                };

                Some(ending)
            },
            Some(token) => {
                let labels = InlineAnnotations::new(
                    Label::new(
                        "expected LF, CRLF, or EOF; put the next statement on a new line",
                        token.span(),
                    ),
                    [Label::new("this statement is already complete", value.span())],
                );

                return Err(LineError::Boundary(Boundary { labels }));
            },
        };

        b0.backport();

        Ok((value, ending))
    }

    #[inline]
    fn build_with_context<'i>((value, ending): Self::Input<'i>, _: &mut ()) -> Result<Self, Self::Error>
    where
        't: 'i,
    {
        Ok(Self { value, ending })
    }
}

/// Expectations governing document order and paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Assert)]
#[assert(input(TokenTree))]
pub enum Fragment {
    /// An absolute path separator.
    #[assert(matches(TokenTree::Punctuation(punct::Punctuation::Dot(_))) as "`.`")]
    Dot,

    /// A table header's bracketed group.
    #[assert(matches(TokenTree::Group(_)) as "a bracketed table header")]
    Header,

    /// A declaration reserved for the document prelude.
    #[assert(
        matches(
            TokenTree::Keyword(
                keyword::Keyword::Include(_)
                | keyword::Keyword::Extend(_)
                | keyword::Keyword::Inherit(_)
                | keyword::Keyword::Domain(_)
                | keyword::Keyword::Type(_)
                | keyword::Keyword::Const(_)
                | keyword::Keyword::Rule(_),
            )
        ) as "a prelude declaration"
    )]
    Prelude,
}

/// Failure in document ordering, table paths, or statements.
#[derive(Debug, Clone, Report)]
#[report(forward)]
pub enum DocumentError {
    /// A path separator was missing.
    Path(PathExpectation),

    /// A table-path component failed.
    Key(KeyError),

    /// Source lexing failed.
    Source(tree::TokenError),

    /// Table path grouping failed.
    Header(super::group::GroupError<DocumentError>),

    /// A root statement failed.
    Statement(LineError<declaration::DeclarationError>),

    /// A section header or field failed.
    Section(LineError<DocumentError>),

    /// A section field failed.
    Field(LineError<declaration::DeclarationError>),

    /// A prelude-only declaration appeared in a section.
    Placement(Placement),
}

impl fmt::Display for DocumentError {
    #[inline]
    fn fmt(&self, writer: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Path(error) => fmt::Display::fmt(error, writer),
            Self::Key(error) => fmt::Display::fmt(error, writer),
            Self::Source(error) => fmt::Display::fmt(error, writer),
            Self::Header(error) => fmt::Display::fmt(error, writer),
            Self::Statement(error) => fmt::Display::fmt(error, writer),
            Self::Section(error) => fmt::Display::fmt(error, writer),
            Self::Field(error) => fmt::Display::fmt(error, writer),
            Self::Placement(error) => fmt::Display::fmt(error, writer),
        }
    }
}

impl Error for DocumentError {
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Path(error) => Some(error),
            Self::Key(error) => Some(error),
            Self::Source(error) => Some(error),
            Self::Header(error) => Some(error),
            Self::Statement(error) => Some(error),
            Self::Section(error) => Some(error),
            Self::Field(error) => Some(error),
            Self::Placement(error) => Some(error),
        }
    }
}

/// A path separator was required here.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("expected a path separator")]
#[error(source(cause))]
#[report(title = "expected a path separator", annotations = labels)]
// NOTE(invariant): None.
pub struct PathExpectation {
    /// Grammar expectation and the token or boundary found at failure.
    cause: Expected<TokenTree, Fragment>,

    /// Primary failure location followed by related source context.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

impl PathExpectation {
    /// Retrieve the syntax fragment expected at the failure location.
    #[inline]
    pub const fn cause(&self) -> &Expected<TokenTree, Fragment> {
        let Self { cause, .. } = self;

        cause
    }
}

/// A prelude declaration appeared after the first table header.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("declaration is not allowed inside a section")]
#[report(title = "declaration is not allowed inside a section", annotations = labels)]
// NOTE(invariant): The related annotation locates the current section header.
pub struct Placement {
    /// Primary failure location followed by related source context.
    labels: InlineAnnotations<Label<&'static str>, 1>,
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for PathTail {
    type Context = ();
    type Error = DocumentError;
    type Input<'i>
        = (punct::Dot, Key)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, DocumentError>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();
        let token = {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.expect_is_next(Fragment::Dot))
        }
        .map_err(DocumentError::Source)?
        .map_err(|cause| {
            let labels = InlineAnnotations::new(
                Label::new("expected `.` between path components", Annotated::target(&cause)),
                [],
            );

            DocumentError::Path(PathExpectation { cause, labels })
        })?;

        let TokenTree::Punctuation(punct::Punctuation::Dot(dot)) = token else {
            Ice::impossible(None)
        };

        let key = Key::lex(&mut b0).map_err(DocumentError::Key)?;

        b0.backport();

        Ok((dot, key))
    }

    #[inline]
    fn build_with_context<'i>((dot, key): Self::Input<'i>, _: &mut ()) -> Result<Self, DocumentError>
    where
        't: 'i,
    {
        let span = dot.span().superset(key.span());

        Ok(Self { dot, key, span })
    }
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Path {
    type Context = ();
    type Error = DocumentError;
    type Input<'i>
        = (Key, Vec<PathTail>)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, DocumentError>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();
        let first = Key::lex(&mut b0).map_err(DocumentError::Key)?;
        let mut rest = Vec::new();

        while {
            let trivia = b0.source().trivia();

            b0.ignore_while(trivia);

            b0.source().checked(b0.is_next(Fragment::Dot))
        }
        .map_err(DocumentError::Source)?
        {
            rest.push(PathTail::lex(&mut b0)?);
        }

        b0.backport();

        Ok((first, rest))
    }

    #[inline]
    fn build_with_context<'i>((first, rest): Self::Input<'i>, _: &mut ()) -> Result<Self, DocumentError>
    where
        't: 'i,
    {
        let span = rest
            .last()
            .map_or(first.span(), |last| first.span().superset(last.span()));

        Ok(Self { first, rest, span })
    }
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Header {
    type Context = ();
    type Error = DocumentError;
    type Input<'i>
        = (punct::LeftBracket, Path, punct::RightBracket)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, DocumentError>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();
        let group = super::Bracketed::<Path>::lex(&mut b0).map_err(DocumentError::Header)?;
        let input = group.parts();

        b0.backport();

        Ok(input)
    }

    #[inline]
    fn build_with_context<'i>((open, path, close): Self::Input<'i>, _: &mut ()) -> Result<Self, DocumentError>
    where
        't: 'i,
    {
        let span = open.span().superset(close.span());

        Ok(Self {
            open,
            path,
            close,
            span,
        })
    }
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Section {
    type Context = ();
    type Error = DocumentError;
    type Input<'i>
        = (Line<Header>, Vec<Line<Field>>)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, DocumentError>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();
        let header = Line::<Header>::lex(&mut b0).map_err(DocumentError::Section)?;
        let mut fields = Vec::new();

        loop {
            while {
                let trivia = b0.source().trivia();

                b0.ignore_while(trivia);

                b0.source().checked(b0.is_next(TriviaFragment::Newline))
            }
            .map_err(DocumentError::Source)?
            {
                {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source().checked(b0.any())
                }
                .map_err(DocumentError::Source)?;
            }
            match {
                let trivia = b0.source().trivia();

                b0.ignore_while(trivia);

                b0.source().checked(b0.peek())
            }
            .map_err(DocumentError::Source)?
            {
                None => break,
                Some(token) if Fragment::Header.assert(token) => break,
                Some(token) if Fragment::Prelude.assert(token) => {
                    let labels = InlineAnnotations::new(
                        Label::new("move this declaration before the first table header", token.span()),
                        [Label::new("this section starts here", header.value().span())],
                    );

                    return Err(DocumentError::Placement(Placement { labels }));
                },
                Some(_) => fields.push(Line::<Field>::lex(&mut b0).map_err(DocumentError::Field)?),
            }
        }

        b0.backport();

        Ok((header, fields))
    }

    #[inline]
    fn build_with_context<'i>((header, fields): Self::Input<'i>, _: &mut ()) -> Result<Self, DocumentError>
    where
        't: 'i,
    {
        let span = fields
            .last()
            .map_or(header.span(), |last| header.span().superset(last.span()));

        Ok(Self { header, fields, span })
    }
}

/// Document syntax before association with a host source identity.
#[derive(Debug)]
// NOTE(invariant): Prelude statements precede all sections in source order.
struct Body {
    /// Statements before the first table header.
    prelude: Vec<Line<Statement>>,

    /// Tables in source order.
    sections: Vec<Section>,
}

impl<'s, 't> Lex<'t, &'t Tokens<'s>> for Body {
    type Context = ();
    type Error = DocumentError;
    type Input<'i>
        = (Vec<Line<Statement>>, Vec<Section>)
    where
        't: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut SyntaxCursor<'s, 't>) -> Result<Self::Input<'i>, DocumentError>
    where
        't: 'i,
    {
        let mut b0 = c0.branch();
        let mut prelude = Vec::new();
        let mut sections = Vec::new();

        loop {
            while {
                let trivia = b0.source().trivia();

                b0.ignore_while(trivia);

                b0.source().checked(b0.is_next(TriviaFragment::Newline))
            }
            .map_err(DocumentError::Source)?
            {
                {
                    let trivia = b0.source().trivia();

                    b0.ignore_while(trivia);

                    b0.source().checked(b0.any())
                }
                .map_err(DocumentError::Source)?;
            }
            match {
                let trivia = b0.source().trivia();

                b0.ignore_while(trivia);

                b0.source().checked(b0.peek())
            }
            .map_err(DocumentError::Source)?
            {
                None => break,
                Some(token) if Fragment::Header.assert(token) => sections.push(Section::lex(&mut b0)?),
                Some(_) => prelude.push(Line::<Statement>::lex(&mut b0).map_err(DocumentError::Statement)?),
            }
        }

        b0.backport();

        Ok((prelude, sections))
    }

    #[inline]
    fn build_with_context<'i>((prelude, sections): Self::Input<'i>, _: &mut ()) -> Result<Self, DocumentError>
    where
        't: 'i,
    {
        Ok(Self { prelude, sections })
    }
}

/// Concrete document syntax associated with its original text and canonical identity.
#[derive(Debug)]
// NOTE(invariant): Syntax spans and interned identities belong to `source` and `context`
// respectively.
pub struct Document<'s, Id> {
    /// Original source text in absolute byte coordinates.
    source: Source<'s, Id>,

    /// Storage domain owning the document’s interned content.
    context: ScanContext<'s>,

    /// Source-ordered document syntax.
    body: Body,
}

impl<'s, Id> Document<'s, Id> {
    /// Retrieve the canonically identified source text.
    #[inline]
    pub const fn source(&self) -> &Source<'s, Id> {
        let Self { source, .. } = self;

        source
    }

    /// Retrieve the storage domain used by this document's lexical occurrences.
    #[inline]
    pub const fn context(&self) -> &ScanContext<'s> {
        let Self { context, .. } = self;

        context
    }

    /// Retrieve statements before the first section.
    #[inline]
    pub fn prelude(&self) -> &[Line<Statement>] {
        let Self {
            body: Body { prelude, .. },
            ..
        } = self;

        prelude
    }

    /// Retrieve sections in their original order.
    #[inline]
    pub fn sections(&self) -> &[Section] {
        let Self {
            body: Body { sections, .. },
            ..
        } = self;

        sections
    }

    /// Retrieve the source extent, absent for empty input.
    #[inline]
    pub fn extent(&self) -> Option<Span> {
        self.source().text().footprint()
    }
}

/// Recognized source and syntax awaiting canonical identity association.
#[derive(Debug)]
// NOTE(invariant): `body` and `context` were constructed from `source`.
pub struct Input<'s> {
    /// Original source text in absolute byte coordinates.
    source: &'s str,

    /// Storage domain owning the document’s interned content.
    context: ScanContext<'s>,

    /// Source-ordered document syntax.
    body: Body,
}

impl<'s, Id: Clone> Lex<'s, str> for Document<'s, Id> {
    type Context = Id;
    type Error = DocumentError;
    type Input<'i>
        = Input<'s>
    where
        's: 'i;

    #[inline]
    fn pass<'i>(c0: &'i mut Cursor<'s, str>) -> Result<Input<'s>, DocumentError>
    where
        's: 'i,
    {
        let mut b0 = c0.branch();
        let source = b0.source();
        let context = Shared::new();
        let stream = Stream::new(*b0, context.clone(), None);

        let tokens = Tokens::new(stream);
        let tokens_ref = &tokens;
        let mut c1 = Cursor::at(&tokens_ref, tokens.stream().position());
        let body = Body::lex(&mut c1)?;

        drop(tokens);

        // NOTE: Syntax stores occurrences, so dropping the stream releases the last shared domain
        // borrow.
        let context = match context.into_inner() {
            Ok(context) => context,
            Err(_shared) => Ice::impossible(None),
        };

        *b0 = Cursor::at(source, source.len());
        b0.backport();

        Ok(Input { source, context, body })
    }

    #[inline]
    fn build_with_context<'i>(
        Input { source, context, body }: Input<'s>,
        identity: &mut Id,
    ) -> Result<Self, DocumentError>
    where
        's: 'i,
    {
        let source = Source::new(identity.clone(), source);

        Ok(Self { source, context, body })
    }
}

#[cfg(test)]
mod tests {
    use alloc::{format, string::String};

    use aldebaran::{
        grammar::stream::TokenStream as _,
        prelude::Cursor,
        report::{
            annotated::Annotations as _,
            render::{
                RenderMut as _,
                textual::{
                    Present, Textual,
                    offload::fancy::{Fancy, FancySettings},
                },
            },
        },
    };

    use super::*;

    extern crate std;

    #[test]
    fn representative_document_preserves_source_and_order() {
        let source = r#"# root definitions
include("common.units")
include("common.types")
const page: size = 4_KiB
const stack_pages: u16 = 32
rule page_bound = 4_KiB..=1_GiB & ^2
rule aligned = % page
rule region_aligned = .start @ aligned & .size @ aligned
domain physical: addr = { width = 52 }
domain virtual: addr = {
    width = 64,
    valid = 0..+128_TiB | 0xffff_8000_0000_0000..+128_TiB
}
domain clocks: ref = { members = ["uart", "pll"] }
domain gic: irq = { valid = 0..1020 }
type phys = addr[physical]
type virt = addr[virtual]
type page_size = size @ page_bound
type memory_region = range[phys] @ region_aligned
type serial_interrupt = {
    line: irq[gic],
    trigger: enum["level", "edge"],
    polarity: enum["high", "low"],
}
inherit(merge)
[arch]
page_size: page_size = page
[stack]
size: size = stack_pages * page
[layout]
physical: memory_region = 0x1000_000..+2_GiB
kernel: range[virt] = 0xffff_8000_0000_0000..+1_GiB
[serial]
frequency: frequency = 48_MHz
clock: ref[clocks] = "uart"
interrupt: serial_interrupt = {
    line = 34,
    trigger = "level",
    polarity = "high",
}
"#;

        let mut c0 = Cursor::create(source);
        let document = Document::lex_with_context(&mut c0, &mut "fixture").expect("document");

        assert_eq!(document.prelude().len(), 17);

        assert_eq!(document.sections().len(), 4);

        assert_eq!(document.source().text(), source);

        assert_eq!(document.source().identity(), &"fixture");

        assert!(c0.is_eof());
    }
    #[test]
    fn every_declaration_and_type_production_is_accepted() {
        for source in [
            "include(\n\"platform.\" + arch\n)",
            "extend(\"base\")",
            "inherit(forbid)",
            "inherit(merge)\ninherit(join)\nextend(\"a\")\nextend(\"b\")",
            "domain PageSize: count = {}",
            "domain p: unknown_category = { unknown = 1 }",
            "type PageSize = (addr[physical]) @ (>= 2 & ^2)",
            "type r = {}",
            "type r = {x: [range[addr[physical]]; N], \"type\": enum[\"a\",\"b\"],}",
            "type r = ctor[a, b @ rule_name]",
            "const x: [u128; 0x10] = [1, 2,]",
            "const x: u128 = 340282366920938463463374607431768211456",
            "rule r = % (page + 1_B) | >= (page * 2)",
            "rule r = .outer.inner[0] @ aligned",
            "rule r = ^3",
            "x: u8 = unknown_forward_reference",
            "\"type\": string = \"x\"",
            "merge(x: u8 = 1)",
            "join(x: u8 = 1)",
            "admerge(x: [u8] = [1])",
            "adjoin(x: u8 = 1)",
            "[\"quoted.key\".Nested]\n\"merge\": bool = true\n[empty]",
            "type x = enum[\"same\", \"same\"]",
            "x: u8 = 1\nx: u8 = 2",
            "[same]\n[same]",
        ] {
            let mut c0 = Cursor::create(source);
            let document = Document::lex_with_context(&mut c0, &mut "fixture").expect(source);

            assert_eq!(document.source().text(), source);

            assert!(c0.is_eof());
        }
    }
    #[test]
    fn source_backed_trivia_and_physical_boundaries() {
        for source in [
            "",
            "#",
            "# note",
            "\n\n",
            "  \t\r\n  # note\n",
            "\u{2003}# λ\r\n",
            "x: string = \"#λ\\n\" # eof",
            "const x: u8 = (1 # inside\r\n + 2) # outside\r\n",
        ] {
            let mut c0 = Cursor::create(source);
            let document = Document::lex_with_context(&mut c0, &mut "fixture").expect(source);
            let mut stream = Stream::new(Cursor::create(document.source().text()), Shared::new(), None);
            let mut recovered = String::new();
            let mut boundary = 0;

            while let Some(token) = stream.next().expect("lexical stream") {
                assert_eq!(token.span().start(), boundary);
                boundary = token.span().end();
                recovered.push_str(source.get(token.span().range()).expect("source span"));
            }
            assert_eq!(boundary, source.len());

            assert_eq!(recovered, source);
        }

        let mut c0 = Cursor::create("x: u8 = 1");
        let document = Document::lex_with_context(&mut c0, &mut "fixture").expect("document");

        assert!(document.prelude()[0].ending().is_none());
        let mut c0 = Cursor::create("x: u8 = 1\r\n");
        let document = Document::lex_with_context(&mut c0, &mut "fixture").expect("document");

        assert_eq!(
            document.prelude()[0].ending().expect("physical ending").span().range(),
            9..11
        );
    }
    #[test]
    fn invalid_productions_fail_without_consuming_source() {
        for source in [
            "inherit(\"merge\")",
            "const type: u8 = 1",
            "type X = enum[]",
            "type X = enum[\"x\",]",
            "type X = ctor[]",
            "type X = ctor[u8,]",
            "x = 1",
            "type X = [u8; 1.0]",
            "forbid(x: u8 = 1)",
            "include()",
            "extend()",
            "inherit()",
            "include(1,2)",
            "inherit(merge,join)",
            "include(1) include(2)",
            "x: u8 = [1,2)",
            "x: u8 = 1<2<3",
            "x: u8 = 1..2..3",
            "x: u8 = 1 +",
            "[arch]\nconst x: u8 = 1",
            "x: u8 = 1\n+ 2",
            "x: u8 = 0b102",
            "x: string = \"\\u{d800}\"",
            "x: string = \"unterminated",
            "x: u8 = 1\r",
        ] {
            let mut c0 = Cursor::create(source);
            let error = Document::<&str>::lex_with_context(&mut c0, &mut "fixture").expect_err(source);

            assert_eq!(c0.index(), 0, "{source}");
            for label in error.annotations().list().expect("annotations").iter() {
                let span = label.target();

                assert!(span.start() <= source.len());
                if span.start() < source.len() {
                    assert!(source.get(span.range()).is_some(), "{source}");
                }
            }
        }
    }
    #[test]
    fn related_locations_survive_nested_error_families() {
        for (source, primary, related) in [
            ("x: [u8] = [1,2", 14, 10),
            ("x: u8 = 1 +", 11, 10),
            ("x: [u8] = [1 2]", 13, 11),
        ] {
            let mut c0 = Cursor::create(source);
            let error = Document::<&str>::lex_with_context(&mut c0, &mut "fixture").expect_err("malformed");
            let labels = error.annotations().list().expect("labels");

            assert_eq!(labels.first().target().start(), primary, "{source}");

            assert!(
                labels.rest().iter().any(|label| label.target().start() == related),
                "{source}"
            );
        }
    }
    #[test]
    fn former_limits_do_not_restrict_accepted_input() {
        let source = " ".repeat(17 * 1024 * 1024);
        let mut c0 = Cursor::create(source.as_str());

        assert!(Document::<&str>::lex_with_context(&mut c0, &mut "large").is_ok());
        let source = "\n".repeat(1_000_001);
        let mut c0 = Cursor::create(source.as_str());

        assert!(Document::<&str>::lex_with_context(&mut c0, &mut "many").is_ok());
        std::thread::Builder::new()
            .stack_size(32 * 1024 * 1024)
            .spawn(|| {
                for source in [
                    format!("x: u8 = {}1{}", "(".repeat(140), ")".repeat(140)),
                    format!("type T = {}u8{}", "[".repeat(140), "]".repeat(140)),
                    format!("x: u8 = {}1", "2**".repeat(140)),
                ] {
                    let mut c0 = Cursor::create(source.as_str());

                    assert!(Document::<&str>::lex_with_context(&mut c0, &mut "deep").is_ok());
                }
            })
            .expect("test thread")
            .join()
            .expect("nested grammar");
    }
    #[test]
    fn rendered_reports_explain_the_failure_and_its_context() {
        for (source, message, context) in [
            (
                "x: [u8] = [1, 2",
                "expected `]` to close this bracketed group",
                "this group starts here",
            ),
            (
                "x: u8 = 1 +",
                "expected a right operand",
                "this operator requires a right operand",
            ),
            (
                "x: [u8] = [1 2]",
                "expected `,` or the closing delimiter",
                "this list element is already complete",
            ),
            (
                "rule r = >=",
                "expected a postfix-expression bound",
                "this predicate requires a bound",
            ),
            ("x: u8 = ()", "expected a literal", "this expression group starts here"),
            ("type X = []", "expected a type name", "this type group starts here"),
            (
                "include()",
                "requires exactly one expression",
                "this source argument group starts here",
            ),
        ] {
            let mut c0 = Cursor::create(source);
            let error = Document::<&str>::lex_with_context(&mut c0, &mut "report").expect_err(source);
            let attached = error.attach(source);
            let mut rendered = String::new();

            Textual::<'_, _, Fancy, _>::new(&mut rendered)
                .render_mut_with_input(Present::with_input(FancySettings::standard()), &attached)
                .expect("render report");

            assert!(rendered.contains(message), "{source}\n{rendered}");

            assert!(rendered.contains(context), "{source}\n{rendered}");

            assert!(!rendered.contains('\u{1b}'));
        }
    }
    #[test]
    fn unsupported_separators_are_lexical_errors() {
        for separator in ['\u{b}', '\u{c}', '\u{85}', '\u{2028}', '\u{2029}'] {
            for source in [format!("x: u8 = 1{separator}"), format!("x: u8 = (1{separator}+2)")] {
                let mut c0 = Cursor::create(source.as_str());

                assert!(Document::<&str>::lex_with_context(&mut c0, &mut "separator").is_err());

                assert_eq!(c0.index(), 0);
            }
        }
    }
    #[test]
    fn completed_document_owns_the_group_storage_domain() {
        let source = "const message: string = (\"λ\\u{1f408}\")";
        let mut c0 = Cursor::create(source);
        let document = Document::lex_with_context(&mut c0, &mut "owned").expect("document");
        let _send: &dyn Send = &document;
        let _sync: &dyn Sync = &document;

        let Statement::Constant(binding) = document.prelude()[0].value() else {
            unreachable!("constant")
        };

        let super::super::expression::Expr::Parenthesized(group) = binding.value() else {
            unreachable!("group")
        };

        let super::super::expression::Expr::Literal(super::super::expression::Literal::String(value)) =
            group.value().as_ref()
        else {
            unreachable!("string")
        };

        assert_eq!(document.context().storage().try_resolve_tagged(value.id()), Some("λ🐈"));
    }
}

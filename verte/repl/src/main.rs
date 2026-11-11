//! Interactive parser REPL for Verte.

use core::error::Error;

use aldebaran::{
    prelude::{Cursor, Lex, Report as _, Span, Spanned},
    report::render::{
        RenderMut as _,
        textual::{
            Present, Textual,
            offload::fancy::{Fancy, FancySettings, Fancyness},
        },
    },
};
use rustyline::{DefaultEditor, error::ReadlineError};
use verte_parse::{
    source::Source,
    syntax::{
        declaration::{Field, Statement},
        document::{Document, Line, Section},
    },
};

/// Run the Verte parser REPL.
fn main() -> Result<(), Box<dyn Error>> {
    let mut editor = DefaultEditor::new()?;
    let mut source_text = String::new();

    println!("Verte parser REPL. Type :help for commands.");

    loop {
        let prompt = if source_text.is_empty() { "verte> " } else { "  ...> " };
        let line = match editor.readline(prompt) {
            Ok(line) => line,
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => break,
            Err(error) => return Err(error.into()),
        };

        let command = line.trim();

        if !line.is_empty() {
            editor.add_history_entry(line.as_str())?;
        }

        match command {
            ":help" => println!("Enter a Verte document, then use :parse. Commands: :help, :parse, :clear, :quit."),
            ":parse" => {
                if source_text.trim().is_empty() {
                    println!("No input to parse.");
                } else {
                    let identity = "<stdin>";
                    let source = Source::new(identity, source_text.as_str());
                    let mut cursor = Cursor::create(source.text());
                    let mut context = source.identity().to_owned();

                    match Document::lex_with_context(&mut cursor, &mut context) {
                        Ok(document) => {
                            print_ast(document.prelude(), document.sections(), source.text());
                        },
                        Err(error) => {
                            let attached = error.attach(source.text());
                            let mut rendered = String::new();

                            Textual::<'_, _, Fancy, _>::new(&mut rendered).render_mut_with_input(
                                Present::with_input(FancySettings::level(Fancyness::Outblown)),
                                &attached,
                            )?;
                            // `str` source metadata renders as `<unknown>` in Aldebaran.
                            let rendered = rendered.replacen("<unknown>", identity, 1);

                            println!("{rendered}");
                        },
                    }
                }

                source_text.clear();
            },
            ":clear" => source_text.clear(),
            ":q" | ":quit" => break,
            _ => {
                source_text.push_str(&line);
                source_text.push('\n');
            },
        }
    }

    Ok(())
}

/// Print source-ordered declarations, sections, and fields as a compact tree.
fn print_ast(prelude: &[Line<Statement>], sections: &[Section], source: &str) {
    println!("Document ({} declarations, {} sections)", prelude.len(), sections.len());

    let item_count = prelude.len() + sections.len();
    let mut item_index = 0;

    for line in prelude {
        item_index += 1;

        let branch = if item_index == item_count {
            "└──"
        } else {
            "├──"
        };
        let statement = line.value();
        let kind = statement_kind(statement);
        let text = compact_fragment(source, statement.span());

        println!("{branch} {kind}: {text}");
    }

    for section in sections {
        item_index += 1;

        let is_last = item_index == item_count;
        let branch = if is_last { "└──" } else { "├──" };
        let prefix = if is_last { "    " } else { "│   " };
        let header = compact_fragment(source, section.header().value().span());

        println!("{branch} section: {header}");

        let fields = section.fields();

        if fields.is_empty() {
            println!("{prefix}└── (empty)");
        }

        for (field_index, line) in fields.iter().enumerate() {
            let branch = if field_index + 1 == fields.len() {
                "└──"
            } else {
                "├──"
            };
            let field = line.value();
            let kind = field_kind(field);
            let text = compact_fragment(source, field.span());

            println!("{prefix}{branch} {kind}: {text}");
        }
    }
}

/// Retrieve the declaration label shown in the AST tree.
fn statement_kind(statement: &Statement) -> &'static str {
    match statement {
        Statement::Include(_) => "include",
        Statement::Extend(_) => "extend",
        Statement::Inherit(_) => "inherit",
        Statement::Domain(_) => "domain",
        Statement::Type(_) => "type",
        Statement::Constant(_) => "constant",
        Statement::Rule(_) => "rule",
        Statement::Field(_) => "field",
    }
}

/// Retrieve the field label shown in the AST tree.
fn field_kind(field: &Field) -> &'static str {
    match field {
        Field::Plain(_) => "field",
        Field::Policy(_) => "policy field",
    }
}

/// Return a one-line source excerpt for a syntax span.
///
/// Long excerpts are capped at 88 Unicode scalar values, including an ellipsis.
fn compact_fragment(source: &str, span: Span) -> String {
    let Some(fragment) = source.get(span.range()) else {
        return String::from("<invalid source span>");
    };
    let compact = fragment
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let mut characters = compact.chars();
    let excerpt = characters.by_ref().take(87).collect::<String>();

    match characters.next() {
        Some(_) => format!("{excerpt}…"),
        None => excerpt,
    }
}

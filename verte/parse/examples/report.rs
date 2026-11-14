//! Render the first parser failure with Aldebaran's source annotations.

use std::{env, fs};

use aldebaran::{
    prelude::{Cursor, Lex, Report as _},
    report::render::{
        RenderMut as _,
        textual::{
            Present, Textual,
            offload::fancy::{Fancy, FancySettings},
        },
    },
};
use verte_parse::{source::Source, syntax::document::Document};

/// Parse a supplied UTF-8 file, or demonstrate a missing array delimiter.
fn main() -> Result<(), Box<dyn core::error::Error>> {
    let (identity, text) = match env::args().nth(1) {
        Some(path) => {
            let text = fs::read_to_string(&path)?;

            (path, text)
        },
        None => (
            String::from("example.verte"),
            String::from("[layout]\npages: [u8] = [1, 2"),
        ),
    };

    let source = Source::new(identity.as_str(), text.as_str());

    let mut c0 = Cursor::create(source.text());

    match Document::lex_with_context(&mut c0, &mut source.identity().to_owned()) {
        Ok(document) => println!(
            "{} prelude statements and {} sections",
            document.prelude().len(),
            document.sections().len()
        ),
        Err(error) => {
            let attached = error.attach(source.text());
            let mut rendered = String::new();

            Textual::<'_, _, Fancy, _>::new(&mut rendered)
                .render_mut_with_input(Present::with_input(FancySettings::standard()), &attached)?;
            println!("{}\n{rendered}", source.identity());
        },
    }
    Ok(())
}

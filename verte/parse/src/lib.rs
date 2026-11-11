//! Lossless concrete syntax for Verte 0.7.
//!
//! Lexical and syntactic productions implement Aldebaran's `Lex` contract.
//! Documents borrow the original source text and own their storage domain. Resolution,
//! evaluation, and declaration uniqueness belong to subsequent language phases.
//!
//! ```
//! use aldebaran::prelude::{Cursor, Lex};
//! use verte_parse::syntax::document::Document;
//!
//! let mut c0 = Cursor::create("const page: size = 4_KiB\n[arch]\npage_size: size = page");
//! let document =
//!     Document::lex_with_context(&mut c0, &mut "settings.verte").expect("valid Verte syntax");
//! assert_eq!(document.prelude().len(), 1);
//! assert_eq!(document.sections().len(), 1);
//! ```
#![no_std]
extern crate alloc;
pub mod scan;
pub mod source;
pub mod syntax;

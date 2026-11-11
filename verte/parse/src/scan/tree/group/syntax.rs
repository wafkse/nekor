//! Structural expectations for already recognized delimiter groups.

use aldebaran::prelude::Assert;

use super::{Delimiter, Group};
use crate::scan::tree::TokenTree;

/// The delimiter kind required by an enclosing grammar production.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Assert)]
#[assert(input(TokenTree))]
pub enum Fragment {
    /// A parenthesized group.
    #[assert(matches(TokenTree::Group(Group { delimiter: Delimiter::Parenthesis(..), .. })) as "a parenthesized group")]
    Parenthesis,

    /// A bracketed group.
    #[assert(matches(TokenTree::Group(Group { delimiter: Delimiter::Bracket(..), .. })) as "a bracketed group")]
    Bracket,

    /// A braced group.
    #[assert(matches(TokenTree::Group(Group { delimiter: Delimiter::Brace(..), .. })) as "a braced group")]
    Brace,
}

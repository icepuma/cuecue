//! Lexer, parser and lossless syntax tree for cuecue source files (see docs/ARCHITECTURE.md).

mod kind;
mod lexer;

pub use kind::SyntaxKind;
pub use lexer::{LexError, Lexed, Token, lex};

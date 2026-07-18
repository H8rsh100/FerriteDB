//! SQL crate — lexer, recursive-descent parser, and AST.
//!
//! Phase 4 will implement the full pipeline. For Phase 0 the module tree
//! is declared so the workspace compiles cleanly.

pub mod lexer;
pub mod ast;
pub mod parser;

pub use lexer::{Lexer, Token};
pub use ast::Statement;
pub use parser::{Parser, ParseError};

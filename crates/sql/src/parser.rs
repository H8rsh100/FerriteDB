//! Recursive-descent parser — produces an AST from a `Token` stream.
//!
//! Phase 4 implementation. For Phase 0 only the error type and struct
//! are declared.

use crate::ast::Statement;
use crate::lexer::Token;

/// A parse error with a human-readable message and source location.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "parse error at {}:{}: {}", self.line, self.col, self.message)
    }
}

impl std::error::Error for ParseError {}

/// Parses a token stream into a sequence of `Statement`s.
pub struct Parser {
    /// Token stream produced by the Lexer. Phase 4 will consume this.
    #[allow(dead_code)]
    tokens: Vec<(Token, crate::lexer::Span)>,
    /// Current read position into `tokens`. Phase 4 will advance this.
    #[allow(dead_code)]
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<(Token, crate::lexer::Span)>) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Parses all statements from the token stream.
    /// Phase 4 implementation.
    pub fn parse(&mut self) -> Result<Vec<Statement>, ParseError> {
        // Phase 4 implementation.
        Ok(vec![])
    }
}

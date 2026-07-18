//! Lexer — tokenises SQL text into a stream of `Token`s.
//!
//! Phase 4 implementation. For Phase 0, the enum variants define the
//! full intended token vocabulary so other stubs can reference them.

/// A single lexical token produced by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // ── Keywords ──────────────────────────────────────────────────────────
    Select, From, Where, Insert, Into, Values, Update, Set,
    Delete, Create, Table, Drop, Join, On, And, Or, Not,
    Null, True, False, As, Distinct, Order, By, Asc, Desc,
    Limit, Offset, Primary, Key, Unique, Index, Begin, Commit, Abort,
    // ── Literals ──────────────────────────────────────────────────────────
    IntLit(i64),
    FloatLit(f64),
    /// String literal (single-quoted, with escape sequences resolved).
    StrLit(String),
    // ── Identifiers ───────────────────────────────────────────────────────
    Ident(String),
    // ── Operators & punctuation ───────────────────────────────────────────
    Eq, Neq, Lt, Gt, Lte, Gte,
    Plus, Minus, Star, Slash,
    LParen, RParen, Comma, Semicolon, Dot,
    // ── Meta ──────────────────────────────────────────────────────────────
    /// End of input.
    Eof,
}

/// Source location for error reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

/// Produces a `Token` stream from a SQL string.
pub struct Lexer {
    _input: Vec<char>,
    _pos: usize,
    _line: usize,
    _col: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            _input: input.chars().collect(),
            _pos: 0,
            _line: 1,
            _col: 1,
        }
    }

    /// Tokenises the entire input, returning `(token, span)` pairs.
    /// Phase 4 implementation.
    pub fn tokenize(&mut self) -> Result<Vec<(Token, Span)>, String> {
        // Phase 4 implementation.
        Ok(vec![(Token::Eof, Span { line: 1, col: 1 })])
    }
}

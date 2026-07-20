//! Lexer — tokenises SQL text into a stream of `Token`s.

/// A single lexical token produced by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // ── Keywords ──────────────────────────────────────────────────────────
    Select, From, Where, Insert, Into, Values, Update, Set,
    Delete, Create, Table, Drop, Join, On, And, Or, Not,
    Null, True, False, As, Distinct, Order, By, Asc, Desc,
    Limit, Offset, Primary, Key, Unique, Index, Begin, Commit, Abort,
    IntType, BigIntType, VarcharType, BooleanType, FloatType,
    // ── Literals ──────────────────────────────────────────────────────────
    IntLit(i64),
    FloatLit(f64),
    /// String literal (single-quoted).
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
    input: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.input.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        if let Some(ch) = self.peek() {
            self.pos += 1;
            if ch == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
            Some(ch)
        } else {
            None
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                self.advance();
            } else if ch == '-' && self.peek_next() == Some('-') {
                // Single-line comment --
                while let Some(c) = self.peek() {
                    if c == '\n' {
                        break;
                    }
                    self.advance();
                }
            } else {
                break;
            }
        }
    }

    /// Tokenises the entire input, returning `(token, span)` pairs.
    pub fn tokenize(&mut self) -> Result<Vec<(Token, Span)>, String> {
        let mut tokens = Vec::new();

        loop {
            self.skip_whitespace_and_comments();
            let span = Span { line: self.line, col: self.col };

            let ch = match self.peek() {
                Some(c) => c,
                None => {
                    tokens.push((Token::Eof, span));
                    break;
                }
            };

            match ch {
                '(' => { self.advance(); tokens.push((Token::LParen, span)); }
                ')' => { self.advance(); tokens.push((Token::RParen, span)); }
                ',' => { self.advance(); tokens.push((Token::Comma, span)); }
                ';' => { self.advance(); tokens.push((Token::Semicolon, span)); }
                '.' => { self.advance(); tokens.push((Token::Dot, span)); }
                '+' => { self.advance(); tokens.push((Token::Plus, span)); }
                '-' => { self.advance(); tokens.push((Token::Minus, span)); }
                '*' => { self.advance(); tokens.push((Token::Star, span)); }
                '/' => { self.advance(); tokens.push((Token::Slash, span)); }
                '=' => { self.advance(); tokens.push((Token::Eq, span)); }
                '!' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        tokens.push((Token::Neq, span));
                    } else {
                        return Err(format!("unexpected character '!' at line {}, col {}", span.line, span.col));
                    }
                }
                '<' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        tokens.push((Token::Lte, span));
                    } else if self.peek() == Some('>') {
                        self.advance();
                        tokens.push((Token::Neq, span));
                    } else {
                        tokens.push((Token::Lt, span));
                    }
                }
                '>' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        tokens.push((Token::Gte, span));
                    } else {
                        tokens.push((Token::Gt, span));
                    }
                }
                '\'' => {
                    self.advance();
                    let mut s = String::new();
                    loop {
                        match self.advance() {
                            Some('\'') => {
                                if self.peek() == Some('\'') {
                                    // Escaped single quote ''
                                    self.advance();
                                    s.push('\'');
                                } else {
                                    break;
                                }
                            }
                            Some(c) => s.push(c),
                            None => return Err(format!("Unterminated string literal starting at line {}, col {}", span.line, span.col)),
                        }
                    }
                    tokens.push((Token::StrLit(s), span));
                }
                c if c.is_ascii_digit() => {
                    let mut num_str = String::new();
                    let mut is_float = false;
                    while let Some(digit) = self.peek() {
                        if digit.is_ascii_digit() {
                            num_str.push(digit);
                            self.advance();
                        } else if digit == '.' && !is_float && self.peek_next().map_or(false, |n| n.is_ascii_digit()) {
                            is_float = true;
                            num_str.push('.');
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    if is_float {
                        let val: f64 = num_str.parse().map_err(|e| format!("invalid float '{num_str}': {e}"))?;
                        tokens.push((Token::FloatLit(val), span));
                    } else {
                        let val: i64 = num_str.parse().map_err(|e| format!("invalid integer '{num_str}': {e}"))?;
                        tokens.push((Token::IntLit(val), span));
                    }
                }
                c if c.is_ascii_alphabetic() || c == '_' => {
                    let mut ident = String::new();
                    while let Some(sym) = self.peek() {
                        if sym.is_ascii_alphanumeric() || sym == '_' {
                            ident.push(sym);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    let token = match ident.to_uppercase().as_str() {
                        "SELECT" => Token::Select,
                        "FROM" => Token::From,
                        "WHERE" => Token::Where,
                        "INSERT" => Token::Insert,
                        "INTO" => Token::Into,
                        "VALUES" => Token::Values,
                        "UPDATE" => Token::Update,
                        "SET" => Token::Set,
                        "DELETE" => Token::Delete,
                        "CREATE" => Token::Create,
                        "TABLE" => Token::Table,
                        "DROP" => Token::Drop,
                        "JOIN" => Token::Join,
                        "ON" => Token::On,
                        "AND" => Token::And,
                        "OR" => Token::Or,
                        "NOT" => Token::Not,
                        "NULL" => Token::Null,
                        "TRUE" => Token::True,
                        "FALSE" => Token::False,
                        "AS" => Token::As,
                        "DISTINCT" => Token::Distinct,
                        "ORDER" => Token::Order,
                        "BY" => Token::By,
                        "ASC" => Token::Asc,
                        "DESC" => Token::Desc,
                        "LIMIT" => Token::Limit,
                        "OFFSET" => Token::Offset,
                        "PRIMARY" => Token::Primary,
                        "KEY" => Token::Key,
                        "UNIQUE" => Token::Unique,
                        "INDEX" => Token::Index,
                        "BEGIN" => Token::Begin,
                        "COMMIT" => Token::Commit,
                        "ABORT" => Token::Abort,
                        "INT" | "INTEGER" => Token::IntType,
                        "BIGINT" => Token::BigIntType,
                        "VARCHAR" => Token::VarcharType,
                        "BOOLEAN" | "BOOL" => Token::BooleanType,
                        "FLOAT" | "DOUBLE" => Token::FloatType,
                        _ => Token::Ident(ident),
                    };
                    tokens.push((token, span));
                }
                other => return Err(format!("unexpected character '{other}' at line {}, col {}", span.line, span.col)),
            }
        }

        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_create_table() {
        let sql = "CREATE TABLE users (id INT, name VARCHAR(255));";
        let mut lexer = Lexer::new(sql);
        let tokens: Vec<Token> = lexer.tokenize().unwrap().into_iter().map(|(t, _)| t).collect();
        assert_eq!(
            tokens,
            vec![
                Token::Create, Token::Table, Token::Ident("users".into()),
                Token::LParen, Token::Ident("id".into()), Token::IntType, Token::Comma,
                Token::Ident("name".into()), Token::VarcharType, Token::LParen, Token::IntLit(255), Token::RParen,
                Token::RParen, Token::Semicolon, Token::Eof
            ]
        );
    }

    #[test]
    fn lex_select_where() {
        let sql = "SELECT id, name FROM users WHERE age >= 18 AND name = 'Alice';";
        let mut lexer = Lexer::new(sql);
        let tokens: Vec<Token> = lexer.tokenize().unwrap().into_iter().map(|(t, _)| t).collect();
        assert_eq!(
            tokens,
            vec![
                Token::Select, Token::Ident("id".into()), Token::Comma, Token::Ident("name".into()),
                Token::From, Token::Ident("users".into()),
                Token::Where, Token::Ident("age".into()), Token::Gte, Token::IntLit(18),
                Token::And, Token::Ident("name".into()), Token::Eq, Token::StrLit("Alice".into()),
                Token::Semicolon, Token::Eof
            ]
        );
    }
}

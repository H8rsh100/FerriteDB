//! Recursive-descent parser — produces an AST from a `Token` stream.

use crate::ast::*;
use crate::lexer::{Span, Token};

/// A parse error with a human-readable message and source location.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    tokens: Vec<(Token, Span)>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<(Token, Span)>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &(Token, Span) {
        if self.pos < self.tokens.len() {
            &self.tokens[self.pos]
        } else {
            &self.tokens[self.tokens.len() - 1]
        }
    }

    fn advance(&mut self) -> (Token, Span) {
        let current = self.peek().clone();
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        current
    }

    fn check(&self, token: &Token) -> bool {
        &self.peek().0 == token
    }

    fn match_token(&mut self, token: &Token) -> bool {
        if self.check(token) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, expected: &Token) -> Result<Span, ParseError> {
        let (token, span) = self.peek().clone();
        if &token == expected {
            self.advance();
            Ok(span)
        } else {
            Err(ParseError {
                message: format!("expected token {:?}, found {:?}", expected, token),
                line: span.line,
                col: span.col,
            })
        }
    }

    fn expect_ident(&mut self) -> Result<(String, Span), ParseError> {
        let (token, span) = self.peek().clone();
        if let Token::Ident(name) = token {
            self.advance();
            Ok((name, span))
        } else {
            Err(ParseError {
                message: format!("expected identifier, found {:?}", token),
                line: span.line,
                col: span.col,
            })
        }
    }

    /// Parses all statements from the token stream until EOF.
    pub fn parse(&mut self) -> Result<Vec<Statement>, ParseError> {
        let mut statements = Vec::new();
        while !self.check(&Token::Eof) {
            let stmt = self.parse_statement()?;
            statements.push(stmt);
            self.match_token(&Token::Semicolon);
        }
        Ok(statements)
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        let (token, _) = self.peek().clone();
        match token {
            Token::Create => self.parse_create_table(),
            Token::Insert => self.parse_insert(),
            Token::Select => self.parse_select(),
            Token::Update => self.parse_update(),
            Token::Delete => self.parse_delete(),
            Token::Begin => { self.advance(); Ok(Statement::BeginTransaction) }
            Token::Commit => { self.advance(); Ok(Statement::Commit) }
            Token::Abort => { self.advance(); Ok(Statement::Abort) }
            _ => Err(ParseError {
                message: format!("unexpected statement start token {:?}", token),
                line: self.peek().1.line,
                col: self.peek().1.col,
            }),
        }
    }

    fn parse_create_table(&mut self) -> Result<Statement, ParseError> {
        self.expect(&Token::Create)?;
        self.expect(&Token::Table)?;
        let (name, _) = self.expect_ident()?;
        self.expect(&Token::LParen)?;

        let mut columns = Vec::new();
        loop {
            let (col_name, _) = self.expect_ident()?;
            let data_type = self.parse_sql_type()?;
            let mut nullable = true;

            if self.match_token(&Token::Not) {
                self.expect(&Token::Null)?;
                nullable = false;
            } else if self.match_token(&Token::Null) {
                nullable = true;
            }

            columns.push(ColumnDef {
                name: col_name,
                data_type,
                nullable,
            });

            if !self.match_token(&Token::Comma) {
                break;
            }
        }
        self.expect(&Token::RParen)?;

        Ok(Statement::CreateTable { name, columns })
    }

    fn parse_sql_type(&mut self) -> Result<SqlType, ParseError> {
        let (token, span) = self.peek().clone();
        match token {
            Token::IntType => { self.advance(); Ok(SqlType::Int) }
            Token::BigIntType => { self.advance(); Ok(SqlType::BigInt) }
            Token::BooleanType => { self.advance(); Ok(SqlType::Boolean) }
            Token::FloatType => { self.advance(); Ok(SqlType::Float) }
            Token::VarcharType => {
                self.advance();
                self.expect(&Token::LParen)?;
                let (len_tok, len_span) = self.peek().clone();
                let len = if let Token::IntLit(n) = len_tok {
                    self.advance();
                    n as usize
                } else {
                    return Err(ParseError {
                        message: "expected length in VARCHAR".into(),
                        line: len_span.line,
                        col: len_span.col,
                    });
                };
                self.expect(&Token::RParen)?;
                Ok(SqlType::Varchar(len))
            }
            _ => Err(ParseError {
                message: format!("invalid SQL type token {:?}", token),
                line: span.line,
                col: span.col,
            }),
        }
    }

    fn parse_insert(&mut self) -> Result<Statement, ParseError> {
        self.expect(&Token::Insert)?;
        self.expect(&Token::Into)?;
        let (table, _) = self.expect_ident()?;
        self.expect(&Token::Values)?;
        self.expect(&Token::LParen)?;

        let mut values = Vec::new();
        loop {
            let expr = self.parse_expr()?;
            values.push(expr);
            if !self.match_token(&Token::Comma) {
                break;
            }
        }
        self.expect(&Token::RParen)?;

        Ok(Statement::Insert { table, values })
    }

    fn parse_select(&mut self) -> Result<Statement, ParseError> {
        self.expect(&Token::Select)?;

        let columns = if self.match_token(&Token::Star) {
            SelectList::Star
        } else {
            let mut exprs = Vec::new();
            loop {
                exprs.push(self.parse_expr()?);
                if !self.match_token(&Token::Comma) {
                    break;
                }
            }
            SelectList::Exprs(exprs)
        };

        self.expect(&Token::From)?;
        let (from, _) = self.expect_ident()?;

        let mut joins = Vec::new();
        while self.match_token(&Token::Join) {
            let (j_table, _) = self.expect_ident()?;
            self.expect(&Token::On)?;
            let condition = self.parse_expr()?;
            joins.push(JoinClause { table: j_table, condition });
        }

        let filter = if self.match_token(&Token::Where) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        let mut order_by = Vec::new();
        if self.match_token(&Token::Order) {
            self.expect(&Token::By)?;
            loop {
                let expr = self.parse_expr()?;
                let dir = if self.match_token(&Token::Desc) {
                    Order::Desc
                } else {
                    self.match_token(&Token::Asc);
                    Order::Asc
                };
                order_by.push((expr, dir));
                if !self.match_token(&Token::Comma) {
                    break;
                }
            }
        }

        let limit = if self.match_token(&Token::Limit) {
            let (tok, span) = self.peek().clone();
            if let Token::IntLit(n) = tok {
                self.advance();
                Some(n as u64)
            } else {
                return Err(ParseError {
                    message: "expected integer for LIMIT".into(),
                    line: span.line,
                    col: span.col,
                });
            }
        } else {
            None
        };

        let offset = if self.match_token(&Token::Offset) {
            let (tok, span) = self.peek().clone();
            if let Token::IntLit(n) = tok {
                self.advance();
                Some(n as u64)
            } else {
                return Err(ParseError {
                    message: "expected integer for OFFSET".into(),
                    line: span.line,
                    col: span.col,
                });
            }
        } else {
            None
        };

        Ok(Statement::Select {
            columns,
            from,
            joins,
            filter,
            order_by,
            limit,
            offset,
        })
    }

    fn parse_update(&mut self) -> Result<Statement, ParseError> {
        self.expect(&Token::Update)?;
        let (table, _) = self.expect_ident()?;
        self.expect(&Token::Set)?;

        let mut assignments = Vec::new();
        loop {
            let (col_name, _) = self.expect_ident()?;
            self.expect(&Token::Eq)?;
            let expr = self.parse_expr()?;
            assignments.push((col_name, expr));
            if !self.match_token(&Token::Comma) {
                break;
            }
        }

        let filter = if self.match_token(&Token::Where) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        Ok(Statement::Update { table, assignments, filter })
    }

    fn parse_delete(&mut self) -> Result<Statement, ParseError> {
        self.expect(&Token::Delete)?;
        self.expect(&Token::From)?;
        let (table, _) = self.expect_ident()?;

        let filter = if self.match_token(&Token::Where) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        Ok(Statement::Delete { table, filter })
    }

    // Expression parsing with operator precedence
    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_and()?;
        while self.check(&Token::Or) {
            let (_, span) = self.advance().clone();
            let right = self.parse_and()?;
            left = Expr::BinOp {
                op: BinOp::Or,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_equality()?;
        while self.check(&Token::And) {
            let (_, span) = self.advance().clone();
            let right = self.parse_equality()?;
            left = Expr::BinOp {
                op: BinOp::And,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_comparison()?;
        loop {
            let (tok, span) = self.peek().clone();
            let op = match tok {
                Token::Eq => BinOp::Eq,
                Token::Neq => BinOp::Neq,
                _ => break,
            };
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::BinOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_additive()?;
        loop {
            let (tok, span) = self.peek().clone();
            let op = match tok {
                Token::Lt => BinOp::Lt,
                Token::Gt => BinOp::Gt,
                Token::Lte => BinOp::Lte,
                Token::Gte => BinOp::Gte,
                _ => break,
            };
            self.advance();
            let right = self.parse_additive()?;
            left = Expr::BinOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let (tok, span) = self.peek().clone();
            let op = match tok {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::BinOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_primary()?;
        loop {
            let (tok, span) = self.peek().clone();
            let op = match tok {
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                _ => break,
            };
            self.advance();
            let right = self.parse_primary()?;
            left = Expr::BinOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let (token, span) = self.peek().clone();
        match token {
            Token::IntLit(n) => { self.advance(); Ok(Expr::IntLit(n)) }
            Token::FloatLit(f) => { self.advance(); Ok(Expr::FloatLit(f)) }
            Token::StrLit(s) => { self.advance(); Ok(Expr::StrLit(s)) }
            Token::True => { self.advance(); Ok(Expr::BoolLit(true)) }
            Token::False => { self.advance(); Ok(Expr::BoolLit(false)) }
            Token::Null => { self.advance(); Ok(Expr::Null) }
            Token::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(expr)
            }
            Token::Ident(name) => {
                self.advance();
                if self.match_token(&Token::Dot) {
                    let (col_name, _) = self.expect_ident()?;
                    Ok(Expr::Column {
                        table: Some(name),
                        name: col_name,
                    })
                } else {
                    Ok(Expr::Column {
                        table: None,
                        name,
                    })
                }
            }
            _ => Err(ParseError {
                message: format!("unexpected token in expression {:?}", token),
                line: span.line,
                col: span.col,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse_str(sql: &str) -> Result<Vec<Statement>, ParseError> {
        let mut lexer = Lexer::new(sql);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        parser.parse()
    }

    #[test]
    fn parse_create_table_statement() {
        let sql = "CREATE TABLE users (id INT NOT NULL, name VARCHAR(255));";
        let stmts = parse_str(sql).unwrap();
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            Statement::CreateTable { name, columns } => {
                assert_eq!(name, "users");
                assert_eq!(columns.len(), 2);
                assert_eq!(columns[0].name, "id");
                assert_eq!(columns[0].data_type, SqlType::Int);
                assert!(!columns[0].nullable);
            }
            _ => panic!("expected CreateTable"),
        }
    }

    #[test]
    fn parse_select_join_where() {
        let sql = "SELECT u.id, u.name FROM users JOIN orders ON u.id = orders.user_id WHERE u.age >= 18;";
        let stmts = parse_str(sql).unwrap();
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            Statement::Select { from, joins, filter, .. } => {
                assert_eq!(from, "users");
                assert_eq!(joins.len(), 1);
                assert_eq!(joins[0].table, "orders");
                assert!(filter.is_some());
            }
            _ => panic!("expected Select"),
        }
    }

    #[test]
    fn parse_insert_update_delete() {
        assert!(parse_str("INSERT INTO users VALUES (1, 'Alice');").is_ok());
        assert!(parse_str("UPDATE users SET name = 'Bob' WHERE id = 1;").is_ok());
        assert!(parse_str("DELETE FROM users WHERE id = 1;").is_ok());
    }
}

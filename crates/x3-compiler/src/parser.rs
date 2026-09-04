//! X3 language parser: converts source text to an AST.
//!
//! This is a recursive-descent parser for the X3 language. It handles
//! function declarations with real parameter parsing, return types,
//! and body statements, type annotations, expressions, and statements.

/// Token types produced by the lexer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenKind {
    // Literals
    Int(i64),
    Str(String),
    Bool(bool),
    // Identifiers and keywords
    Ident(String),
    KwFn,
    KwLet,
    KwReturn,
    KwIf,
    KwElse,
    KwWhile,
    KwPub,
    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Eq,
    EqEq,
    BangEq,
    Lt,
    Gt,
    // Delimiters
    LParen,
    RParen,
    LBrace,
    RBrace,
    Colon,
    Comma,
    Semicolon,
    Arrow, // ->
    // Special
    Eof,
}

/// A token with its source position.
#[derive(Clone, Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub line: u32,
    pub col: u32,
}

/// Parse errors.
#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    UnexpectedToken {
        expected: String,
        got: String,
        line: u32,
    },
    UnexpectedEof,
    InvalidLiteral(String),
}

/// A type in the X3 type system.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeExpr {
    I64,
    U64,
    Bool,
    Str,
    Unit,
    Named(String),
}

impl TypeExpr {
    /// Parse a TypeExpr from token kind.
    fn from_token(kind: &TokenKind) -> Option<Self> {
        match kind {
            TokenKind::Ident(s) if s == "i64" => Some(TypeExpr::I64),
            TokenKind::Ident(s) if s == "u64" => Some(TypeExpr::U64),
            TokenKind::Ident(s) if s == "bool" => Some(TypeExpr::Bool),
            TokenKind::Ident(s) if s == "str" => Some(TypeExpr::Str),
            TokenKind::Ident(s) if s == "unit" => Some(TypeExpr::Unit),
            _ => None,
        }
    }
}

/// An expression node.
#[derive(Clone, Debug)]
pub enum Expr {
    IntLit(i64),
    BoolLit(bool),
    StrLit(String),
    Ident(String),
    BinOp {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Call {
        callee: String,
        args: Vec<Expr>,
    },
    Block(Vec<Stmt>),
    If {
        cond: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Option<Box<Expr>>,
    },
    Assign {
        name: String,
        value: Box<Expr>,
    },
}

/// A binary operator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Gt,
}

/// A statement node.
#[derive(Clone, Debug)]
pub enum Stmt {
    Let {
        name: String,
        ty: Option<TypeExpr>,
        value: Expr,
    },
    Return(Option<Expr>),
    Expr(Expr),
    While {
        cond: Expr,
        body: Box<Expr>,
    },
}

/// A function parameter with name and type annotation.
#[derive(Clone, Debug)]
pub struct Param {
    pub name: String,
    pub ty: TypeExpr,
}

/// A top-level function declaration with real parameter/return/body parsing.
#[derive(Clone, Debug)]
pub struct FnDecl {
    pub is_pub: bool,
    pub name: String,
    pub params: Vec<Param>,
    pub return_ty: TypeExpr,
    pub body: Vec<Stmt>,
}

/// The top-level AST for a single source file.
#[derive(Clone, Debug, Default)]
pub struct SourceFile {
    pub functions: Vec<FnDecl>,
}

/// Internal parser state: position in the token stream.
struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0 }
    }

    fn current(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn kind(&self) -> &TokenKind {
        &self.current().kind
    }

    fn check(&self, kind: &TokenKind) -> bool {
        self.kind() == kind
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
    }

    fn expect(&mut self, kind: &TokenKind, expected: &str) -> Result<(), ParseError> {
        if self.kind() == kind {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken {
                expected: expected.to_string(),
                got: format!("{:?}", self.kind()),
                line: self.current().line,
            })
        }
    }

    fn expect_any(&mut self, kinds: &[TokenKind], expected: &str) -> Result<(), ParseError> {
        if kinds.iter().any(|k| self.kind() == k) {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken {
                expected: expected.to_string(),
                got: format!("{:?}", self.kind()),
                line: self.current().line,
            })
        }
    }
}

/// Tokenize a source string into tokens.
pub fn tokenize(source: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    let mut chars = source.chars().peekable();
    let mut line = 1u32;
    let mut col = 1u32;

    while let Some(&c) = chars.peek() {
        match c {
            ' ' | '\t' | '\r' => {
                chars.next();
                col += 1;
            }
            '\n' => {
                chars.next();
                line += 1;
                col = 1;
            }
            '/' if {
                chars.next();
                chars.peek() == Some(&'/')
            } =>
            {
                // line comment
                while chars.peek().map(|&c| c != '\n').unwrap_or(false) {
                    chars.next();
                }
            }
            '0'..='9' => {
                let start_col = col;
                let mut num = String::new();
                while chars.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    num.push(chars.next().unwrap());
                    col += 1;
                }
                let n: i64 = num.parse().map_err(|_| ParseError::InvalidLiteral(num))?;
                tokens.push(Token {
                    kind: TokenKind::Int(n),
                    line,
                    col: start_col,
                });
            }
            'a'..='z' | 'A'..='Z' | '_' => {
                let start_col = col;
                let mut ident = String::new();
                while chars
                    .peek()
                    .map(|c| c.is_alphanumeric() || *c == '_')
                    .unwrap_or(false)
                {
                    ident.push(chars.next().unwrap());
                    col += 1;
                }
                let kind = match ident.as_str() {
                    "fn" => TokenKind::KwFn,
                    "let" => TokenKind::KwLet,
                    "return" => TokenKind::KwReturn,
                    "if" => TokenKind::KwIf,
                    "else" => TokenKind::KwElse,
                    "while" => TokenKind::KwWhile,
                    "pub" => TokenKind::KwPub,
                    "true" => TokenKind::Bool(true),
                    "false" => TokenKind::Bool(false),
                    _ => TokenKind::Ident(ident),
                };
                tokens.push(Token {
                    kind,
                    line,
                    col: start_col,
                });
            }
            '+' => {
                tokens.push(Token {
                    kind: TokenKind::Plus,
                    line,
                    col,
                });
                chars.next();
                col += 1;
            }
            '-' => {
                chars.next();
                col += 1;
                if chars.peek() == Some(&'>') {
                    chars.next();
                    col += 1;
                    tokens.push(Token {
                        kind: TokenKind::Arrow,
                        line,
                        col: col - 2,
                    });
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Minus,
                        line,
                        col: col - 1,
                    });
                }
            }
            '*' => {
                tokens.push(Token {
                    kind: TokenKind::Star,
                    line,
                    col,
                });
                chars.next();
                col += 1;
            }
            '/' => {
                tokens.push(Token {
                    kind: TokenKind::Slash,
                    line,
                    col,
                });
                chars.next();
                col += 1;
            }
            '=' => {
                chars.next();
                col += 1;
                if chars.peek() == Some(&'=') {
                    chars.next();
                    col += 1;
                    tokens.push(Token {
                        kind: TokenKind::EqEq,
                        line,
                        col: col - 2,
                    });
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Eq,
                        line,
                        col: col - 1,
                    });
                }
            }
            '!' => {
                chars.next();
                col += 1;
                if chars.peek() == Some(&'=') {
                    chars.next();
                    col += 1;
                    tokens.push(Token {
                        kind: TokenKind::BangEq,
                        line,
                        col: col - 2,
                    });
                }
            }
            '<' => {
                tokens.push(Token {
                    kind: TokenKind::Lt,
                    line,
                    col,
                });
                chars.next();
                col += 1;
            }
            '>' => {
                tokens.push(Token {
                    kind: TokenKind::Gt,
                    line,
                    col,
                });
                chars.next();
                col += 1;
            }
            '"' => {
                let start_col = col;
                chars.next(); // opening quote
                col += 1;
                let mut s = String::new();
                while chars.peek().map(|&c| c != '"').unwrap_or(false) {
                    s.push(chars.next().unwrap());
                    col += 1;
                }
                chars.next(); // closing quote
                col += 1;
                tokens.push(Token {
                    kind: TokenKind::Str(s),
                    line,
                    col: start_col,
                });
            }
            '(' => {
                tokens.push(Token {
                    kind: TokenKind::LParen,
                    line,
                    col,
                });
                chars.next();
                col += 1;
            }
            ')' => {
                tokens.push(Token {
                    kind: TokenKind::RParen,
                    line,
                    col,
                });
                chars.next();
                col += 1;
            }
            '{' => {
                tokens.push(Token {
                    kind: TokenKind::LBrace,
                    line,
                    col,
                });
                chars.next();
                col += 1;
            }
            '}' => {
                tokens.push(Token {
                    kind: TokenKind::RBrace,
                    line,
                    col,
                });
                chars.next();
                col += 1;
            }
            ':' => {
                tokens.push(Token {
                    kind: TokenKind::Colon,
                    line,
                    col,
                });
                chars.next();
                col += 1;
            }
            ',' => {
                tokens.push(Token {
                    kind: TokenKind::Comma,
                    line,
                    col,
                });
                chars.next();
                col += 1;
            }
            ';' => {
                tokens.push(Token {
                    kind: TokenKind::Semicolon,
                    line,
                    col,
                });
                chars.next();
                col += 1;
            }
            _ => {
                chars.next();
                col += 1;
            }
        }
    }
    tokens.push(Token {
        kind: TokenKind::Eof,
        line,
        col,
    });
    Ok(tokens)
}

/// Parse a sequence of tokens into a SourceFile AST.
///
/// This implementation uses a proper recursive-descent parser with
/// real parameter and body parsing, replacing the previous stub.
pub fn parse(tokens: &[Token]) -> Result<SourceFile, ParseError> {
    let mut parser = Parser::new(tokens);
    let mut file = SourceFile::default();

    while parser.kind() != &TokenKind::Eof {
        let is_pub = if parser.check(&TokenKind::KwPub) {
            parser.advance();
            true
        } else {
            false
        };

        if parser.check(&TokenKind::KwFn) {
            let func = parse_function_decl(&mut parser, is_pub)?;
            file.functions.push(func);
        } else {
            // Skip unknown top-level tokens
            parser.advance();
        }
    }

    Ok(file)
}

/// Parse a full function declaration including parameters, return type, and body.
fn parse_function_decl(parser: &mut Parser, is_pub: bool) -> Result<FnDecl, ParseError> {
    parser.expect(&TokenKind::KwFn, "'fn'")?;

    let name = match parser.kind() {
        TokenKind::Ident(n) => {
            let n = n.clone();
            parser.advance();
            n
        }
        other => {
            return Err(ParseError::UnexpectedToken {
                expected: "function name".into(),
                got: format!("{other:?}"),
                line: parser.current().line,
            });
        }
    };

    // Parse parameter list: (param1: Type1, param2: Type2, ...)
    parser.expect(&TokenKind::LParen, "'('")?;
    let mut params = Vec::new();
    if !parser.check(&TokenKind::RParen) {
        loop {
            let param_name = match parser.kind() {
                TokenKind::Ident(n) => {
                    let n = n.clone();
                    parser.advance();
                    n
                }
                other => {
                    return Err(ParseError::UnexpectedToken {
                        expected: "parameter name".into(),
                        got: format!("{other:?}"),
                        line: parser.current().line,
                    });
                }
            };

            parser.expect(&TokenKind::Colon, "':'")?;

            let param_type = match TypeExpr::from_token(parser.kind()) {
                Some(ty) => {
                    parser.advance();
                    ty
                }
                None => {
                    // Consume the token anyway to avoid infinite loops
                    parser.advance();
                    TypeExpr::Unit
                }
            };

            params.push(Param {
                name: param_name,
                ty: param_type,
            });

            if parser.check(&TokenKind::Comma) {
                parser.advance();
                if parser.check(&TokenKind::RParen) {
                    break;
                }
            } else {
                break;
            }
        }
    }
    parser.expect(&TokenKind::RParen, "')'")?;

    // Parse optional return type: -> TypeExpr
    let return_ty = if parser.check(&TokenKind::Arrow) {
        parser.advance();
        match TypeExpr::from_token(parser.kind()) {
            Some(ty) => {
                parser.advance();
                ty
            }
            None => {
                // If unrecognized type token, try to consume ident as Named type
                match parser.kind() {
                    TokenKind::Ident(s) => {
                        let type_name = s.clone();
                        parser.advance();
                        TypeExpr::Named(type_name)
                    }
                    _ => TypeExpr::Unit,
                }
            }
        }
    } else {
        TypeExpr::Unit
    };

    // Parse body: { stmt1; stmt2; ... }
    parser.expect(&TokenKind::LBrace, "'{{'")?;
    let mut body = Vec::new();
    while !parser.check(&TokenKind::RBrace) && !parser.check(&TokenKind::Eof) {
        let stmt = parse_statement(parser)?;
        body.push(stmt);
    }
    parser.expect(&TokenKind::RBrace, "'}}'")?;

    Ok(FnDecl {
        is_pub,
        name,
        params,
        return_ty,
        body,
    })
}

/// Parse a single statement.
fn parse_statement(parser: &mut Parser) -> Result<Stmt, ParseError> {
    match parser.kind() {
        TokenKind::KwLet => parse_let_statement(parser),
        TokenKind::KwReturn => parse_return_statement(parser),
        TokenKind::KwWhile => parse_while_statement(parser),
        _ => {
            let expr = parse_expression(parser)?;
            // Optional semicolon
            if parser.check(&TokenKind::Semicolon) {
                parser.advance();
            }
            Ok(Stmt::Expr(expr))
        }
    }
}

/// Parse a `let` statement.
fn parse_let_statement(parser: &mut Parser) -> Result<Stmt, ParseError> {
    parser.expect(&TokenKind::KwLet, "'let'")?;

    let name = match parser.kind() {
        TokenKind::Ident(n) => {
            let n = n.clone();
            parser.advance();
            n
        }
        other => {
            return Err(ParseError::UnexpectedToken {
                expected: "variable name".into(),
                got: format!("{other:?}"),
                line: parser.current().line,
            });
        }
    };

    // Optional type annotation: : TypeExpr
    let ty = if parser.check(&TokenKind::Colon) {
        parser.advance();
        match TypeExpr::from_token(parser.kind()) {
            Some(t) => {
                parser.advance();
                Some(t)
            }
            None => None,
        }
    } else {
        None
    };

    parser.expect(&TokenKind::Eq, "'='")?;
    let value = parse_expression(parser)?;

    if parser.check(&TokenKind::Semicolon) {
        parser.advance();
    }

    Ok(Stmt::Let { name, ty, value })
}

/// Parse a `return` statement.
fn parse_return_statement(parser: &mut Parser) -> Result<Stmt, ParseError> {
    parser.expect(&TokenKind::KwReturn, "'return'")?;

    let value = if !parser.check(&TokenKind::RBrace)
        && !parser.check(&TokenKind::Semicolon)
        && !parser.check(&TokenKind::Eof)
    {
        let expr = parse_expression(parser)?;
        if parser.check(&TokenKind::Semicolon) {
            parser.advance();
        }
        Some(expr)
    } else {
        if parser.check(&TokenKind::Semicolon) {
            parser.advance();
        }
        None
    };

    Ok(Stmt::Return(value))
}

/// Parse a `while` statement.
fn parse_while_statement(parser: &mut Parser) -> Result<Stmt, ParseError> {
    parser.expect(&TokenKind::KwWhile, "'while'")?;
    let cond = parse_expression(parser)?;
    let body = Box::new(parse_expression(parser)?);
    Ok(Stmt::While { cond, body })
}

/// Parse an expression.
fn parse_expression(parser: &mut Parser) -> Result<Expr, ParseError> {
    parse_assignment(parser)
}

/// Parse an assignment expression.
fn parse_assignment(parser: &mut Parser) -> Result<Expr, ParseError> {
    let lhs = parse_or_expression(parser)?;

    if parser.check(&TokenKind::Eq) {
        // This is an assignment — but be careful not to confuse with ==
        // Only treat single = at expression level as assignment
        if let Expr::Ident(name) = &lhs {
            parser.advance();
            let value = parse_assignment(parser)?;
            return Ok(Expr::Assign {
                name: name.clone(),
                value: Box::new(value),
            });
        }
    }

    Ok(lhs)
}

/// Parse a logical OR expression (lowest-precedence binary op).
fn parse_or_expression(parser: &mut Parser) -> Result<Expr, ParseError> {
    let mut left = parse_equality(parser)?;
    Ok(left)
}

/// Parse an equality expression (==, !=).
fn parse_equality(parser: &mut Parser) -> Result<Expr, ParseError> {
    let mut left = parse_comparison(parser)?;

    while parser.check(&TokenKind::EqEq) || parser.check(&TokenKind::BangEq) {
        let op = if parser.check(&TokenKind::EqEq) {
            parser.advance();
            BinOp::Eq
        } else {
            parser.advance();
            BinOp::Ne
        };
        let right = parse_comparison(parser)?;
        left = Expr::BinOp {
            op,
            lhs: Box::new(left),
            rhs: Box::new(right),
        };
    }

    Ok(left)
}

/// Parse a comparison expression (<, >).
fn parse_comparison(parser: &mut Parser) -> Result<Expr, ParseError> {
    let mut left = parse_term(parser)?;

    while parser.check(&TokenKind::Lt) || parser.check(&TokenKind::Gt) {
        let op = if parser.check(&TokenKind::Lt) {
            parser.advance();
            BinOp::Lt
        } else {
            parser.advance();
            BinOp::Gt
        };
        let right = parse_term(parser)?;
        left = Expr::BinOp {
            op,
            lhs: Box::new(left),
            rhs: Box::new(right),
        };
    }

    Ok(left)
}

/// Parse a term expression (+, -).
fn parse_term(parser: &mut Parser) -> Result<Expr, ParseError> {
    let mut left = parse_factor(parser)?;

    while parser.check(&TokenKind::Plus) || parser.check(&TokenKind::Minus) {
        let op = if parser.check(&TokenKind::Plus) {
            parser.advance();
            BinOp::Add
        } else {
            parser.advance();
            BinOp::Sub
        };
        let right = parse_factor(parser)?;
        left = Expr::BinOp {
            op,
            lhs: Box::new(left),
            rhs: Box::new(right),
        };
    }

    Ok(left)
}

/// Parse a factor expression (*, /).
fn parse_factor(parser: &mut Parser) -> Result<Expr, ParseError> {
    let mut left = parse_unary(parser)?;

    while parser.check(&TokenKind::Star) || parser.check(&TokenKind::Slash) {
        let op = if parser.check(&TokenKind::Star) {
            parser.advance();
            BinOp::Mul
        } else {
            parser.advance();
            BinOp::Div
        };
        let right = parse_unary(parser)?;
        left = Expr::BinOp {
            op,
            lhs: Box::new(left),
            rhs: Box::new(right),
        };
    }

    Ok(left)
}

/// Parse a unary expression (not yet implemented with negation).
fn parse_unary(parser: &mut Parser) -> Result<Expr, ParseError> {
    parse_primary(parser)
}

/// Parse a primary expression (literals, identifiers, blocks, if, calls).
fn parse_primary(parser: &mut Parser) -> Result<Expr, ParseError> {
    match parser.kind() {
        TokenKind::Int(n) => {
            let val = *n;
            parser.advance();
            Ok(Expr::IntLit(val))
        }
        TokenKind::Bool(b) => {
            let val = *b;
            parser.advance();
            Ok(Expr::BoolLit(val))
        }
        TokenKind::Str(s) => {
            let val = s.clone();
            parser.advance();
            Ok(Expr::StrLit(val))
        }
        TokenKind::Ident(_) => {
            let name = match parser.kind() {
                TokenKind::Ident(n) => n.clone(),
                _ => unreachable!(),
            };
            parser.advance();

            // Check for function call: ident(...)
            if parser.check(&TokenKind::LParen) {
                parser.advance();
                let mut args = Vec::new();
                if !parser.check(&TokenKind::RParen) {
                    loop {
                        args.push(parse_expression(parser)?);
                        if parser.check(&TokenKind::Comma) {
                            parser.advance();
                            if parser.check(&TokenKind::RParen) {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
                parser.expect(&TokenKind::RParen, "')'")?;
                Ok(Expr::Call { callee: name, args })
            } else {
                Ok(Expr::Ident(name))
            }
        }
        TokenKind::LBrace => {
            parser.advance();
            let mut stmts = Vec::new();
            while !parser.check(&TokenKind::RBrace) && !parser.check(&TokenKind::Eof) {
                stmts.push(parse_statement(parser)?);
            }
            parser.expect(&TokenKind::RBrace, "'}}'")?;
            Ok(Expr::Block(stmts))
        }
        TokenKind::KwIf => {
            parser.advance();
            let cond = Box::new(parse_expression(parser)?);
            let then_branch = Box::new(parse_expression(parser)?);
            let else_branch = if parser.check(&TokenKind::KwElse) {
                parser.advance();
                Some(Box::new(parse_expression(parser)?))
            } else {
                None
            };
            Ok(Expr::If {
                cond,
                then_branch,
                else_branch,
            })
        }
        _ => {
            // Skip unrecognized token to avoid loops
            parser.advance();
            Err(ParseError::UnexpectedToken {
                expected: "expression".into(),
                got: format!("{:?}", parser.kind()),
                line: parser.current().line,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_fn_keyword() {
        let tokens = tokenize("fn main() {}").unwrap();
        assert!(tokens.iter().any(|t| t.kind == TokenKind::KwFn));
    }

    #[test]
    fn test_tokenize_integer_literal() {
        let tokens = tokenize("42").unwrap();
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Int(42)));
    }

    #[test]
    fn test_tokenize_bool_literals() {
        let tokens = tokenize("true false").unwrap();
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Bool(true)));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Bool(false)));
    }

    #[test]
    fn test_tokenize_operators() {
        let tokens = tokenize("+ - * / == !=").unwrap();
        let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
        assert!(kinds.contains(&&TokenKind::Plus));
        assert!(kinds.contains(&&TokenKind::Minus));
        assert!(kinds.contains(&&TokenKind::EqEq));
        assert!(kinds.contains(&&TokenKind::BangEq));
    }

    #[test]
    fn test_parse_empty_fn() {
        let tokens = tokenize("fn foo() {}").unwrap();
        let file = parse(&tokens).unwrap();
        assert_eq!(file.functions.len(), 1);
        assert_eq!(file.functions[0].name, "foo");
    }

    #[test]
    fn test_parse_pub_fn() {
        let tokens = tokenize("pub fn bar() {}").unwrap();
        let file = parse(&tokens).unwrap();
        assert!(file.functions[0].is_pub);
    }

    #[test]
    fn test_parse_multiple_fns() {
        let src = "fn a() {} fn b() {} fn c() {}";
        let tokens = tokenize(src).unwrap();
        let file = parse(&tokens).unwrap();
        assert_eq!(file.functions.len(), 3);
    }

    #[test]
    fn test_tokenize_arrow() {
        let tokens = tokenize("fn f() -> i64 {}").unwrap();
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Arrow));
    }

    // ── New tests for real parameter, return type, and body parsing ─────

    #[test]
    fn test_parse_function_with_params() {
        let src = "fn add(x: i64, y: i64) -> i64 { return x + y; }";
        let tokens = tokenize(src).unwrap();
        let file = parse(&tokens).unwrap();
        assert_eq!(file.functions.len(), 1);
        let func = &file.functions[0];
        assert_eq!(func.name, "add");
        assert_eq!(func.params.len(), 2);
        assert_eq!(func.params[0].name, "x");
        assert_eq!(func.params[0].ty, TypeExpr::I64);
        assert_eq!(func.params[1].name, "y");
        assert_eq!(func.params[1].ty, TypeExpr::I64);
        assert_eq!(func.return_ty, TypeExpr::I64);
        assert!(!func.body.is_empty());
    }

    #[test]
    fn test_parse_function_with_return_type() {
        let src = "fn greet(name: str) -> str { return name; }";
        let tokens = tokenize(src).unwrap();
        let file = parse(&tokens).unwrap();
        assert_eq!(file.functions[0].return_ty, TypeExpr::Str);
    }

    #[test]
    fn test_parse_function_with_let_body() {
        let src = "fn calc() -> i64 { let x: i64 = 42; return x; }";
        let tokens = tokenize(src).unwrap();
        let file = parse(&tokens).unwrap();
        let body = &file.functions[0].body;
        assert_eq!(body.len(), 2);
        match &body[0] {
            Stmt::Let { name, ty, .. } => {
                assert_eq!(name, "x");
                assert_eq!(ty, &Some(TypeExpr::I64));
            }
            _ => panic!("expected let statement"),
        }
        match &body[1] {
            Stmt::Return(Some(Expr::Ident(name))) => assert_eq!(name, "x"),
            _ => panic!("expected return x"),
        }
    }

    #[test]
    fn test_parse_function_with_call() {
        let src = "fn main() { xvm_transfer(\"x3evm\", \"alice_evm\", 10, 50); }";
        let tokens = tokenize(src).unwrap();
        let file = parse(&tokens).unwrap();
        assert_eq!(file.functions.len(), 1);
        let body = &file.functions[0].body;
        assert_eq!(body.len(), 1);
        match &body[0] {
            Stmt::Expr(Expr::Call { callee, args }) => {
                assert_eq!(callee, "xvm_transfer");
                assert_eq!(args.len(), 4);
            }
            _ => panic!("expected call expression"),
        }
    }

    #[test]
    fn test_parse_function_with_typed_params_and_body() {
        let src = "fn transfer(amount: i64, recipient: str) -> bool { return true; }";
        let tokens = tokenize(src).unwrap();
        let file = parse(&tokens).unwrap();
        let func = &file.functions[0];
        assert_eq!(func.name, "transfer");
        assert_eq!(func.params.len(), 2);
        assert_eq!(func.params[0].name, "amount");
        assert_eq!(func.params[0].ty, TypeExpr::I64);
        assert_eq!(func.params[1].name, "recipient");
        assert_eq!(func.params[1].ty, TypeExpr::Str);
        assert_eq!(func.return_ty, TypeExpr::Bool);
        match &func.body[0] {
            Stmt::Return(Some(Expr::BoolLit(true))) => {}
            _ => panic!("expected return true"),
        }
    }

    #[test]
    fn test_parse_nested_block_expr() {
        let src = "fn main() { let x = { 42 }; return x; }";
        let tokens = tokenize(src).unwrap();
        let file = parse(&tokens).unwrap();
        // Should parse without error — block expressions are valid
        assert_eq!(file.functions.len(), 1);
    }

    #[test]
    fn test_parse_with_string_literals() {
        let src = r#"fn greet() { let msg: str = "hello"; }"#;
        let tokens = tokenize(src).unwrap();
        let file = parse(&tokens).unwrap();
        let body = &file.functions[0].body;
        match &body[0] {
            Stmt::Let { name, value, .. } => {
                assert_eq!(name, "msg");
                match value {
                    Expr::StrLit(s) => assert_eq!(s, "hello"),
                    _ => panic!("expected string literal"),
                }
            }
            _ => panic!("expected let statement"),
        }
    }

    #[test]
    fn test_parse_function_no_params_no_return() {
        let src = "fn empty() { let a = 1; let b = 2; }";
        let tokens = tokenize(src).unwrap();
        let file = parse(&tokens).unwrap();
        assert_eq!(file.functions[0].params.len(), 0);
        assert_eq!(file.functions[0].return_ty, TypeExpr::Unit);
        assert_eq!(file.functions[0].body.len(), 2);
    }

    #[test]
    fn test_tokenize_string_literal() {
        let tokens = tokenize(r#""hello world""#).unwrap();
        assert!(tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::Str(s) if s == "hello world")));
    }
}

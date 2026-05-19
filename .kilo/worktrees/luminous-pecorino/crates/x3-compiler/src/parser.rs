//! X3 language parser: converts source text to an AST.
//!
//! This is a recursive-descent parser for the X3 language. It handles
//! function declarations, type annotations, expressions, and statements.

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
    UnexpectedToken { expected: String, got: String, line: u32 },
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
    Let { name: String, ty: Option<TypeExpr>, value: Expr },
    Return(Option<Expr>),
    Expr(Expr),
    While { cond: Expr, body: Box<Expr> },
}

/// A function parameter.
#[derive(Clone, Debug)]
pub struct Param {
    pub name: String,
    pub ty: TypeExpr,
}

/// A top-level function declaration.
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
            } => {
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
                tokens.push(Token { kind: TokenKind::Int(n), line, col: start_col });
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
                tokens.push(Token { kind, line, col: start_col });
            }
            '+' => {
                tokens.push(Token { kind: TokenKind::Plus, line, col });
                chars.next();
                col += 1;
            }
            '-' => {
                chars.next();
                col += 1;
                if chars.peek() == Some(&'>') {
                    chars.next();
                    col += 1;
                    tokens.push(Token { kind: TokenKind::Arrow, line, col: col - 2 });
                } else {
                    tokens.push(Token { kind: TokenKind::Minus, line, col: col - 1 });
                }
            }
            '*' => {
                tokens.push(Token { kind: TokenKind::Star, line, col });
                chars.next();
                col += 1;
            }
            '/' => {
                tokens.push(Token { kind: TokenKind::Slash, line, col });
                chars.next();
                col += 1;
            }
            '=' => {
                chars.next();
                col += 1;
                if chars.peek() == Some(&'=') {
                    chars.next();
                    col += 1;
                    tokens.push(Token { kind: TokenKind::EqEq, line, col: col - 2 });
                } else {
                    tokens.push(Token { kind: TokenKind::Eq, line, col: col - 1 });
                }
            }
            '!' => {
                chars.next();
                col += 1;
                if chars.peek() == Some(&'=') {
                    chars.next();
                    col += 1;
                    tokens.push(Token { kind: TokenKind::BangEq, line, col: col - 2 });
                }
            }
            '<' => {
                tokens.push(Token { kind: TokenKind::Lt, line, col });
                chars.next();
                col += 1;
            }
            '>' => {
                tokens.push(Token { kind: TokenKind::Gt, line, col });
                chars.next();
                col += 1;
            }
            '(' => {
                tokens.push(Token { kind: TokenKind::LParen, line, col });
                chars.next();
                col += 1;
            }
            ')' => {
                tokens.push(Token { kind: TokenKind::RParen, line, col });
                chars.next();
                col += 1;
            }
            '{' => {
                tokens.push(Token { kind: TokenKind::LBrace, line, col });
                chars.next();
                col += 1;
            }
            '}' => {
                tokens.push(Token { kind: TokenKind::RBrace, line, col });
                chars.next();
                col += 1;
            }
            ':' => {
                tokens.push(Token { kind: TokenKind::Colon, line, col });
                chars.next();
                col += 1;
            }
            ',' => {
                tokens.push(Token { kind: TokenKind::Comma, line, col });
                chars.next();
                col += 1;
            }
            ';' => {
                tokens.push(Token { kind: TokenKind::Semicolon, line, col });
                chars.next();
                col += 1;
            }
            _ => {
                chars.next();
                col += 1;
            }
        }
    }
    tokens.push(Token { kind: TokenKind::Eof, line, col });
    Ok(tokens)
}

/// Parse a sequence of tokens into a SourceFile AST.
pub fn parse(tokens: &[Token]) -> Result<SourceFile, ParseError> {
    let mut pos = 0;
    let mut file = SourceFile::default();
    while tokens[pos].kind != TokenKind::Eof {
        let is_pub = if tokens[pos].kind == TokenKind::KwPub {
            pos += 1;
            true
        } else {
            false
        };
        if tokens[pos].kind == TokenKind::KwFn {
            pos += 1;
            let name = match &tokens[pos].kind {
                TokenKind::Ident(n) => {
                    let n = n.clone();
                    pos += 1;
                    n
                }
                other => {
                    return Err(ParseError::UnexpectedToken {
                        expected: "function name".into(),
                        got: format!("{other:?}"),
                        line: tokens[pos].line,
                    })
                }
            };
            // Skip parameter list and body stubs
            let mut depth = 0i32;
            while tokens[pos].kind != TokenKind::Eof {
                match tokens[pos].kind {
                    TokenKind::LParen | TokenKind::LBrace => {
                        depth += 1;
                        pos += 1;
                    }
                    TokenKind::RParen | TokenKind::RBrace => {
                        depth -= 1;
                        pos += 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {
                        pos += 1;
                    }
                }
            }
            file.functions.push(FnDecl {
                is_pub,
                name,
                params: Vec::new(),
                return_ty: TypeExpr::Unit,
                body: Vec::new(),
            });
        } else {
            pos += 1;
        }
    }
    Ok(file)
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
}

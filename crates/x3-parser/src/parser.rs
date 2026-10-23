use crate::cursor::Cursor;
use crate::error::{ParseError, ParseResult};
use crate::grammar::{binary_binding_power, symbol_to_binary_op, PREFIX_BP};
use crate::tokens::MAX_NESTING_DEPTH;
use x3_ast::{
    Agent, ArrayExpression, AssignExpression, AtomicBlock, Automation, BinaryExpression, Block,
    BreakStatement, CallExpression, Const, ContinueStatement, EmitStatement, Expression,
    FieldAccessExpression, FieldAssignExpression, FieldInit, ForLoopKind, ForStatement, Function,
    GlobalLet, Identifier, IfStatement, IndexAssignExpression, IndexExpression, Item, LetStatement,
    LiteralExpression, LoopStatement, Module, Param, Permission, PermissionKind, RangeExpression,
    Statement, StructDef, StructField, StructLiteralExpression, TupleExpression, TypeAnnotation,
    TypeShape, UnaryExpression, UnaryOp, WhileStatement,
};
use x3_common::{Keyword, Literal, Span, Symbol, Token, TokenKind};
use x3_lexer::Lexer;

/// Convenient entrypoint for parsing a module.
pub struct Parser {
    cursor: Cursor,
    nesting_depth: usize,
    /// False while parsing an `if`/`while` condition, where `name {` opens the body, not a struct
    /// literal (Rust's rule). Parentheses, brackets and call arguments turn it back on.
    struct_literals: bool,
}

impl Parser {
    /// Create a parser from the raw source text.
    pub fn from_source(source: &str) -> Self {
        Self {
            cursor: Cursor::new(Lexer::lex_all(source)),
            nesting_depth: 0,
            struct_literals: true,
        }
    }

    pub fn parse_module(&mut self) -> ParseResult<Module> {
        let start_span = self.peek().span;
        let mut items = Vec::new();
        while !self.peek_kind_is_eof() {
            items.push(self.parse_item()?);
        }
        let span = start_span.merge(self.peek().span);
        Ok(Module { items, span })
    }

    fn parse_item(&mut self) -> ParseResult<Item> {
        match &self.peek().kind {
            TokenKind::Keyword(Keyword::Fn) => self.parse_function_item(),
            TokenKind::Keyword(Keyword::Agent) => self.parse_agent_item(),
            TokenKind::Keyword(Keyword::Const) => self.parse_const_item(),
            TokenKind::Keyword(Keyword::Let) => self.parse_global_let(),
            TokenKind::Keyword(Keyword::Struct) => self.parse_struct_item(),
            TokenKind::Keyword(Keyword::Automation) => self.parse_automation_item(),
            TokenKind::Eof => Err(ParseError::new("unexpected end of input", self.peek().span)),
            token => Err(ParseError::new(
                format!("unexpected token {token:?} at top-level"),
                self.peek().span,
            )),
        }
    }

    fn parse_function_item(&mut self) -> ParseResult<Item> {
        let fn_token = self.expect_keyword(Keyword::Fn)?;
        let name = self.expect_identifier()?;
        let _ = self.expect_symbol(Symbol::LeftParen)?;
        let params = self.parse_parameter_list()?;
        let _ = self.expect_symbol(Symbol::RightParen)?;
        let ret_ty = if self.accept_symbol(Symbol::Arrow) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };
        let body = self.parse_block()?;
        let span = fn_token.span.merge(body.span);
        Ok(Item::Function(Function {
            name,
            params,
            ret_ty,
            body,
            span,
        }))
    }

    fn parse_global_let(&mut self) -> ParseResult<Item> {
        let let_token = self.expect_keyword(Keyword::Let)?;
        let mutable = self.accept_keyword(Keyword::Mut);
        let name = self.expect_identifier()?;
        let ty = if self.accept_symbol(Symbol::Colon) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };
        self.expect_symbol(Symbol::Equals)?;
        let initializer = self.parse_expression_bp(0)?;
        let _ = self.expect_symbol(Symbol::Semicolon)?;
        let span = let_token.span.merge(initializer.span());
        Ok(Item::GlobalLet(GlobalLet {
            name,
            mutable,
            ty,
            initializer,
            span,
        }))
    }

    fn parse_const_item(&mut self) -> ParseResult<Item> {
        self.expect_keyword(Keyword::Const)?;
        let name = self.expect_identifier()?;
        self.expect_symbol(Symbol::Colon)?;
        let ty = self.parse_type_annotation()?;
        self.expect_symbol(Symbol::Equals)?;
        let value = self.parse_expression_bp(0)?;
        self.expect_symbol(Symbol::Semicolon)?;
        Ok(Item::Const(Const {
            name,
            ty,
            value: Box::new(value),
            span: Span::new(0, 0), // Span tracking requires token position propagation
        }))
    }

    /// `struct Name { field: Type, ... }` (a trailing comma is allowed).
    fn parse_struct_item(&mut self) -> ParseResult<Item> {
        let keyword = self.expect_keyword(Keyword::Struct)?;
        let name = self.expect_identifier()?;
        self.expect_symbol(Symbol::LeftBrace)?;
        let mut fields = Vec::new();
        while !self.peek_symbol(Symbol::RightBrace) {
            let field_name = self.expect_identifier()?;
            self.expect_symbol(Symbol::Colon)?;
            let ty = self.parse_type_annotation()?;
            let span = field_name.span.merge(ty.span);
            fields.push(StructField {
                name: field_name,
                ty,
                span,
            });
            if !self.accept_symbol(Symbol::Comma) {
                break;
            }
        }
        let close = self.expect_symbol(Symbol::RightBrace)?;
        Ok(Item::Struct(StructDef {
            name,
            fields,
            span: keyword.span.merge(close.span),
        }))
    }

    /// Parse with struct literals allowed or not, restoring the previous setting.
    fn with_struct_literals<R>(
        &mut self,
        allowed: bool,
        parse: impl FnOnce(&mut Self) -> ParseResult<R>,
    ) -> ParseResult<R> {
        let previous = std::mem::replace(&mut self.struct_literals, allowed);
        let result = parse(self);
        self.struct_literals = previous;
        result
    }

    /// A condition: an expression in which `name {` opens the block that follows. Inside
    /// parentheses a struct literal is allowed again (`if (S { v: 1 }).v == 1 {`).
    fn parse_condition(&mut self) -> ParseResult<Expression> {
        self.with_struct_literals(false, |p| p.parse_expression_bp(0))
    }

    /// After a name, `{ }` or `{ field :` starts a struct literal.
    fn at_struct_literal(&self) -> bool {
        if !self.struct_literals || !self.peek_symbol(Symbol::LeftBrace) {
            return false;
        }
        match self.cursor.peek_n(1).map(|t| &t.kind) {
            Some(TokenKind::Symbol(Symbol::RightBrace)) => true,
            Some(TokenKind::Identifier(_)) => {
                self.cursor.peek_n(2).and_then(|t| t.kind.symbol()) == Some(Symbol::Colon)
            }
            _ => false,
        }
    }

    fn parse_struct_literal(&mut self, name: Identifier) -> ParseResult<Expression> {
        self.expect_symbol(Symbol::LeftBrace)?;
        let mut fields = Vec::new();
        while !self.peek_symbol(Symbol::RightBrace) {
            let field = self.expect_identifier()?;
            self.expect_symbol(Symbol::Colon)?;
            let value = self.with_struct_literals(true, |p| p.parse_expression_bp(0))?;
            let span = field.span.merge(value.span());
            fields.push(FieldInit {
                name: field,
                value,
                span,
            });
            if !self.accept_symbol(Symbol::Comma) {
                break;
            }
        }
        let close = self.expect_symbol(Symbol::RightBrace)?;
        let span = name.span.merge(close.span);
        Ok(Expression::StructLiteral(StructLiteralExpression {
            name,
            fields,
            span,
        }))
    }

    fn parse_agent_item(&mut self) -> ParseResult<Item> {
        self.expect_keyword(Keyword::Agent)?;
        let name = self.expect_identifier()?;
        self.expect_symbol(Symbol::LeftBrace)?;
        let mut items = Vec::new();
        while !self.peek_symbol(Symbol::RightBrace) {
            items.push(self.parse_item()?);
        }
        self.expect_symbol(Symbol::RightBrace)?;
        Ok(Item::Agent(Agent {
            name,
            items,
            span: Span::new(0, 0), // Span tracking requires token position propagation
        }))
    }

    /// `automation Name { trigger <expr> permissions {..} execute {..} on_failure {..} }`
    ///
    /// `trigger` is a single expression so it can name a chain-observable fact
    /// (`BlockNumber(100)`, `PriceThreshold { asset_id: 1, threshold: 1000,
    /// above: true }`); `on_failure` is optional.
    fn parse_automation_item(&mut self) -> ParseResult<Item> {
        let start = self.expect_keyword(Keyword::Automation)?.span;
        // A quoted name is allowed so a workflow can carry the human-facing
        // title it was generated from: `automation "X3 Autonomous Trading"`.
        let name = match self.peek().kind.clone() {
            TokenKind::Literal(Literal::String(text)) => {
                let token = self.cursor.bump();
                Identifier {
                    name: text,
                    span: token.span,
                }
            }
            _ => self.expect_identifier()?,
        };
        self.expect_symbol(Symbol::LeftBrace)?;

        let trigger = if self.accept_keyword(Keyword::Trigger) {
            self.expect_symbol(Symbol::LeftBrace)?;
            let expr = self.with_struct_literals(true, |p| p.parse_expression_bp(0))?;
            self.expect_symbol(Symbol::RightBrace)?;
            expr
        } else {
            return Err(ParseError::new(
                "automation requires a `trigger` section",
                self.peek().span,
            ));
        };

        let mut permissions = Vec::new();
        if self.accept_keyword(Keyword::Permissions) {
            self.expect_symbol(Symbol::LeftBrace)?;
            while !self.peek_symbol(Symbol::RightBrace) {
                permissions.push(self.parse_permission()?);
            }
            self.expect_symbol(Symbol::RightBrace)?;
        }

        if !self.accept_keyword(Keyword::Execute) {
            return Err(ParseError::new(
                "automation requires an `execute` section",
                self.peek().span,
            ));
        }
        let execute = self.parse_block()?;

        let on_failure = if self.accept_keyword(Keyword::OnFailure) {
            Some(self.parse_block()?)
        } else {
            None
        };

        let end = self.expect_symbol(Symbol::RightBrace)?.span;
        Ok(Item::Automation(Automation {
            name,
            trigger,
            permissions,
            execute,
            on_failure,
            span: start.merge(end),
        }))
    }

    /// One `kind: value` capability declaration.
    fn parse_permission(&mut self) -> ParseResult<Permission> {
        let token = self.peek().clone();
        let kind = match &token.kind {
            TokenKind::Identifier(text) => match text.as_str() {
                "wallet" => PermissionKind::Wallet,
                "max_trade" => PermissionKind::MaxTrade,
                "daily_loss_limit" => PermissionKind::DailyLossLimit,
                "allowed_chains" => PermissionKind::AllowedChains,
                other => {
                    return Err(ParseError::new(
                        format!("unknown automation permission `{other}`"),
                        token.span,
                    ))
                }
            },
            _ => return Err(ParseError::new("expected a permission name", token.span)),
        };
        self.cursor.bump();
        self.expect_symbol(Symbol::Colon)?;
        let value = self.with_struct_literals(true, |p| p.parse_expression_bp(0))?;
        let span = token.span.merge(value.span());
        // Optional separator, so `wallet: w` and `wallet: w,` both parse.
        let _ = self.accept_symbol(Symbol::Comma);
        Ok(Permission { kind, value, span })
    }

    fn parse_parameter_list(&mut self) -> ParseResult<Vec<Param>> {
        let mut params = Vec::new();
        if self.peek_symbol(Symbol::RightParen) {
            return Ok(params);
        }
        loop {
            let mutable = self.accept_keyword(Keyword::Mut);
            let name = self.expect_identifier()?;
            let ty = if self.accept_symbol(Symbol::Colon) {
                Some(self.parse_type_annotation()?)
            } else {
                None
            };
            let span = name.span;
            params.push(Param {
                name,
                mutable,
                ty,
                span,
            });
            if !self.accept_symbol(Symbol::Comma) {
                break;
            }
        }
        Ok(params)
    }

    /// Run `parse` one nesting level deeper, refusing past [`MAX_NESTING_DEPTH`].
    ///
    /// Expressions, blocks and types all recurse, and every later pass walks the tree the same
    /// way, so each of them counts against one budget: a program nested past it is refused with a
    /// parse error instead of overflowing the stack of whatever compiles it.
    fn descend<R>(&mut self, parse: impl FnOnce(&mut Self) -> ParseResult<R>) -> ParseResult<R> {
        self.nesting_depth += 1;
        if self.nesting_depth > MAX_NESTING_DEPTH {
            let span = self.peek().span;
            self.nesting_depth -= 1;
            return Err(ParseError::new(
                format!("maximum nesting depth ({MAX_NESTING_DEPTH}) exceeded"),
                span,
            ));
        }
        let result = parse(self);
        self.nesting_depth -= 1;
        result
    }

    /// A type: a name (`i64`), an array (`[T; N]`, `[T]`), or a tuple (`(T1, T2, ...)`).
    fn parse_type_annotation(&mut self) -> ParseResult<TypeAnnotation> {
        self.descend(Self::parse_type_annotation_inner)
    }

    fn parse_type_annotation_inner(&mut self) -> ParseResult<TypeAnnotation> {
        if self.peek_symbol(Symbol::LeftBracket) {
            let open = self.expect_symbol(Symbol::LeftBracket)?;
            let element = self.parse_type_annotation()?;
            let size = if self.accept_symbol(Symbol::Semicolon) {
                let token = self.bump();
                match token.kind {
                    TokenKind::Literal(Literal::Integer(n)) if n >= 0 => Some(n as usize),
                    other => {
                        return Err(ParseError::new(
                            format!(
                                "an array size must be a non-negative integer, found {other:?}"
                            ),
                            token.span,
                        ))
                    }
                }
            } else {
                None
            };
            let close = self.expect_symbol(Symbol::RightBracket)?;
            let span = open.span.merge(close.span);
            let (name, shape) = match size {
                Some(size) => (
                    "array",
                    TypeShape::Array {
                        element: Box::new(element),
                        size,
                    },
                ),
                None => (
                    "vec",
                    TypeShape::Vector {
                        element: Box::new(element),
                    },
                ),
            };
            return Ok(TypeAnnotation {
                name: Identifier {
                    name: name.to_string(),
                    span,
                },
                span,
                shape,
            });
        }
        if self.peek_symbol(Symbol::LeftParen) {
            let open = self.expect_symbol(Symbol::LeftParen)?;
            let mut elements = Vec::new();
            if !self.peek_symbol(Symbol::RightParen) {
                loop {
                    elements.push(self.parse_type_annotation()?);
                    if !self.accept_symbol(Symbol::Comma) {
                        break;
                    }
                }
            }
            let close = self.expect_symbol(Symbol::RightParen)?;
            let span = open.span.merge(close.span);
            if elements.is_empty() {
                // `()` is the unit type, as `unit` is.
                return Ok(TypeAnnotation {
                    name: Identifier {
                        name: "()".to_string(),
                        span,
                    },
                    span,
                    shape: TypeShape::Named,
                });
            }
            return Ok(TypeAnnotation {
                name: Identifier {
                    name: "tuple".to_string(),
                    span,
                },
                span,
                shape: TypeShape::Tuple { elements },
            });
        }
        let identifier = self.expect_identifier()?;
        let span = identifier.span;
        Ok(TypeAnnotation {
            name: identifier,
            span,
            shape: TypeShape::Named,
        })
    }

    /// The elements of a bracketed or parenthesised list, up to and including `close`.
    fn parse_expression_list(&mut self, close: Symbol) -> ParseResult<(Vec<Expression>, Span)> {
        self.with_struct_literals(true, |p| p.parse_expression_list_inner(close))
    }

    fn parse_expression_list_inner(
        &mut self,
        close: Symbol,
    ) -> ParseResult<(Vec<Expression>, Span)> {
        let mut elements = Vec::new();
        if !self.peek_symbol(close) {
            loop {
                elements.push(self.parse_expression_bp(0)?);
                if !self.accept_symbol(Symbol::Comma) {
                    break;
                }
                if self.peek_symbol(close) {
                    break; // trailing comma
                }
            }
        }
        let end = self.expect_symbol(close)?;
        Ok((elements, end.span))
    }

    fn parse_block(&mut self) -> ParseResult<Block> {
        self.descend(Self::parse_block_inner)
    }

    fn parse_block_inner(&mut self) -> ParseResult<Block> {
        let left = self.expect_symbol(Symbol::LeftBrace)?;
        let mut statements = Vec::new();
        while !self.accept_symbol(Symbol::RightBrace) {
            if matches!(self.peek().kind, TokenKind::Eof) {
                return Err(ParseError::new("missing closing brace", self.peek().span));
            }
            statements.push(self.parse_statement()?);
        }
        let span = left.span.merge(self.previous_span());
        Ok(Block { statements, span })
    }

    fn parse_statement(&mut self) -> ParseResult<Statement> {
        match &self.peek().kind {
            TokenKind::Keyword(Keyword::Let) => self.parse_let_statement().map(Statement::Let),
            TokenKind::Keyword(Keyword::If) => self.parse_if_statement().map(Statement::If),
            TokenKind::Keyword(Keyword::While) => {
                self.parse_while_statement().map(Statement::While)
            }
            TokenKind::Keyword(Keyword::Loop) => self.parse_loop_statement().map(Statement::Loop),
            TokenKind::Keyword(Keyword::For) => self.parse_for_statement().map(Statement::For),
            TokenKind::Keyword(Keyword::Atomic) => self.parse_atomic_block().map(Statement::Atomic),
            TokenKind::Keyword(Keyword::Emit) => self.parse_emit_statement().map(Statement::Emit),
            TokenKind::Keyword(Keyword::Break) => {
                self.parse_break_statement().map(Statement::Break)
            }
            TokenKind::Keyword(Keyword::Continue) => {
                self.parse_continue_statement().map(Statement::Continue)
            }
            TokenKind::Keyword(Keyword::Return) => self.parse_return_statement(),
            _ => {
                let expr = self.parse_expression_bp(0)?;
                self.expect_symbol(Symbol::Semicolon)?;
                Ok(Statement::Expr(expr))
            }
        }
    }

    fn parse_let_statement(&mut self) -> ParseResult<LetStatement> {
        let let_token = self.expect_keyword(Keyword::Let)?;
        let mutable = self.accept_keyword(Keyword::Mut);
        let name = self.expect_identifier()?;
        let ty = if self.accept_symbol(Symbol::Colon) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };
        self.expect_symbol(Symbol::Equals)?;
        let initializer = self.parse_expression_bp(0)?;
        self.expect_symbol(Symbol::Semicolon)?;
        let span = let_token.span.merge(initializer.span());
        Ok(LetStatement {
            name,
            mutable,
            ty,
            initializer,
            span,
        })
    }

    fn parse_if_statement(&mut self) -> ParseResult<IfStatement> {
        let if_token = self.expect_keyword(Keyword::If)?;
        let condition = self.parse_condition()?;
        let then_block = self.parse_block()?;
        let else_block = if self.accept_keyword(Keyword::Else) {
            Some(self.parse_block()?)
        } else {
            None
        };
        let span = if_token.span.merge(then_block.span);
        Ok(IfStatement {
            condition,
            then_block,
            else_block,
            span,
        })
    }

    fn parse_while_statement(&mut self) -> ParseResult<WhileStatement> {
        let while_token = self.expect_keyword(Keyword::While)?;
        let condition = self.parse_condition()?;
        let body = self.parse_block()?;
        let span = while_token.span.merge(body.span);
        Ok(WhileStatement {
            condition,
            body,
            span,
        })
    }

    fn parse_return_statement(&mut self) -> ParseResult<Statement> {
        let return_token = self.expect_keyword(Keyword::Return)?;
        let value = if self.peek_symbol(Symbol::Semicolon) {
            None
        } else {
            Some(self.parse_expression_bp(0)?)
        };
        self.expect_symbol(Symbol::Semicolon)?;
        let span = if let Some(expr) = &value {
            return_token.span.merge(expr.span())
        } else {
            return_token.span
        };
        Ok(Statement::Return(value, span))
    }

    fn parse_loop_statement(&mut self) -> ParseResult<LoopStatement> {
        let loop_token = self.expect_keyword(Keyword::Loop)?;
        let body = self.parse_block()?;
        let span = loop_token.span.merge(body.span);
        Ok(LoopStatement { body, span })
    }

    fn parse_for_statement(&mut self) -> ParseResult<ForStatement> {
        let for_token = self.expect_keyword(Keyword::For)?;
        self.expect_symbol(Symbol::LeftParen)?;

        // Check if this is a range-based for loop: for (let var in range)
        if self.peek_keyword(Keyword::Let) {
            // Look ahead to see if there's an 'in' keyword after the let statement
            let mut lookahead = self.cursor.clone();
            // Skip 'let'
            lookahead.bump();
            // Skip optional 'mut'
            if lookahead.peek_keyword(Keyword::Mut) {
                lookahead.bump();
            }
            // Skip identifier
            if let TokenKind::Identifier(_) = lookahead.peek().kind {
                lookahead.bump();
                // Skip optional type annotation
                if lookahead.peek_symbol(Symbol::Colon) {
                    lookahead.bump();
                    // Skip type (simplified - just skip until = or in)
                    while !lookahead.peek_symbol(Symbol::Equals)
                        && !lookahead.peek_keyword(Keyword::In)
                        && !lookahead.is_eof()
                    {
                        lookahead.bump();
                    }
                }
                // Check for 'in' keyword
                if lookahead.peek_keyword(Keyword::In) {
                    // This is a range-based for loop
                    let _ = self.expect_keyword(Keyword::Let);
                    let mutable = self.accept_keyword(Keyword::Mut);
                    if mutable {
                        return Err(ParseError::new(
                            "range-based for loop variables cannot be mutable",
                            self.peek().span,
                        ));
                    }
                    let variable = self.expect_identifier()?;
                    // Skip optional type annotation (not allowed in range for)
                    if self.peek_symbol(Symbol::Colon) {
                        return Err(ParseError::new(
                            "range-based for loop variables cannot have explicit types",
                            self.peek().span,
                        ));
                    }
                    self.expect_keyword(Keyword::In)?;
                    let range_expr = self.parse_expression_bp(0)?;
                    let range = match range_expr {
                        Expression::Range(r) => r,
                        _ => {
                            return Err(ParseError::new(
                                "expected range expression after 'in'",
                                range_expr.span(),
                            ))
                        }
                    };
                    self.expect_symbol(Symbol::RightParen)?;
                    let body = self.parse_block()?;
                    let span = for_token.span.merge(body.span);
                    return Ok(ForStatement {
                        kind: ForLoopKind::Range { variable, range },
                        body,
                        span,
                    });
                }
            }
        }

        // C-style for loop: for (init; cond; update)
        // Initializer: let_stmt | expr_stmt | ";"
        let init = if self.accept_symbol(Symbol::Semicolon) {
            None
        } else if self.peek_keyword(Keyword::Let) {
            // Parse let statement without semicolon for for loop initializer
            let let_token = self.expect_keyword(Keyword::Let)?;
            let mutable = self.accept_keyword(Keyword::Mut);
            let name = self.expect_identifier()?;
            let ty = if self.accept_symbol(Symbol::Colon) {
                Some(self.parse_type_annotation()?)
            } else {
                None
            };
            self.expect_symbol(Symbol::Equals)?;
            let initializer = self.parse_expression_bp(0)?;
            let span = let_token.span.merge(initializer.span());
            self.expect_symbol(Symbol::Semicolon)?;
            Some(Box::new(Statement::Let(LetStatement {
                name,
                mutable,
                ty,
                initializer,
                span,
            })))
        } else {
            let expr = self.parse_expression_bp(0)?;
            self.expect_symbol(Symbol::Semicolon)?;
            Some(Box::new(Statement::Expr(expr)))
        };

        // Condition (optional)
        let condition = if self.accept_symbol(Symbol::Semicolon) {
            None
        } else {
            let expr = self.parse_expression_bp(0)?;
            self.expect_symbol(Symbol::Semicolon)?;
            Some(expr)
        };

        // Update (optional)
        let update = if self.peek_symbol(Symbol::RightParen) {
            None
        } else {
            Some(self.parse_expression_bp(0)?)
        };
        self.expect_symbol(Symbol::RightParen)?;

        let body = self.parse_block()?;
        let span = for_token.span.merge(body.span);
        Ok(ForStatement {
            kind: ForLoopKind::CStyle {
                init,
                condition,
                update,
            },
            body,
            span,
        })
    }

    fn parse_atomic_block(&mut self) -> ParseResult<AtomicBlock> {
        let atomic_token = self.expect_keyword(Keyword::Atomic)?;
        // atomic(expr) { ... } or atomic { ... }
        let metadata = if self.accept_symbol(Symbol::LeftParen) {
            let expr = self.parse_expression_bp(0)?;
            self.expect_symbol(Symbol::RightParen)?;
            Some(expr)
        } else {
            None
        };
        let body = self.parse_block()?;
        let span = atomic_token.span.merge(body.span);
        Ok(AtomicBlock {
            metadata,
            body,
            span,
        })
    }

    fn parse_emit_statement(&mut self) -> ParseResult<EmitStatement> {
        let emit_token = self.expect_keyword(Keyword::Emit)?;
        let value = self.parse_expression_bp(0)?;
        self.expect_symbol(Symbol::Semicolon)?;
        let span = emit_token.span.merge(self.previous_span());
        Ok(EmitStatement { value, span })
    }

    fn parse_break_statement(&mut self) -> ParseResult<BreakStatement> {
        let break_token = self.expect_keyword(Keyword::Break)?;
        self.expect_symbol(Symbol::Semicolon)?;
        Ok(BreakStatement {
            span: break_token.span,
        })
    }

    fn parse_continue_statement(&mut self) -> ParseResult<ContinueStatement> {
        let continue_token = self.expect_keyword(Keyword::Continue)?;
        self.expect_symbol(Symbol::Semicolon)?;
        Ok(ContinueStatement {
            span: continue_token.span,
        })
    }

    fn parse_expression_bp(&mut self, min_bp: u8) -> ParseResult<Expression> {
        self.descend(|parser| parser.parse_expression_bp_inner(min_bp))
    }

    fn parse_expression_bp_inner(&mut self, min_bp: u8) -> ParseResult<Expression> {
        let mut lhs = self.parse_prefix_expression()?;
        while let TokenKind::Symbol(symbol) = self.peek().kind.clone() {
            if symbol == Symbol::Equals {
                return self.parse_assignment(lhs);
            }

            let bp = match binary_binding_power(symbol) {
                Some(bp) => bp,
                None => break,
            };

            if bp.left < min_bp {
                break;
            }

            let op_symbol = symbol;
            self.bump();
            let rhs = self.parse_expression_bp(bp.right)?;
            let span = lhs.span().merge(rhs.span());
            let op = symbol_to_binary_op(op_symbol).unwrap_or(x3_ast::BinaryOp::Equal);
            let binary = BinaryExpression {
                op,
                left: Box::new(lhs),
                right: Box::new(rhs),
                span,
            };
            lhs = Expression::Binary(binary);
        }
        Ok(lhs)
    }

    fn parse_assignment(&mut self, lhs: Expression) -> ParseResult<Expression> {
        let equals = self.expect_symbol(Symbol::Equals)?;
        match lhs {
            // `name[index] = value`. Only a named array can be written: arrays have value
            // semantics, so the write rebinds the variable.
            Expression::Index(IndexExpression { base, index, span }) => match *base {
                Expression::Identifier(target) => {
                    let rhs = self.parse_expression_bp(1)?;
                    let span = span.merge(rhs.span());
                    Ok(Expression::IndexAssign(IndexAssignExpression {
                        target,
                        index,
                        value: Box::new(rhs),
                        span,
                    }))
                }
                _ => Err(ParseError::new(
                    "an element can only be assigned through a named array (`a[i] = v`)",
                    equals.span,
                )),
            },
            // `name.field = value`, rebinding `name` as `a[i] = v` does.
            Expression::FieldAccess(FieldAccessExpression {
                object,
                field,
                span,
            }) => match *object {
                Expression::Identifier(target) => {
                    let rhs = self.parse_expression_bp(1)?;
                    let span = span.merge(rhs.span());
                    Ok(Expression::FieldAssign(FieldAssignExpression {
                        target,
                        field,
                        value: Box::new(rhs),
                        span,
                    }))
                }
                _ => Err(ParseError::new(
                    "a field can only be assigned through a named value (`s.f = v`)",
                    equals.span,
                )),
            },
            Expression::Identifier(ident) => {
                let rhs = self.parse_expression_bp(1)?;
                let span = ident.span.merge(rhs.span());
                Ok(Expression::Assign(AssignExpression {
                    target: ident,
                    value: Box::new(rhs),
                    span,
                }))
            }
            _ => Err(ParseError::new(
                "assignment target must be an identifier",
                equals.span,
            )),
        }
    }

    fn parse_prefix_expression(&mut self) -> ParseResult<Expression> {
        match self.peek().kind.clone() {
            TokenKind::Symbol(Symbol::Minus) => {
                let token = self.bump();
                let rhs = self.parse_expression_bp(PREFIX_BP)?;
                let span = token.span.merge(rhs.span());

                // Collapse `-<number>` into a signed literal when the operand is a
                // numeric literal expression. This makes `-42` behave as a true
                // signed numeric literal instead of a unary negation operator.
                if let Expression::Literal(lit_expr) = &rhs {
                    let lit = match &lit_expr.literal {
                        Literal::Integer(value) => Literal::Integer(-value),
                        Literal::Float(value) => Literal::Float(-value),
                        other => other.clone(),
                    };

                    if matches!(lit, Literal::Integer(_) | Literal::Float(_)) {
                        return Ok(Expression::Literal(LiteralExpression {
                            literal: lit,
                            span,
                        }));
                    }
                }

                Ok(Expression::Unary(UnaryExpression {
                    op: UnaryOp::Negate,
                    expr: Box::new(rhs),
                    span,
                }))
            }
            TokenKind::Symbol(Symbol::Bang) => {
                let token = self.bump();
                let rhs = self.parse_expression_bp(PREFIX_BP)?;
                let span = token.span.merge(rhs.span());
                Ok(Expression::Unary(UnaryExpression {
                    op: UnaryOp::Not,
                    expr: Box::new(rhs),
                    span,
                }))
            }
            TokenKind::Literal(literal) => {
                let token = self.bump();
                let lit = LiteralExpression {
                    literal: literal.clone(),
                    span: token.span,
                };
                let expr = Expression::Literal(lit);
                Ok(self.finish_postfix(expr)?)
            }
            TokenKind::Keyword(Keyword::True) => {
                let token = self.bump();
                let lit = LiteralExpression {
                    literal: Literal::Bool(true),
                    span: token.span,
                };
                Ok(self.finish_postfix(Expression::Literal(lit))?)
            }
            TokenKind::Keyword(Keyword::False) => {
                let token = self.bump();
                let lit = LiteralExpression {
                    literal: Literal::Bool(false),
                    span: token.span,
                };
                Ok(self.finish_postfix(Expression::Literal(lit))?)
            }
            TokenKind::Identifier(_) => {
                let identifier = self.expect_identifier()?;
                if self.at_struct_literal() {
                    let literal = self.parse_struct_literal(identifier)?;
                    return self.finish_postfix(literal);
                }
                Ok(self.finish_postfix(Expression::Identifier(identifier))?)
            }
            TokenKind::Symbol(Symbol::LeftParen) => {
                let open = self.expect_symbol(Symbol::LeftParen)?;
                let first = self.with_struct_literals(true, |p| p.parse_expression_bp(0))?;
                if self.accept_symbol(Symbol::Comma) {
                    // `(a, b, ...)`: a tuple. `(a)` stays a parenthesised expression.
                    let (mut rest, end) = self.parse_expression_list(Symbol::RightParen)?;
                    let mut elements = vec![first];
                    elements.append(&mut rest);
                    let tuple = Expression::Tuple(TupleExpression {
                        elements,
                        span: open.span.merge(end),
                    });
                    return self.finish_postfix(tuple);
                }
                self.expect_symbol(Symbol::RightParen)?;
                Ok(self.finish_postfix(first)?)
            }
            TokenKind::Symbol(Symbol::LeftBracket) => {
                let open = self.expect_symbol(Symbol::LeftBracket)?;
                let (elements, end) = self.parse_expression_list(Symbol::RightBracket)?;
                let array = Expression::Array(ArrayExpression {
                    elements,
                    span: open.span.merge(end),
                });
                Ok(self.finish_postfix(array)?)
            }
            other => Err(ParseError::new(
                format!("unexpected token {other:?} in expression"),
                self.peek().span,
            )),
        }
    }

    fn finish_postfix(&mut self, mut base: Expression) -> ParseResult<Expression> {
        loop {
            if self.peek_symbol(Symbol::LeftParen) {
                self.expect_symbol(Symbol::LeftParen)?;
                let mut args = Vec::new();
                if !self.peek_symbol(Symbol::RightParen) {
                    loop {
                        args.push(self.with_struct_literals(true, |p| p.parse_expression_bp(0))?);
                        if !self.accept_symbol(Symbol::Comma) {
                            break;
                        }
                    }
                }
                let right_paren = self.expect_symbol(Symbol::RightParen)?;
                let span = base.span().merge(right_paren.span);
                base = Expression::Call(CallExpression {
                    callee: Box::new(base),
                    args,
                    span,
                });
            } else if self.peek_symbol(Symbol::LeftBracket) {
                self.expect_symbol(Symbol::LeftBracket)?;
                let index = self.with_struct_literals(true, |p| p.parse_expression_bp(0))?;
                let close = self.expect_symbol(Symbol::RightBracket)?;
                let span = base.span().merge(close.span);
                base = Expression::Index(IndexExpression {
                    base: Box::new(base),
                    index: Box::new(index),
                    span,
                });
            } else if self.peek_symbol(Symbol::Dot) {
                // Check if this is a range operator (..) by looking at the next token
                if self.cursor.peek_n(1).and_then(|t| t.kind.symbol()) == Some(Symbol::Dot) {
                    // Range operator ..
                    self.expect_symbol(Symbol::Dot)?;
                    self.expect_symbol(Symbol::Dot)?;
                    let end = self.parse_expression_bp(0)?;
                    let span = base.span().merge(end.span());
                    base = Expression::Range(RangeExpression {
                        start: Box::new(base),
                        end: Box::new(end),
                        span,
                    });
                } else {
                    // Field access: `s.name`, or `t.0` for a tuple's element.
                    self.expect_symbol(Symbol::Dot)?;
                    let field = match self.peek().kind.clone() {
                        TokenKind::Literal(Literal::Integer(n)) if n >= 0 => {
                            let token = self.bump();
                            Identifier {
                                name: n.to_string(),
                                span: token.span,
                            }
                        }
                        _ => self.expect_identifier()?,
                    };
                    let span = base.span().merge(field.span);
                    base = Expression::FieldAccess(FieldAccessExpression {
                        object: Box::new(base),
                        field,
                        span,
                    });
                }
            } else {
                break;
            }
        }
        Ok(base)
    }

    fn expect_identifier(&mut self) -> ParseResult<Identifier> {
        match self.bump().kind {
            TokenKind::Identifier(name) => Ok(Identifier {
                name,
                span: self.previous_span(),
            }),
            other => Err(ParseError::new(
                format!("expected identifier but found {other:?}"),
                self.peek().span,
            )),
        }
    }

    fn peek(&self) -> &Token {
        self.cursor.peek()
    }

    fn peek_kind_is_eof(&self) -> bool {
        self.cursor.is_eof()
    }

    fn previous_span(&self) -> Span {
        self.cursor.previous_span()
    }

    fn peek_symbol(&self, symbol: Symbol) -> bool {
        self.cursor.peek_symbol(symbol)
    }

    fn accept_symbol(&mut self, symbol: Symbol) -> bool {
        self.cursor.match_symbol(symbol)
    }

    fn expect_symbol(&mut self, symbol: Symbol) -> ParseResult<Token> {
        self.cursor
            .consume_symbol(symbol)
            .ok_or_else(|| ParseError::new(format!("expected symbol {symbol:?}"), self.peek().span))
    }

    fn expect_keyword(&mut self, keyword: Keyword) -> ParseResult<Token> {
        self.cursor.consume_keyword(keyword).ok_or_else(|| {
            ParseError::new(format!("expected keyword {keyword:?}"), self.peek().span)
        })
    }

    fn accept_keyword(&mut self, keyword: Keyword) -> bool {
        self.cursor.match_keyword(keyword)
    }

    fn bump(&mut self) -> Token {
        self.cursor.bump()
    }

    fn peek_keyword(&self, keyword: Keyword) -> bool {
        self.cursor.peek_keyword(keyword)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_ast::{BinaryOp, Expression};

    #[test]
    fn parse_function_and_return() {
        let source = r#"
            fn add(x: i32, y: i32) -> i32 {
                let result = x + y;
                return result;
            }
        "#;
        let mut parser = Parser::from_source(source);
        let module = parser.parse_module().expect("should parse module");
        assert_eq!(module.items.len(), 1);
        match &module.items[0] {
            Item::Function(func) => {
                assert_eq!(func.params.len(), 2);
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parse_function_with_let_and_return() {
        let source = r#"
            fn measure() -> i32 {
                let total = 1 + 2 * 3;
                return total;
            }
        "#;
        let mut parser = Parser::from_source(source);
        let module = parser.parse_module().expect("should parse module");
        let function = match &module.items[0] {
            Item::Function(f) => f,
            _ => panic!("expected function"),
        };
        assert_eq!(function.body.statements.len(), 2);
        match &function.body.statements[0] {
            Statement::Let(stmt) => {
                assert_eq!(stmt.name.name, "total");
                match &stmt.initializer {
                    Expression::Binary(bin) => {
                        assert_eq!(bin.op, BinaryOp::Add);
                    }
                    _ => panic!("expected binary initializer"),
                }
            }
            _ => panic!("expected let statement"),
        }
        match &function.body.statements[1] {
            Statement::Return(Some(expr), _) => match expr {
                Expression::Identifier(id) => assert_eq!(id.name, "total"),
                _ => panic!("expected identifier return"),
            },
            _ => panic!("expected return statement"),
        }
    }

    #[test]
    fn parse_call_expression_with_binary_argument() {
        let source = r#"
            fn emit_log() {
                log(sum(1, 2 + 3));
            }
        "#;
        let mut parser = Parser::from_source(source);
        let module = parser.parse_module().expect("should parse module");
        let function = match &module.items[0] {
            Item::Function(f) => f,
            _ => panic!("expected function"),
        };
        match &function.body.statements[0] {
            Statement::Expr(Expression::Call(call)) => {
                match &*call.callee {
                    Expression::Identifier(id) => assert_eq!(id.name, "log"),
                    _ => panic!("expected identifier callee"),
                }
                assert_eq!(call.args.len(), 1);
                match &call.args[0] {
                    Expression::Call(inner) => match &inner.args[1] {
                        Expression::Binary(bin) => assert_eq!(bin.op, BinaryOp::Add),
                        _ => panic!("expected binary second argument"),
                    },
                    _ => panic!("expected nested call"),
                }
            }
            _ => panic!("expected expression statement"),
        }
    }

    #[test]
    fn parse_minimal_fixture() {
        let source = r#"let x = 1;
fn main() {
    return x;
}"#;
        let mut parser = Parser::from_source(source);
        let module = parser.parse_module().expect("should parse module");
        assert_eq!(module.items.len(), 2);
    }

    #[test]
    fn test_range_parsing() {
        let source = "fn test() { let x = 0..10; }";
        let mut parser = Parser::from_source(source);
        let module = parser.parse_module();
        assert!(module.is_ok(), "Failed: {:?}", module.err());
    }

    #[test]
    fn parse_negative_integer_literal_as_literal() {
        let source = "fn test() { greet(-42); }";
        let mut parser = Parser::from_source(source);
        let module = parser.parse_module().expect("should parse module");

        let function = match &module.items[0] {
            Item::Function(func) => func,
            _ => panic!("expected function item"),
        };

        let statement = match &function.body.statements[0] {
            Statement::Expr(expr) => expr,
            _ => panic!("expected expression statement"),
        };

        let call = match statement {
            Expression::Call(call_expr) => call_expr,
            _ => panic!("expected call expression"),
        };

        assert!(matches!(
            call.args.as_slice(),
            [Expression::Literal(LiteralExpression {
                literal: Literal::Integer(-42),
                ..
            })]
        ));
    }
}

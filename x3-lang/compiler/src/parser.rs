//! Token-based parser for the X3 intent language.
//!
//! Parses X3 source text into the full AST, including:
//!   - agents with context blocks and strategies
//!   - bridge / atomic_swap / strategy / proposal declarations
//!   - capability calls (`capability(args)`)
//!   - cross-chain asset operations and guards
//!   - expressions with operators, if-exprs, closures
//!
//! Architecture: hand-rolled recursive descent with explicit cursor
//! (avoids pulling in the lexer crate until it's ready — the lexer
//!  currently returns placeholder tokens; we tokenize inline here).

use x3_lang_ast::ast::*;
use x3_lang_common::{IntBase, Span, Spanned, Symbol, X3Error};

// ---------------------------------------------------------------------------
// Public entry
// ---------------------------------------------------------------------------

pub fn parse_source(source: &str) -> Result<Program, X3Error> {
    let tokens = tokenize(source);
    let mut p = Parser::new(&tokens);
    let items = p.parse_program()?;
    Ok(Program::new(items))
}

// ===========================================================================
// Tokenizer (inline, no dependency on x3-lexer for now)
// ===========================================================================

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Ident(String),
    Int(u128),
    String_(String),
    // Punctuation / delimiters
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Colon,
    Semicolon,
    Eq,
    Arrow,    // ->
    FatArrow, // =>
    Dot,
    At,
    // Keywords
    KwFn,
    KwLet,
    KwMut,
    KwReturn,
    KwIf,
    KwElse,
    KwWhile,
    KwFor,
    KwIn,
    KwLoop,
    KwBreak,
    KwContinue,
    KwAgent,
    KwStruct,
    KwEnum,
    KwUse,
    KwMod,
    KwImport,
    KwConst,
    KwBridge,
    KwAtomicSwap,
    KwStrategy,
    KwProposal,
    KwGpu,
    KwSimulate,
    KwScheduled,
    KwIntent,
    KwSubscription,
    KwPub,
    KwAsync,
    KwAs,
    KwTrue,
    KwFalse,
    KwRequire,
    KwOnFail,
    KwOnTimeout,
    KwLock,
    KwMint,
    KwBurn,
    KwRelease,
    KwSwap,
    KwMatch,
    KwAtomic,
    KwEmit,
    KwTry,
    KwAwait,
    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    AmpAmp,
    PipePipe,
    EqEq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    Bang,
    Eof,
}

struct Parser<'a> {
    tokens: &'a [Tok],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Tok]) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> Tok {
        self.tokens.get(self.pos).cloned().unwrap_or(Tok::Eof)
    }

    fn peek_n(&self, n: usize) -> Tok {
        self.tokens.get(self.pos + n).cloned().unwrap_or(Tok::Eof)
    }

    fn advance(&mut self) -> Tok {
        let tok = self.peek();
        if tok != Tok::Eof {
            self.pos += 1;
        }
        tok
    }

    fn expect_ident(&mut self, context: &str) -> Result<String, X3Error> {
        match self.advance() {
            Tok::Ident(name) => Ok(name),
            found => Err(parse_err(format!("{context}: expected identifier"), found)),
        }
    }

    // ------------------------------------------------------------------
    // parse_program
    // ------------------------------------------------------------------

    fn parse_program(&mut self) -> Result<Vec<Spanned<Item>>, X3Error> {
        let mut items = Vec::new();
        loop {
            match self.peek() {
                Tok::Eof => break,
                Tok::At => {
                    // Annotations are collected eagerly and attached to the
                    // next top-level item.
                    let annots = self.parse_annotations()?;
                    let item = self.parse_top_item()?;
                    let with_annots = annotate_item(item, annots);
                    items.push(Spanned::new(with_annots, Span::DUMMY));
                }
                _ => {
                    let item = self.parse_top_item()?;
                    items.push(Spanned::new(item, Span::DUMMY));
                }
            }
        }
        Ok(items)
    }

    fn parse_top_item(&mut self) -> Result<Item, X3Error> {
        match self.peek() {
            Tok::KwAsync => self.parse_function_item(),
            Tok::KwFn => self.parse_function_item(),
            Tok::KwAgent => self.parse_agent_item(),
            Tok::KwStruct => self.parse_struct_item(),
            Tok::KwEnum => self.parse_enum_item(),
            Tok::KwUse => self.parse_use_item(),
            Tok::KwMod => self.parse_mod_item(),
            Tok::KwImport => self.parse_import_item(),
            Tok::KwConst => self.parse_const_item(),
            Tok::KwBridge => self.parse_bridge_item(),
            Tok::KwAtomicSwap => self.parse_atomic_swap_item(),
            Tok::KwStrategy => self.parse_strategy_item(),
            Tok::KwProposal => self.parse_proposal_item(),
            Tok::KwGpu => self.parse_gpu_item(),
            Tok::KwSimulate => self.parse_simulate_item(),
            Tok::KwScheduled => self.parse_scheduled_item(),
            Tok::KwIntent => self.parse_intent_item(),
            Tok::KwSubscription => self.parse_subscription_item(),
            _ => Err(parse_err("expected top-level item".into(), self.peek())),
        }
    }

    // ------------------------------------------------------------------
    // Items
    // ------------------------------------------------------------------

    fn parse_function_item(&mut self) -> Result<Item, X3Error> {
        let is_async = if self.peek() == Tok::KwAsync {
            self.advance();
            self.expect(Tok::KwFn, "async function: expected fn")?;
            true
        } else {
            self.advance(); // 'fn'
            false
        };
        let name = self.expect_ident("function name")?;
        let generics = self.parse_optional_generics()?;
        let params = self.parse_param_list()?;
        let ret = self.parse_optional_ret_type()?;
        let body = self.parse_block()?;
        Ok(Item::Function(Function {
            name: Symbol::new(&name),
            id: None,
            params,
            ret,
            generics,
            body,
            visibility: Visibility::Pub,
            is_async,
            annotations: vec![],
        }))
    }

    fn parse_agent_item(&mut self) -> Result<Item, X3Error> {
        self.advance(); // 'agent'
        let name = self.expect_ident("agent name")?;
        let mut context = None;
        let mut state = Vec::new();
        let mut methods: Vec<Spanned<Function>> = Vec::new();
        let mut strategies: Vec<Spanned<StrategyDecl>> = Vec::new();
        let annotations = Vec::new();

        // Optional context block { k: v, ... }
        if self.peek() == Tok::LBrace {
            self.advance();
            let mut entries = Vec::new();
            while self.peek() != Tok::RBrace && self.peek() != Tok::Eof {
                let key = self.expect_ident("context key")?;
                self.expect(Tok::Colon, "':' after context key")?;
                let val = self.parse_primary_expr()?;
                entries.push((Symbol::new(&key), val));
                if self.peek() == Tok::Comma {
                    self.advance();
                }
            }
            self.expect(Tok::RBrace, "expected '}' after context block")?;
            context = Some(ContextBlock { entries });
        }

        // State block { field: type, ... }
        if self.peek() == Tok::LBrace {
            self.advance();
            while self.peek() != Tok::RBrace && self.peek() != Tok::Eof {
                let field_name = self.expect_ident("field name")?;
                self.expect(Tok::Colon, "':' after field")?;
                let ty = self.parse_type()?;
                state.push(StructField {
                    name: Symbol::new(&field_name),
                    ty,
                    visibility: Visibility::Pub,
                });
                if self.peek() == Tok::Comma {
                    self.advance();
                }
            }
            self.expect(Tok::RBrace, "expected '}' after state")?;
        }

        // Main body { methods & strategies }
        if self.peek() == Tok::LBrace {
            self.advance();
            while self.peek() != Tok::RBrace && self.peek() != Tok::Eof {
                let item_annots = self.parse_annotations()?;
                match self.peek() {
                    Tok::KwFn => {
                        let func = self.parse_fn_into_struct()?;
                        methods.push(Spanned::new(func, Span::DUMMY));
                    }
                    Tok::KwStrategy => {
                        let s = self.parse_strategy_decl()?;
                        strategies.push(Spanned::new(s, Span::DUMMY));
                    }
                    Tok::At => {
                        // annotations on the next fn/strategy
                        let inner = self.parse_annotations()?;
                        let mut all_annots = item_annots;
                        all_annots.extend(inner);
                        match self.peek() {
                            Tok::KwFn => {
                                let mut func = self.parse_fn_into_struct()?;
                                func.annotations = all_annots;
                                methods.push(Spanned::new(func, Span::DUMMY));
                            }
                            Tok::KwStrategy => {
                                let s = self.parse_strategy_decl()?;
                                strategies.push(Spanned::new(s, Span::DUMMY));
                            }
                            _ => {
                                return Err(parse_err(
                                    "expected fn or strategy after annotations".into(),
                                    self.peek(),
                                ));
                            }
                        }
                    }
                    _ => {
                        return Err(parse_err(
                            "expected fn or strategy in agent body".into(),
                            self.peek(),
                        ));
                    }
                }
            }
            self.expect(Tok::RBrace, "expected '}' after agent body")?;
        }

        Ok(Item::Agent(Agent {
            name: Symbol::new(&name),
            id: None,
            context,
            state,
            methods,
            strategies,
            visibility: Visibility::Pub,
            annotations,
        }))
    }

    fn parse_struct_item(&mut self) -> Result<Item, X3Error> {
        self.advance(); // 'struct'
        let name = self.expect_ident("struct name")?;
        let generics = self.parse_optional_generics()?;
        self.expect(Tok::LBrace, "expected '{' for struct body")?;
        let mut fields = Vec::new();
        while self.peek() != Tok::RBrace && self.peek() != Tok::Eof {
            let field_name = self.expect_ident("field name")?;
            self.expect(Tok::Colon, "':' after field")?;
            let ty = self.parse_type()?;
            fields.push(StructField {
                name: Symbol::new(&field_name),
                ty,
                visibility: Visibility::Pub,
            });
            if self.peek() == Tok::Comma {
                self.advance();
            }
        }
        self.expect(Tok::RBrace, "expected '}' after struct body")?;
        Ok(Item::Struct(StructDecl {
            name: Symbol::new(&name),
            fields,
            generics,
            visibility: Visibility::Pub,
        }))
    }

    fn parse_enum_item(&mut self) -> Result<Item, X3Error> {
        self.advance();
        let name = self.expect_ident("enum name")?;
        self.expect(Tok::LBrace, "expected '{' for enum body")?;
        let mut variants = Vec::new();
        while self.peek() != Tok::RBrace && self.peek() != Tok::Eof {
            let vname = self.expect_ident("variant name")?;
            let payload = if self.peek() == Tok::LParen {
                self.advance();
                let ty = self.parse_type()?;
                self.expect(Tok::RParen, "expected ')' after variant payload")?;
                Some(ty)
            } else {
                None
            };
            variants.push(EnumVariant {
                name: Symbol::new(&vname),
                payload,
            });
            if self.peek() == Tok::Comma {
                self.advance();
            }
        }
        self.expect(Tok::RBrace, "expected '}'")?;
        Ok(Item::Enum(EnumDecl {
            name: Symbol::new(&name),
            variants,
        }))
    }

    fn parse_use_item(&mut self) -> Result<Item, X3Error> {
        self.advance();
        let mut path = Vec::new();
        loop {
            path.push(Symbol::new(&self.expect_ident("use path segment")?));
            if self.peek() == Tok::Colon {
                // :: separator — we treat two colons as a separator
                self.advance();
                if self.peek() == Tok::Colon {
                    self.advance();
                }
                continue;
            }
            break;
        }
        let alias = if self.peek() == Tok::KwAs {
            self.advance();
            Some(Symbol::new(&self.expect_ident("alias")?))
        } else {
            None
        };
        self.expect(Tok::Semicolon, "expected ';' after use")?;
        Ok(Item::Use(UseDecl { path, alias }))
    }

    fn parse_mod_item(&mut self) -> Result<Item, X3Error> {
        self.advance();
        let name = self.expect_ident("module name")?;
        self.expect(Tok::LBrace, "expected '{'")?;
        let mut parser = Parser::new(self.tokens);
        parser.pos = self.pos;
        let items = parser.parse_program()?;
        self.pos = parser.pos;
        self.expect(Tok::RBrace, "expected '}'")?;
        Ok(Item::Mod(ModDecl {
            name: Symbol::new(&name),
            items,
        }))
    }

    fn parse_import_item(&mut self) -> Result<Item, X3Error> {
        self.advance();
        let mut module = Vec::new();
        loop {
            module.push(Symbol::new(&self.expect_ident("import path")?));
            if self.peek() == Tok::Colon {
                self.advance();
                if self.peek() == Tok::Colon {
                    self.advance();
                }
                continue;
            }
            break;
        }
        let as_alias = if self.peek() == Tok::KwAs {
            self.advance();
            Some(Symbol::new(&self.expect_ident("alias")?))
        } else {
            None
        };
        self.expect(Tok::Semicolon, "expected ';'")?;
        Ok(Item::Import(ImportDecl { module, as_alias }))
    }

    fn parse_const_item(&mut self) -> Result<Item, X3Error> {
        self.advance();
        let name = self.expect_ident("const name")?;
        let ty = if self.peek() == Tok::Colon {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect(Tok::Eq, "expected '=' in const declaration")?;
        let value = self.parse_expr()?;
        self.expect(Tok::Semicolon, "expected ';' after const")?;
        Ok(Item::Const(ConstDecl {
            name: Symbol::new(&name),
            ty,
            value,
        }))
    }

    fn parse_bridge_item(&mut self) -> Result<Item, X3Error> {
        self.advance();
        let name = self.expect_ident("bridge name")?;
        let from_asset = self.parse_asset_ref()?;
        self.expect_ident("to")?; // skip 'to'
        let to_asset = self.parse_asset_ref()?;
        self.expect(Tok::LBrace, "expected '{'")?;
        let mut body = Vec::new();
        let mut requires = Vec::new();
        let mut on_fail = None;
        let mut timeout = None;
        while self.peek() != Tok::RBrace && self.peek() != Tok::Eof {
            match self.peek() {
                Tok::KwRequire => {
                    requires.push(self.parse_require_guard()?);
                }
                Tok::KwOnFail => {
                    self.advance();
                    on_fail = Some(self.parse_failure_action()?);
                }
                Tok::KwOnTimeout => {
                    self.advance();
                    let dur = self.parse_expr()?;
                    let action = self.parse_failure_action()?;
                    timeout = Some(dur);
                    if on_fail.is_none() {
                        on_fail = Some(action);
                    }
                }
                _ => {
                    body.push(self.parse_statement()?);
                }
            }
        }
        self.expect(Tok::RBrace, "expected '}'")?;
        Ok(Item::Bridge(BridgeDecl {
            name: Symbol::new(&name),
            from_asset,
            to_asset,
            body,
            requires,
            on_fail,
            timeout,
        }))
    }

    fn parse_atomic_swap_item(&mut self) -> Result<Item, X3Error> {
        self.advance();
        let name = self.expect_ident("atomic_swap name")?;
        self.expect(Tok::LBrace, "expected '{'")?;
        let mut body = Vec::new();
        let mut on_fail = None;
        let mut timeout = None;
        while self.peek() != Tok::RBrace && self.peek() != Tok::Eof {
            match self.peek() {
                Tok::KwOnFail => {
                    self.advance();
                    on_fail = Some(self.parse_failure_action()?);
                }
                Tok::KwOnTimeout => {
                    self.advance();
                    timeout = Some(self.parse_expr()?);
                    on_fail = Some(self.parse_failure_action()?);
                }
                _ => body.push(self.parse_statement()?),
            }
        }
        self.expect(Tok::RBrace, "expected '}'")?;
        Ok(Item::AtomicSwap(AtomicSwapDecl {
            name: Symbol::new(&name),
            body,
            on_fail,
            timeout,
        }))
    }

    fn parse_strategy_item(&mut self) -> Result<Item, X3Error> {
        self.advance();
        let name = self.expect_ident("strategy name")?;
        let mut max_steps = None;
        let mut max_gas = None;
        if self.peek() == Tok::LBrace {
            // no config; proceed
        } else {
            // optional max_steps / max_gas
            while self.peek() != Tok::LBrace && self.peek() != Tok::Eof {
                match self.peek() {
                    Tok::Ident(key) if key.as_str() == "max_steps" => {
                        self.advance();
                        self.expect(Tok::Eq, "expected '='")?;
                        max_steps = Some(self.parse_expr()?);
                        if self.peek() == Tok::Comma {
                            self.advance();
                        }
                    }
                    Tok::Ident(key) if key.as_str() == "max_gas" => {
                        self.advance();
                        self.expect(Tok::Eq, "expected '='")?;
                        max_gas = Some(self.parse_expr()?);
                        if self.peek() == Tok::Comma {
                            self.advance();
                        }
                    }
                    _ => break,
                }
            }
        }
        self.expect(Tok::LBrace, "expected '{'")?;
        let mut body = Vec::new();
        let mut requires = Vec::new();
        let mut on_fail = None;
        while self.peek() != Tok::RBrace && self.peek() != Tok::Eof {
            match self.peek() {
                Tok::KwRequire => requires.push(self.parse_require_guard()?),
                Tok::KwOnFail => {
                    self.advance();
                    on_fail = Some(self.parse_failure_action()?);
                }
                _ => body.push(self.parse_statement()?),
            }
        }
        self.expect(Tok::RBrace, "expected '}'")?;
        Ok(Item::Strategy(CrossChainStrategy {
            name: Symbol::new(&name),
            max_steps,
            max_gas,
            body,
            requires,
            on_fail,
        }))
    }

    fn parse_proposal_item(&mut self) -> Result<Item, X3Error> {
        self.advance();
        let name = self.expect_ident("proposal name")?;
        let title = if self.peek() == Tok::Colon {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.expect(Tok::LBrace, "expected '{'")?;
        let mut body = Vec::new();
        let mut requires = Vec::new();
        while self.peek() != Tok::RBrace && self.peek() != Tok::Eof {
            if self.peek() == Tok::KwRequire {
                requires.push(self.parse_require_guard()?);
            } else {
                body.push(self.parse_statement()?);
            }
        }
        self.expect(Tok::RBrace, "expected '}'")?;
        Ok(Item::Proposal(ProposalDecl {
            name: Symbol::new(&name),
            title,
            body,
            requires,
        }))
    }

    fn parse_gpu_item(&mut self) -> Result<Item, X3Error> {
        self.advance();
        let is_simd = if matches!(self.peek(), Tok::Ident(ref s) if s == "simd") {
            self.advance();
            true
        } else {
            false
        };
        let body = self.parse_block()?;
        Ok(Item::GpuBlock(GpuBlock { body, is_simd }))
    }

    fn parse_simulate_item(&mut self) -> Result<Item, X3Error> {
        self.advance();
        let name = self.expect_ident("simulate name")?;
        let body = self.parse_block()?;
        let receipt = if matches!(self.peek(), Tok::Ident(ref s) if s == "receipt") {
            self.advance();
            self.expect(Tok::Colon, "expected ':'")?;
            Some(Symbol::new(&self.expect_ident("receipt name")?))
        } else {
            None
        };
        Ok(Item::SimulateDecl(SimulateDecl {
            name: Symbol::new(&name),
            body,
            receipt,
        }))
    }

    fn parse_scheduled_item(&mut self) -> Result<Item, X3Error> {
        self.advance();
        let name = self.expect_ident("scheduled task name")?;
        self.expect(Tok::Colon, "expected ':'")?;
        let period = self.parse_expr()?;
        let period_blocks = expr_to_u64(&period);
        let body = self.parse_block()?;
        Ok(Item::ScheduledTask(ScheduledTask {
            name: Symbol::new(&name),
            period_blocks,
            body,
        }))
    }

    fn parse_intent_item(&mut self) -> Result<Item, X3Error> {
        self.advance();
        let name = self.expect_ident("intent name")?;
        let mut constraints = Vec::new();
        if self.peek() == Tok::LBracket {
            self.advance();
            while self.peek() != Tok::RBracket && self.peek() != Tok::Eof {
                constraints.push(self.parse_expr()?);
                if self.peek() == Tok::Comma {
                    self.advance();
                }
            }
            self.expect(Tok::RBracket, "expected ']'")?;
        }
        let body = self.parse_block()?;
        Ok(Item::IntentDecl(IntentDecl {
            name: Symbol::new(&name),
            constraints,
            body,
        }))
    }

    fn parse_subscription_item(&mut self) -> Result<Item, X3Error> {
        self.advance();
        let name = self.expect_ident("subscription name")?;
        self.expect(Tok::Colon, "expected ':'")?;
        let amount = self.parse_expr()?;
        let amount_val = expr_to_u128(&amount)?;
        let period_blocks = if self.peek() == Tok::Comma {
            self.advance();
            let p = self.parse_expr()?;
            expr_to_u64(&p)
        } else {
            1
        };
        let body = self.parse_block()?;
        Ok(Item::SubscriptionDecl(SubscriptionDecl {
            name: Symbol::new(&name),
            amount: amount_val,
            period_blocks,
            body,
        }))
    }

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    fn parse_fn_into_struct(&mut self) -> Result<Function, X3Error> {
        self.advance(); // 'fn'
        let name = self.expect_ident("function name")?;
        let generics = self.parse_optional_generics()?;
        let params = self.parse_param_list()?;
        let ret = self.parse_optional_ret_type()?;
        let body = self.parse_block()?;
        Ok(Function {
            name: Symbol::new(&name),
            id: None,
            params,
            ret,
            generics,
            body,
            visibility: Visibility::Pub,
            is_async: false,
            annotations: vec![],
        })
    }

    fn parse_strategy_decl(&mut self) -> Result<StrategyDecl, X3Error> {
        self.advance(); // 'strategy'
        let name = self.expect_ident("strategy name")?;
        let params = self.parse_param_list()?;
        let body = self.parse_block()?;
        Ok(StrategyDecl {
            name: Symbol::new(&name),
            id: None,
            params,
            body,
            is_async: false,
        })
    }

    fn parse_annotations(&mut self) -> Result<Vec<Annotation>, X3Error> {
        let mut annots = Vec::new();
        while self.peek() == Tok::At {
            self.advance();
            annots.push(self.parse_single_annotation()?);
        }
        Ok(annots)
    }

    fn parse_single_annotation(&mut self) -> Result<Annotation, X3Error> {
        let name = self.expect_ident("annotation name")?;
        let args: Vec<Expression> = if self.peek() == Tok::LParen {
            self.advance();
            let exprs = self.parse_expr_list()?;
            self.expect(Tok::RParen, "expected ')'")?;
            exprs
        } else {
            vec![]
        };
        annotation_from_name_args(&name, &args)
    }

    fn parse_param_list(&mut self) -> Result<Vec<Parameter>, X3Error> {
        self.expect(Tok::LParen, "expected '('")?;
        let mut params = Vec::new();
        if self.peek() == Tok::RParen {
            self.advance();
            return Ok(params);
        }
        loop {
            let is_mut = if self.peek() == Tok::KwMut {
                self.advance();
                true
            } else {
                false
            };
            let name = if matches!(self.peek(), Tok::Ident(_)) {
                Some(Symbol::new(&self.expect_ident("param name")?))
            } else {
                None
            };
            let ty = if self.peek() == Tok::Colon {
                self.advance();
                Some(self.parse_type()?)
            } else {
                None
            };
            params.push(Parameter { name, ty, is_mut });
            if self.peek() == Tok::Comma {
                self.advance();
                continue;
            }
            break;
        }
        self.expect(Tok::RParen, "expected ')'")?;
        Ok(params)
    }

    fn parse_optional_generics(&mut self) -> Result<Vec<GenericParam>, X3Error> {
        if self.peek() != Tok::Lt {
            return Ok(vec![]);
        }
        self.advance();
        let mut g = Vec::new();
        loop {
            let name = Symbol::new(&self.expect_ident("generic param")?);
            let mut bounds = Vec::new();
            if self.peek() == Tok::Colon {
                self.advance();
                loop {
                    bounds.push(self.parse_type()?);
                    if self.peek() == Tok::Plus {
                        self.advance();
                        continue;
                    }
                    break;
                }
            }
            g.push(GenericParam { name, bounds });
            if self.peek() == Tok::Comma {
                self.advance();
                continue;
            }
            break;
        }
        self.expect(Tok::Gt, "expected '>'")?;
        Ok(g)
    }

    fn parse_optional_ret_type(&mut self) -> Result<Option<TypeExpr>, X3Error> {
        if self.peek() == Tok::Arrow {
            self.advance();
            Ok(Some(self.parse_type()?))
        } else {
            Ok(None)
        }
    }

    fn parse_block(&mut self) -> Result<Block, X3Error> {
        self.expect(Tok::LBrace, "expected '{'")?;
        let mut stmts = Vec::new();
        while self.peek() != Tok::RBrace && self.peek() != Tok::Eof {
            stmts.push(self.parse_statement()?);
        }
        self.expect(Tok::RBrace, "expected '}'")?;
        Ok(Block::new(stmts))
    }

    fn parse_statement(&mut self) -> Result<Statement, X3Error> {
        match self.peek() {
            Tok::KwLet => self.parse_let_stmt(),
            Tok::KwReturn => {
                self.advance();
                let expr = if self.peek() == Tok::Semicolon || self.peek() == Tok::RBrace {
                    None
                } else {
                    Some(self.parse_expr()?)
                };
                self.opt_semi();
                Ok(Statement::Return(expr))
            }
            Tok::KwBreak => {
                self.advance();
                self.opt_semi();
                Ok(Statement::Break)
            }
            Tok::KwContinue => {
                self.advance();
                self.opt_semi();
                Ok(Statement::Continue)
            }
            Tok::KwIf => {
                self.advance();
                let cond = self.parse_expr()?;
                let then_block = self.parse_block()?;
                let else_block = if self.peek() == Tok::KwElse {
                    self.advance();
                    if self.peek() == Tok::KwIf {
                        // else if -> wrap as block with single if statement
                        let inner = self.parse_statement()?;
                        Some(Block::new(vec![inner]))
                    } else {
                        Some(self.parse_block()?)
                    }
                } else {
                    None
                };
                Ok(Statement::If {
                    cond,
                    then_block,
                    else_block,
                })
            }
            Tok::KwWhile => {
                self.advance();
                let cond = self.parse_expr()?;
                let body = self.parse_block()?;
                Ok(Statement::While { cond, body })
            }
            Tok::KwFor => {
                self.advance();
                let pattern = self.parse_pattern()?;
                self.expect(Tok::KwIn, "expected 'in'")?;
                let iterable = self.parse_expr()?;
                let body = self.parse_block()?;
                Ok(Statement::For {
                    pattern,
                    iterable,
                    body,
                })
            }
            Tok::KwLoop => {
                self.advance();
                let body = self.parse_block()?;
                Ok(Statement::Loop(body))
            }
            Tok::KwAtomic => {
                self.advance();
                let meta = if self.peek() == Tok::LParen {
                    self.advance();
                    let expr = self.parse_expr()?;
                    self.expect(Tok::RParen, "expected ')'")?;
                    Some(expr)
                } else {
                    None
                };
                let body = self.parse_block()?;
                Ok(Statement::Atomic(AtomicBlock { meta, body }))
            }
            Tok::KwEmit => {
                self.advance();
                let name = self.expect_ident("event name")?;
                let mut payload = Vec::new();
                if self.peek() == Tok::LParen {
                    self.advance();
                    payload = self.parse_expr_list()?;
                    self.expect(Tok::RParen, "expected ')'")?;
                }
                self.opt_semi();
                Ok(Statement::Emit(EventEmit {
                    name: Symbol::new(&name),
                    payload,
                }))
            }
            // Cross-chain statements
            Tok::KwLock => {
                self.advance();
                let chain = self.parse_chain_ref()?;
                self.expect(Tok::Dot, "expected '.' in chain.asset")?;
                let asset_name = self.expect_ident("asset name")?;
                self.expect_ident("amount")?;
                let amount = self.parse_expr()?;
                self.expect_ident("from")?;
                let from = self.parse_expr()?;
                self.opt_semi();
                Ok(Statement::Lock {
                    asset: AssetRef::new(chain.clone(), Symbol::new(&asset_name)),
                    chain,
                    amount,
                    from,
                })
            }
            Tok::KwMint => {
                self.advance();
                let asset = self.parse_asset_ref()?;
                self.expect_ident("amount")?;
                let amount = self.parse_expr()?;
                self.expect_ident("to")?;
                let to = self.parse_expr()?;
                self.opt_semi();
                Ok(Statement::Mint { asset, amount, to })
            }
            Tok::KwBurn => {
                self.advance();
                let asset = self.parse_asset_ref()?;
                self.expect_ident("amount")?;
                let amount = self.parse_expr()?;
                self.expect_ident("from")?;
                let from = self.parse_expr()?;
                self.opt_semi();
                Ok(Statement::Burn {
                    asset,
                    amount,
                    from,
                })
            }
            Tok::KwRelease => {
                self.advance();
                let chain = self.parse_chain_ref()?;
                self.expect(Tok::Dot, "expected '.'")?;
                let asset_name = self.expect_ident("asset name")?;
                self.expect_ident("to")?;
                let to = self.parse_expr()?;
                self.opt_semi();
                Ok(Statement::Release {
                    chain: chain.clone(),
                    asset: AssetRef::new(chain, Symbol::new(&asset_name)),
                    to,
                })
            }
            Tok::KwSwap => {
                self.advance();
                let from = self.parse_asset_ref()?;
                let route: Option<Expression> = if self.peek() == Tok::Arrow {
                    // skip arrow between assets, e.g., eth.USDC -> sol.USDC
                    self.advance();
                    None
                } else {
                    None
                };
                let _route_expr = route;
                let to = self.parse_asset_ref()?;
                let dex = if matches!(self.peek(), Tok::Ident(ref s) if s == "dex") {
                    self.advance();
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                self.opt_semi();
                Ok(Statement::Swap {
                    from,
                    to,
                    route: None,
                    dex,
                })
            }
            Tok::KwRequire => Ok(self.parse_require_stmt()?),
            Tok::KwOnFail => {
                self.advance();
                let action = self.parse_failure_action()?;
                self.opt_semi();
                Ok(Statement::OnFail(action))
            }
            Tok::KwOnTimeout => {
                self.advance();
                let duration = self.parse_expr()?;
                let action = self.parse_failure_action()?;
                self.opt_semi();
                Ok(Statement::OnTimeout { duration, action })
            }
            Tok::KwMatch => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(Tok::LBrace, "expected '{'")?;
                let mut arms = Vec::new();
                while self.peek() != Tok::RBrace && self.peek() != Tok::Eof {
                    let pattern = self.parse_pattern()?;
                    self.expect(Tok::FatArrow, "expected '=>'")?;
                    let body = self.parse_expr()?;
                    arms.push((pattern, body));
                    if self.peek() == Tok::Comma {
                        self.advance();
                    }
                }
                self.expect(Tok::RBrace, "expected '}'")?;
                Ok(Statement::Expr(Expression::Match {
                    expr: Box::new(expr),
                    arms,
                }))
            }
            Tok::KwTry => {
                self.advance();
                let expr = self.parse_expr()?;
                self.opt_semi();
                Ok(Statement::Expr(Expression::Try(Box::new(expr))))
            }
            Tok::KwAwait => {
                self.advance();
                let expr = self.parse_expr()?;
                self.opt_semi();
                Ok(Statement::Expr(Expression::Await(Box::new(expr))))
            }
            Tok::KwAsync => {
                self.advance();
                let expr = self.parse_expr()?;
                self.opt_semi();
                Ok(Statement::Expr(Expression::Async(Box::new(expr))))
            }
            _ => {
                // Expression statement
                let expr = self.parse_expr()?;
                // Check for capability short-form like `snapshot`, `diff(...)`, etc.
                if let Expression::Call { callee, args } = &expr {
                    if let Expression::Ident(sym) = callee.as_ref() {
                        let name = sym.as_str();
                        let cap = capability_from_call(name, args);
                        if !matches!(cap, Statement::Expr(_)) {
                            self.opt_semi();
                            return Ok(cap);
                        }
                    }
                }
                self.opt_semi();
                Ok(Statement::Expr(expr))
            }
        }
    }

    fn parse_let_stmt(&mut self) -> Result<Statement, X3Error> {
        self.advance(); // 'let'
        let is_mut = if self.peek() == Tok::KwMut {
            self.advance();
            true
        } else {
            false
        };
        let name = self.expect_ident("variable name")?;
        let ty = if self.peek() == Tok::Colon {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };
        let expr = if self.peek() == Tok::Eq {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.expect(Tok::Semicolon, "expected ';' after let")?;
        Ok(Statement::Let {
            name: Symbol::new(&name),
            ty,
            expr,
            is_mut,
        })
    }

    // ------------------------------------------------------------------
    // Expressions (pratt-style)
    // ------------------------------------------------------------------

    fn parse_expr(&mut self) -> Result<Expression, X3Error> {
        self.parse_binary(0)
    }

    fn parse_binary(&mut self, min_prec: u8) -> Result<Expression, X3Error> {
        let mut lhs = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Tok::PipePipe => (1, BinOp::OrOr),
                Tok::AmpAmp => (2, BinOp::AndAnd),
                Tok::EqEq => (3, BinOp::EqEq),
                Tok::Ne => (3, BinOp::Ne),
                Tok::Lt => (4, BinOp::Lt),
                Tok::Gt => (4, BinOp::Gt),
                Tok::Le => (4, BinOp::Le),
                Tok::Ge => (4, BinOp::Ge),
                Tok::Plus => (5, BinOp::Plus),
                Tok::Minus => (5, BinOp::Minus),
                Tok::Star => (6, BinOp::Star),
                Tok::Slash => (6, BinOp::Slash),
                Tok::Percent => (6, BinOp::Percent),
                _ => break,
            };
            if op.0 < min_prec {
                break;
            }
            self.advance();
            let rhs = self.parse_binary(op.0 + 1)?;
            lhs = Expression::Binary {
                op: op.1,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> Result<Expression, X3Error> {
        match self.peek() {
            Tok::Minus => {
                self.advance();
                Ok(Expression::Unary {
                    op: UnOp::Neg,
                    expr: Box::new(self.parse_unary()?),
                })
            }
            Tok::Bang => {
                self.advance();
                Ok(Expression::Unary {
                    op: UnOp::Not,
                    expr: Box::new(self.parse_unary()?),
                })
            }
            _ => self.parse_primary_expr(),
        }
    }

    fn parse_primary_expr(&mut self) -> Result<Expression, X3Error> {
        let mut expr = match self.advance() {
            Tok::Int(v) => Expression::Literal(LiteralExpr::Int {
                value: v,
                base: IntBase::Decimal,
                suffix: None,
            }),
            Tok::String_(s) => Expression::Literal(LiteralExpr::String(Symbol::new(&s))),
            Tok::KwTrue => Expression::Literal(LiteralExpr::Bool(true)),
            Tok::KwFalse => Expression::Literal(LiteralExpr::Bool(false)),
            Tok::Ident(name) => Expression::Ident(Symbol::new(&name)),
            Tok::LParen => {
                let inner = self.parse_expr()?;
                self.expect(Tok::RParen, "expected ')'")?;
                inner
            }
            Tok::LBrace => {
                // Block expression
                let mut stmts = Vec::new();
                while self.peek() != Tok::RBrace && self.peek() != Tok::Eof {
                    stmts.push(self.parse_statement()?);
                }
                self.expect(Tok::RBrace, "expected '}'")?;
                Expression::BlockExpr(Block::new(stmts))
            }
            Tok::PipePipe => {
                // Closure: || body or |a, b| body
                let mut params = Vec::new();
                if self.peek() != Tok::PipePipe {
                    loop {
                        let name = self.expect_ident("closure param")?;
                        params.push(Parameter {
                            name: Some(Symbol::new(&name)),
                            ty: None,
                            is_mut: false,
                        });
                        if self.peek() == Tok::Comma {
                            self.advance();
                            continue;
                        }
                        break;
                    }
                }
                self.expect(Tok::PipePipe, "expected '|'")?;
                let body = self.parse_expr()?;
                Expression::Closure {
                    params,
                    body: Box::new(body),
                    is_async: false,
                }
            }
            found => return Err(parse_err("expected expression".into(), found)),
        };

        // Postfix: call, method, field, index
        loop {
            match self.peek() {
                Tok::LParen => {
                    self.advance();
                    let args = self.parse_expr_list()?;
                    self.expect(Tok::RParen, "expected ')'")?;
                    expr = Expression::Call {
                        callee: Box::new(expr),
                        args,
                    };
                }
                Tok::Dot => {
                    self.advance();
                    let field = self.expect_ident("field name")?;
                    // Check if followed by '(' -> method call
                    if self.peek() == Tok::LParen {
                        self.advance();
                        let args = self.parse_expr_list()?;
                        self.expect(Tok::RParen, "expected ')'")?;
                        expr = Expression::MethodCall {
                            receiver: Box::new(expr),
                            method: Symbol::new(&field),
                            args,
                        };
                    } else {
                        expr = Expression::FieldAccess {
                            target: Box::new(expr),
                            field: Symbol::new(&field),
                        };
                    }
                }
                Tok::LBracket => {
                    self.advance();
                    let index = self.parse_expr()?;
                    self.expect(Tok::RBracket, "expected ']'")?;
                    expr = Expression::Index {
                        target: Box::new(expr),
                        index: Box::new(index),
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_expr_list(&mut self) -> Result<Vec<Expression>, X3Error> {
        let mut exprs = Vec::new();
        if self.peek() == Tok::RParen || self.peek() == Tok::RBracket {
            return Ok(exprs);
        }
        loop {
            if matches!(self.peek(), Tok::Ident(_)) && self.peek_n(1) == Tok::Eq {
                self.advance();
                self.advance();
            }
            exprs.push(self.parse_expr()?);
            if self.peek() == Tok::Comma {
                self.advance();
                continue;
            }
            break;
        }
        Ok(exprs)
    }

    // ------------------------------------------------------------------
    // Types
    // ------------------------------------------------------------------

    fn parse_type(&mut self) -> Result<TypeExpr, X3Error> {
        match self.peek() {
            Tok::Ident(name) => {
                self.advance();
                let mut path = vec![Symbol::new(&name)];
                while self.peek() == Tok::Colon {
                    self.advance();
                    self.advance(); // second ':'
                    path.push(Symbol::new(&self.expect_ident("type path")?));
                }
                let base = if path.len() == 1 {
                    TypeExpr::Path(path)
                } else {
                    // multi-segment
                    TypeExpr::Path(path)
                };
                // Optional generic args
                if self.peek() == Tok::Lt {
                    self.advance();
                    let mut args = Vec::new();
                    loop {
                        args.push(self.parse_type()?);
                        if self.peek() == Tok::Comma {
                            self.advance();
                            continue;
                        }
                        break;
                    }
                    self.expect(Tok::Gt, "expected '>'")?;
                    Ok(TypeExpr::Generic {
                        base: Box::new(base),
                        args,
                    })
                } else {
                    Ok(base)
                }
            }
            Tok::LBracket => {
                self.advance();
                let inner = self.parse_type()?;
                let size = if self.peek() == Tok::Semicolon {
                    self.advance();
                    if let Tok::Int(n) = self.advance() {
                        Some(n as usize)
                    } else {
                        return Err(parse_err("expected array size".into(), self.peek()));
                    }
                } else {
                    None
                };
                self.expect(Tok::RBracket, "expected ']'")?;
                Ok(TypeExpr::Array(Box::new(inner), size))
            }
            Tok::LParen => {
                self.advance();
                let mut types = Vec::new();
                if self.peek() != Tok::RParen {
                    loop {
                        types.push(self.parse_type()?);
                        if self.peek() == Tok::Comma {
                            self.advance();
                            continue;
                        }
                        break;
                    }
                }
                self.expect(Tok::RParen, "expected ')'")?;
                Ok(TypeExpr::Tuple(types))
            }
            Tok::KwFn => {
                self.advance();
                self.expect(Tok::LParen, "expected '('")?;
                let mut params = Vec::new();
                if self.peek() != Tok::RParen {
                    loop {
                        params.push(self.parse_type()?);
                        if self.peek() == Tok::Comma {
                            self.advance();
                            continue;
                        }
                        break;
                    }
                }
                self.expect(Tok::RParen, "expected ')'")?;
                self.expect(Tok::Arrow, "expected '->' in function type")?;
                let ret = self.parse_type()?;
                Ok(TypeExpr::Func {
                    params,
                    ret: Box::new(ret),
                })
            }
            _ => Err(parse_err("expected type".into(), self.peek())),
        }
    }

    // ------------------------------------------------------------------
    // Special parsers
    // ------------------------------------------------------------------

    fn parse_asset_ref(&mut self) -> Result<AssetRef, X3Error> {
        let chain = self.parse_chain_ref()?;
        self.expect(Tok::Dot, "expected '.' in asset ref")?;
        let name = self.expect_ident("asset name")?;
        Ok(AssetRef::new(chain, Symbol::new(&name)))
    }

    fn parse_chain_ref(&mut self) -> Result<ChainRef, X3Error> {
        let name = self.expect_ident("chain name")?;
        Ok(ChainRef(Symbol::new(&name)))
    }

    fn parse_require_guard(&mut self) -> Result<RequireGuard, X3Error> {
        self.advance(); // 'require'
        let kind = self.parse_require_kind()?;
        let subject = if matches!(self.peek(), Tok::Ident(_)) {
            Some(Symbol::new(&self.expect_ident("require subject")?))
        } else {
            None
        };
        let value = self.parse_expr()?;
        self.opt_semi();
        Ok(RequireGuard {
            kind,
            subject,
            value,
        })
    }

    fn parse_require_stmt(&mut self) -> Result<Statement, X3Error> {
        let guard = self.parse_require_guard()?;
        Ok(Statement::Require(guard))
    }

    fn parse_require_kind(&mut self) -> Result<RequireKind, X3Error> {
        let name = self.expect_ident("require kind")?;
        match name.as_str() {
            "finality" => Ok(RequireKind::Finality),
            "slippage" => Ok(RequireKind::Slippage),
            "profit" => Ok(RequireKind::Profit),
            "invariant" => Ok(RequireKind::InvariantCheck),
            "risk" => Ok(RequireKind::RiskScore),
            "nonce" => Ok(RequireKind::Nonce),
            "audit_gate" => Ok(RequireKind::AuditGate),
            "bridge_liquidity" => Ok(RequireKind::BridgeLiquidity),
            "canonical_supply" => Ok(RequireKind::CanonicalSupply),
            other => Ok(RequireKind::Custom(Symbol::new(other))),
        }
    }

    fn parse_failure_action(&mut self) -> Result<FailureAction, X3Error> {
        match self.peek() {
            Tok::Ident(ref s) if s == "rollback" => {
                self.advance();
                Ok(FailureAction::Rollback)
            }
            Tok::Ident(ref s) if s == "refund" => {
                self.advance();
                let expr = self.parse_expr()?;
                Ok(FailureAction::Refund(expr))
            }
            Tok::Ident(ref s) if s == "halt" => {
                self.advance();
                Ok(FailureAction::Halt)
            }
            Tok::Ident(ref s) if s == "quarantine" => {
                self.advance();
                Ok(FailureAction::Quarantine)
            }
            _ => Err(parse_err(
                "expected rollback | refund | halt | quarantine".into(),
                self.peek(),
            )),
        }
    }

    fn parse_pattern(&mut self) -> Result<Pattern, X3Error> {
        match self.peek() {
            Tok::Ident(ref s) if s == "_" => {
                self.advance();
                Ok(Pattern::Wildcard)
            }
            Tok::Ident(name) => {
                self.advance();
                Ok(Pattern::Ident(Symbol::new(&name)))
            }
            Tok::KwTrue => Ok(Pattern::Literal(LiteralExpr::Bool(true))),
            Tok::KwFalse => Ok(Pattern::Literal(LiteralExpr::Bool(false))),
            Tok::Int(v) => Ok(Pattern::Literal(LiteralExpr::Int {
                value: v,
                base: IntBase::Decimal,
                suffix: None,
            })),
            _ => Err(parse_err("expected pattern".into(), self.peek())),
        }
    }

    fn opt_semi(&mut self) {
        if self.peek() == Tok::Semicolon {
            self.advance();
        }
    }

    fn expect(&mut self, expected: Tok, msg: &str) -> Result<(), X3Error> {
        let found = self.advance();
        if found == expected {
            Ok(())
        } else {
            Err(parse_err(msg.into(), found))
        }
    }
}

// ===========================================================================
// Helpers
// ===========================================================================

fn annotation_from_name_args(name: &str, args: &[Expression]) -> Result<Annotation, X3Error> {
    let s = |idx: usize| -> Result<String, X3Error> {
        let e = args.get(idx).ok_or_else(|| X3Error::ParseError {
            message: format!("@{name}: missing argument {idx}"),
            span: Span::DUMMY,
            expected: vec![],
            found: "".into(),
        })?;
        Ok(expr_to_string(e))
    };
    let u = |idx: usize| -> Result<u32, X3Error> {
        let e = args.get(idx).ok_or_else(|| X3Error::ParseError {
            message: format!("@{name}: missing numeric arg {idx}"),
            span: Span::DUMMY,
            expected: vec![],
            found: "".into(),
        })?;
        expr_to_u32(e)
    };
    match name {
        "no_heap" => Ok(Annotation::NoHeap),
        "no_recursion" => Ok(Annotation::NoRecursion(u(0)?)),
        "hot" => Ok(Annotation::Hot),
        "audit" => Ok(Annotation::Audit),
        "role" => Ok(Annotation::Role(Symbol::new(&s(0)?))),
        "multisig" => Ok(Annotation::Multisig(u(0)?, u(1)?)),
        "version" => Ok(Annotation::Version(Symbol::new(&s(0)?))),
        "upgrade_from" => Ok(Annotation::UpgradeFrom(Symbol::new(&s(0)?))),
        "on_chain" => Ok(Annotation::OnChain),
        "off_chain" => Ok(Annotation::OffChain),
        "sandbox" => Ok(Annotation::Sandbox),
        "whitelist" => {
            let items: Vec<Symbol> = args
                .iter()
                .map(|e| Symbol::new(&expr_to_string(e)))
                .collect();
            Ok(Annotation::Whitelist(items))
        }
        "concurrent" => Ok(Annotation::Concurrent),
        "scheduled" => {
            let period = args
                .iter()
                .find_map(|e| {
                    let s = expr_to_string(e);
                    if let Some(val) = s.strip_prefix("period=") {
                        val.parse::<u64>().ok()
                    } else {
                        None
                    }
                })
                .or_else(|| u(0).ok().map(|v| v as u64))
                .unwrap_or(1);
            Ok(Annotation::Scheduled(period))
        }
        "subscription" => {
            let amount = args
                .iter()
                .find_map(|e| {
                    let s = expr_to_string(e);
                    if let Some(val) = s.strip_prefix("amount=") {
                        val.parse::<u128>().ok()
                    } else {
                        None
                    }
                })
                .or_else(|| args.first().and_then(|e| expr_to_u128(e).ok()))
                .unwrap_or(0);
            let period = args
                .iter()
                .find_map(|e| {
                    let s = expr_to_string(e);
                    if let Some(val) = s.strip_prefix("period=") {
                        val.parse::<u64>().ok()
                    } else {
                        None
                    }
                })
                .or_else(|| {
                    args.get(1)
                        .and_then(|e| expr_to_u128(e).ok().map(|v| v as u64))
                })
                .unwrap_or(1);
            Ok(Annotation::Subscription(amount, period))
        }
        "extern" => Ok(Annotation::Extern),
        "payable" => Ok(Annotation::Payable),
        "simd" => Ok(Annotation::Simd),
        "subscribe" => Ok(Annotation::Subscribe(Symbol::new(&s(0)?))),
        "sponsor" => Ok(Annotation::Sponsor),
        "gas_adaptive" => Ok(Annotation::GasAdaptive),
        _ => Err(X3Error::ParseError {
            message: format!("unknown annotation @{name}"),
            span: Span::DUMMY,
            expected: vec![],
            found: name.into(),
        }),
    }
}

fn capability_from_call(name: &str, args: &[Expression]) -> Statement {
    let a = |i: usize| -> Expression {
        args.get(i)
            .cloned()
            .unwrap_or(Expression::Literal(LiteralExpr::Unit))
    };
    match name {
        "snapshot" => Statement::Snapshot,
        "diff" => Statement::Diff {
            before: a(0),
            after: a(1),
        },
        "crdt_get" | "get_crdt" => Statement::CrdtOp(CrdtOp {
            kind: CrdtOpKind::Get,
            key: a(0),
            value: None,
        }),
        "crdt_set" | "set_crdt" => Statement::CrdtOp(CrdtOp {
            kind: CrdtOpKind::Set,
            key: a(0),
            value: Some(a(1)),
        }),
        "crdt_append" => Statement::CrdtOp(CrdtOp {
            kind: CrdtOpKind::Append,
            key: a(0),
            value: Some(a(1)),
        }),
        "migrate_and_destroy" => Statement::Migrate { new_contract: a(0) },
        "self_destruct" => Statement::SelfDestruct,
        "verify_zk" => Statement::ZkVerify {
            proof: a(0),
            public_input: a(1),
            key: a(2),
        },
        "verify_mpc" => Statement::MpcVerify {
            result: a(0),
            signatures: a(1),
            threshold: a(2),
        },
        "storage_store" => Statement::StorageRef {
            op: StorageRefOp::Store,
            data: a(0),
        },
        "storage_load" => Statement::StorageRef {
            op: StorageRefOp::Load,
            data: a(0),
        },
        "pathfind" => Statement::Pathfind {
            from: a(0),
            to: a(1),
            max_depth: a(2),
        },
        "mempool_scan" => Statement::MempoolScan { max_results: a(0) },
        "oracle_request" => Statement::OracleRequest {
            token: a(0),
            reward: a(1),
        },
        "pause" => Statement::Pause,
        "resume" => Statement::Resume,
        _ => Statement::Expr(Expression::Call {
            callee: Box::new(Expression::Ident(Symbol::new(name))),
            args: args.to_vec(),
        }),
    }
}

fn expr_to_string(e: &Expression) -> String {
    match e {
        Expression::Literal(LiteralExpr::Int { value, .. }) => value.to_string(),
        Expression::Literal(LiteralExpr::String(s)) | Expression::Literal(LiteralExpr::Hash(s)) => {
            s.as_str().to_string()
        }
        Expression::Ident(s) => s.as_str().to_string(),
        _ => format!("{:?}", e),
    }
}

fn expr_to_u32(e: &Expression) -> Result<u32, X3Error> {
    match e {
        Expression::Literal(LiteralExpr::Int { value, .. }) => Ok(*value as u32),
        _ => Err(parse_err("expected integer".into(), Tok::Eof)),
    }
}

fn expr_to_u64(e: &Expression) -> u64 {
    match e {
        Expression::Literal(LiteralExpr::Int { value, .. }) => *value as u64,
        _ => 0,
    }
}

fn expr_to_u128(e: &Expression) -> Result<u128, X3Error> {
    match e {
        Expression::Literal(LiteralExpr::Int { value, .. }) => Ok(*value),
        _ => Err(parse_err("expected integer".into(), Tok::Eof)),
    }
}

fn annotate_item(item: Item, annotations: Vec<Annotation>) -> Item {
    match item {
        Item::Function(mut f) => {
            f.annotations = annotations;
            Item::Function(f)
        }
        Item::Agent(mut a) => {
            a.annotations = annotations;
            Item::Agent(a)
        }
        other => other,
    }
}

/// Inline tokenizer used until the lexer crate is production-ready
fn tokenize(source: &str) -> Vec<Tok> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        // Skip whitespace and comments
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2;
            continue;
        }
        // Strings
        if c == '"' {
            i += 1;
            let mut s = String::new();
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' {
                    i += 1;
                }
                s.push(chars[i]);
                i += 1;
            }
            i += 1; // closing "
            tokens.push(Tok::String_(s));
            continue;
        }
        // Numbers
        if c.is_ascii_digit() {
            let mut num = 0u128;
            while i < chars.len() && chars[i].is_ascii_digit() {
                num = num * 10 + (chars[i] as u8 - b'0') as u128;
                i += 1;
            }
            tokens.push(Tok::Int(num));
            continue;
        }
        // Identifiers and keywords
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            tokens.push(keyword_or_ident(&word));
            continue;
        }
        // Multi-char punctuation
        if c == '-' && i + 1 < chars.len() && chars[i + 1] == '>' {
            tokens.push(Tok::Arrow);
            i += 2;
            continue;
        }
        if c == '=' && i + 1 < chars.len() {
            match chars[i + 1] {
                '>' => {
                    tokens.push(Tok::FatArrow);
                    i += 2;
                    continue;
                }
                '=' => {
                    tokens.push(Tok::EqEq);
                    i += 2;
                    continue;
                }
                _ => {}
            }
        }
        if c == '!' && i + 1 < chars.len() && chars[i + 1] == '=' {
            tokens.push(Tok::Ne);
            i += 2;
            continue;
        }
        if c == '&' && i + 1 < chars.len() && chars[i + 1] == '&' {
            tokens.push(Tok::AmpAmp);
            i += 2;
            continue;
        }
        if c == '|' && i + 1 < chars.len() && chars[i + 1] == '|' {
            tokens.push(Tok::PipePipe);
            i += 2;
            continue;
        }
        if c == '<' && i + 1 < chars.len() && chars[i + 1] == '=' {
            tokens.push(Tok::Le);
            i += 2;
            continue;
        }
        if c == '>' && i + 1 < chars.len() && chars[i + 1] == '=' {
            tokens.push(Tok::Ge);
            i += 2;
            continue;
        }
        // Single-char punctuation
        let tok = match c {
            '(' => Tok::LParen,
            ')' => Tok::RParen,
            '{' => Tok::LBrace,
            '}' => Tok::RBrace,
            '[' => Tok::LBracket,
            ']' => Tok::RBracket,
            ',' => Tok::Comma,
            ':' => Tok::Colon,
            ';' => Tok::Semicolon,
            '=' => Tok::Eq,
            '.' => Tok::Dot,
            '@' => Tok::At,
            '+' => Tok::Plus,
            '-' => Tok::Minus,
            '*' => Tok::Star,
            '/' => Tok::Slash,
            '%' => Tok::Percent,
            '<' => Tok::Lt,
            '>' => Tok::Gt,
            '!' => Tok::Bang,
            _ => {
                i += 1;
                continue; // skip unrecognized
            }
        };
        tokens.push(tok);
        i += 1;
    }
    tokens
}

fn keyword_or_ident(word: &str) -> Tok {
    match word {
        "fn" => Tok::KwFn,
        "let" => Tok::KwLet,
        "mut" => Tok::KwMut,
        "return" => Tok::KwReturn,
        "if" => Tok::KwIf,
        "else" => Tok::KwElse,
        "while" => Tok::KwWhile,
        "for" => Tok::KwFor,
        "in" => Tok::KwIn,
        "loop" => Tok::KwLoop,
        "break" => Tok::KwBreak,
        "continue" => Tok::KwContinue,
        "agent" => Tok::KwAgent,
        "struct" => Tok::KwStruct,
        "enum" => Tok::KwEnum,
        "use" => Tok::KwUse,
        "mod" => Tok::KwMod,
        "import" => Tok::KwImport,
        "const" => Tok::KwConst,
        "bridge" => Tok::KwBridge,
        "atomic_swap" => Tok::KwAtomicSwap,
        "strategy" => Tok::KwStrategy,
        "proposal" => Tok::KwProposal,
        "gpu" => Tok::KwGpu,
        "simulate" => Tok::KwSimulate,
        "scheduled" => Tok::KwScheduled,
        "intent" => Tok::KwIntent,
        "subscription" => Tok::KwSubscription,
        "pub" => Tok::KwPub,
        "async" => Tok::KwAsync,
        "as" => Tok::KwAs,
        "true" => Tok::KwTrue,
        "false" => Tok::KwFalse,
        "require" => Tok::KwRequire,
        "on_fail" => Tok::KwOnFail,
        "on_timeout" => Tok::KwOnTimeout,
        "lock" => Tok::KwLock,
        "mint" => Tok::KwMint,
        "burn" => Tok::KwBurn,
        "release" => Tok::KwRelease,
        "swap" => Tok::KwSwap,
        "match" => Tok::KwMatch,
        "atomic" => Tok::KwAtomic,
        "emit" => Tok::KwEmit,
        "try" => Tok::KwTry,
        "await" => Tok::KwAwait,
        other => Tok::Ident(other.to_string()),
    }
}

fn parse_err(message: String, found: Tok) -> X3Error {
    X3Error::ParseError {
        message,
        span: Span::DUMMY,
        expected: vec![],
        found: format!("{:?}", found),
    }
}

/// Re-export the `BinOp` and `UnOp` types the parser uses so callers can
/// reference them through the compiler crate.
pub use x3_lang_common::{BinOp, UnOp};

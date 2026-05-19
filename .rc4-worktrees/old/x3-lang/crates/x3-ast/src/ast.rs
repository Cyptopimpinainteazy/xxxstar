use serde::{Deserialize, Serialize};
use std::sync::Arc;
use x3_lang_common::{
    BinOp, DurationUnit, FloatSuffix, IntBase, IntSuffix, SizeUnit, Span, Spanned, Symbol, UnOp,
};

/// Node ID - deterministic, 0-based index assigned during parsing/lowering when required.
/// Internally is a simple u32 wrapper for compactness and reproducibility.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u32);

impl Default for NodeId {
    fn default() -> Self {
        NodeId(0)
    }
}

/// The root of an X3 program AST.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    /// Top-level module items (functions, agents, types, etc.).
    pub items: Vec<Spanned<Item>>,
}

impl Program {
    pub fn new(items: Vec<Spanned<Item>>) -> Self {
        Program { items }
    }
}

/// Top-level items (declarations)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Item {
    Function(Function),
    Agent(Agent),
    Struct(StructDecl),
    Enum(EnumDecl),
    Use(UseDecl),
    Mod(ModDecl),
    Import(ImportDecl),
    Const(ConstDecl),
    // More can be added
}

/// A `use` declaration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseDecl {
    pub path: Vec<Symbol>,
    pub alias: Option<Symbol>,
}

/// An `import` declaration for FFI or runtime adapters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportDecl {
    pub module: Vec<Symbol>,
    pub as_alias: Option<Symbol>,
}

/// A `mod` declaration - for modules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModDecl {
    pub name: Symbol,
    pub items: Vec<Spanned<Item>>,
}

/// Constant declaration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstDecl {
    pub name: Symbol,
    pub ty: Option<TypeExpr>,
    pub value: Expression,
}

/// Function declaration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub name: Symbol,
    pub id: Option<NodeId>,
    pub params: Vec<Parameter>,
    pub ret: Option<TypeExpr>,
    pub generics: Vec<GenericParam>,
    pub body: Block,
    pub visibility: Visibility,
    pub is_async: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: Option<Symbol>,
    pub ty: Option<TypeExpr>,
    pub is_mut: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Visibility {
    Pub,
    Priv,
}

/// Generic parameter declaration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericParam {
    pub name: Symbol,
    pub bounds: Vec<TypeExpr>,
}

/// Struct declaration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructDecl {
    pub name: Symbol,
    pub fields: Vec<StructField>,
    pub generics: Vec<GenericParam>,
    pub visibility: Visibility,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructField {
    pub name: Symbol,
    pub ty: TypeExpr,
    pub visibility: Visibility,
}

/// Enum declaration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumDecl {
    pub name: Symbol,
    pub variants: Vec<EnumVariant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumVariant {
    pub name: Symbol,
    pub payload: Option<TypeExpr>,
}

/// Agent declaration - core X3 construct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub name: Symbol,
    pub id: Option<NodeId>,
    /// Optional context block - runtime configuration inherited by strategies
    pub context: Option<ContextBlock>,
    /// State variables for the agent
    pub state: Vec<StructField>,
    /// Methods and strategies (functions)
    pub methods: Vec<Spanned<Function>>,
    /// Strategies (named entry points)
    pub strategies: Vec<Spanned<StrategyDecl>>,
    pub visibility: Visibility,
}

/// Context block defines configuration for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBlock {
    pub entries: Vec<(Symbol, Expression)>,
}

/// Strategy declaration inside agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyDecl {
    pub name: Symbol,
    pub id: Option<NodeId>,
    pub params: Vec<Parameter>,
    pub body: Block,
    pub is_async: bool,
}

/// Block - a sequence of statements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub stmts: Vec<Statement>,
}

impl Block {
    pub fn new(stmts: Vec<Statement>) -> Self {
        Block { stmts }
    }
}

/// Statements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Statement {
    Let {
        name: Symbol,
        ty: Option<TypeExpr>,
        expr: Option<Expression>,
        is_mut: bool,
    },
    Expr(Expression),
    Return(Option<Expression>),
    Break,
    Continue,
    If {
        cond: Expression,
        then_block: Block,
        else_block: Option<Block>,
    },
    While {
        cond: Expression,
        body: Block,
    },
    For {
        pattern: Pattern,
        iterable: Expression,
        body: Block,
    },
    Loop(Block),
    Atomic(AtomicBlock),
    Emit(EventEmit),
}

/// Pattern is used in `let`, `for`, `match` (keep simple for now)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Pattern {
    Wildcard,
    Ident(Symbol),
    Tuple(Vec<Pattern>),
    Literal(LiteralExpr),
}

/// Atomic block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomicBlock {
    /// Optional 'prepare' or metadata to enforce prepare_root check
    pub meta: Option<Expression>,
    pub body: Block,
}

/// Emitted event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEmit {
    pub name: Symbol,
    pub payload: Vec<Expression>,
}

/// Types used in the AST
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeExpr {
    Path(Vec<Symbol>),
    Array(Box<TypeExpr>, Option<usize>),
    Tuple(Vec<TypeExpr>),
    Primitive(Symbol),
    Generic {
        base: Box<TypeExpr>,
        args: Vec<TypeExpr>,
    },
    Func {
        params: Vec<TypeExpr>,
        ret: Box<TypeExpr>,
    },
    Option(Box<TypeExpr>),
}

/// Expressions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Expression {
    Literal(LiteralExpr),
    Ident(Symbol),
    Binary {
        op: BinOp,
        lhs: Box<Expression>,
        rhs: Box<Expression>,
    },
    Unary {
        op: UnOp,
        expr: Box<Expression>,
    },
    Call {
        callee: Box<Expression>,
        args: Vec<Expression>,
    },
    MethodCall {
        receiver: Box<Expression>,
        method: Symbol,
        args: Vec<Expression>,
    },
    FieldAccess {
        target: Box<Expression>,
        field: Symbol,
    },
    Index {
        target: Box<Expression>,
        index: Box<Expression>,
    },
    IfExpr {
        cond: Box<Expression>,
        then_block: Box<Block>,
        else_block: Option<Box<Block>>,
    },
    BlockExpr(Block),
    Closure {
        params: Vec<Parameter>,
        body: Box<Expression>,
        is_async: bool,
    },
    Await(Box<Expression>),
    Async(Box<Expression>),
    Match {
        expr: Box<Expression>,
        arms: Vec<(Pattern, Expression)>,
    },
    Try(Box<Expression>),
    Atomic(Box<AtomicBlock>),
}

/// Literal expression (matching token Literal)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LiteralExpr {
    Int {
        value: u128,
        base: IntBase,
        suffix: Option<IntSuffix>,
    },
    Float {
        raw: Symbol,
        suffix: Option<FloatSuffix>,
    },
    String(Symbol),
    RawString {
        value: Symbol,
        hash_count: u8,
    },
    ByteString(Vec<u8>),
    Char(char),
    Byte(u8),
    Address(Symbol),
    Hash(Symbol),
    Percentage {
        value: Symbol,
    },
    Duration {
        value: u64,
        unit: DurationUnit,
    },
    Size {
        value: u64,
        unit: SizeUnit,
    },
    Bool(bool),
    Unit,
}

impl LiteralExpr {
    pub fn is_truthy(&self) -> bool {
        match self {
            LiteralExpr::Bool(b) => *b,
            LiteralExpr::Int { value, .. } => *value != 0,
            LiteralExpr::Float { raw, .. } => raw.as_str() != "0.0",
            LiteralExpr::String(s) => !s.as_str().is_empty(),
            _ => true,
        }
    }
}

// === Helpers ===

/// Walk the program AST using the provided visitor.
impl Program {
    pub fn walk(&self, v: &mut dyn crate::visitor::AstVisitor) {
        v.enter_program(self);
        for item in &self.items {
            v.enter_item(item);
            match &item.node {
                Item::Agent(a) => v.visit_agent(a),
                Item::Function(f) => v.visit_function(f),
                Item::Struct(s) => v.visit_struct(s),
                Item::Enum(e) => v.visit_enum(e),
                _ => (),
            }
            v.exit_item(item);
        }
        v.exit_program(self);
    }
}

// Keep the AST minimal, deterministic, and deterministic-friendly for serialization.

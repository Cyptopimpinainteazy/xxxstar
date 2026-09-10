# X3 Lang Trading Core v1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a vertically complete, Rust-native trading core that parses typed assets and atomic trades, proves debt repayment and net-profit guards, executes atomically, and emits tamper-evident receipts.

**Architecture:** Extend the existing X3 Rust AST and compiler pipeline instead of creating a second language path. New trading declarations lower into explicit accounting-oriented IR; a conservative control-flow verifier rejects unsafe programs before the VM executes capability-controlled venue operations and atomically commits or rolls back.

**Tech Stack:** Rust 2021, Serde, SHA-256/SHA-3 already present in the workspace, existing X3 lexer/parser/compiler/VM/CLI crates, Cargo tests, Proptest.

**Spec:** `docs/superpowers/specs/2026-09-09-x3-lang-trading-core-v1-design.md`

## Global Constraints

- The canonical implementation is the Rust workspace under `x3-lang`.
- Python mock RPC and simulator code cannot satisfy production evidence.
- V1 accepts explicit routes only; it does not discover routes.
- Production mode requires bounded slippage, a deadline, minimum net profit, and compatible execution capabilities.
- Assets with different VM family, chain, canonical identifier, or decimals are distinct types.
- Borrowed capital is linear and must be repaid exactly once on every successful path.
- Net profit includes every committed fee and execution cost.
- The verifier is conservative: inability to prove safety is rejection.
- Fixtures are visibly non-production and cannot satisfy production capability attestation.
- No live-provider or mainnet claim is made without real adapter evidence.
- Preserve the existing public syntax and tests unless a test encodes behavior explicitly rejected by the approved spec.

---

## File map

### New focused modules

- `x3-lang/crates/x3-ast/src/trading.rs`: trading declarations and typed identifiers.
- `x3-lang/compiler/src/trading_semantic.rs`: symbol resolution, typed accounting, and debt-flow analysis.
- `x3-lang/compiler/src/trading_lowering.rs`: trading AST to trading IR conversion.
- `x3-lang/compiler/src/trading_verify.rs`: control-flow and economic invariant verification.
- `x3-lang/vm/src/trading.rs`: host capabilities, atomic journal, accounting, and receipts.
- `x3-lang/examples/trading_core_v1.x3`: canonical end-to-end example.

### Existing integration points

- `x3-lang/crates/x3-ast/src/ast.rs`: export trading item variants without duplicating existing `AssetRef` or `Swap`.
- `x3-lang/crates/x3-ast/src/lib.rs`: expose the trading module.
- `x3-lang/crates/x3-lexer/src/token.rs`: trading keywords and token display.
- `x3-lang/crates/x3-lexer/src/lexer.rs`: keyword mapping.
- `x3-lang/compiler/src/parser.rs`: trading declaration and statement parsers.
- `x3-lang/compiler/src/semantic.rs`: invoke trading semantic analysis.
- `x3-lang/compiler/src/risk.rs`: consume validated trading policies.
- `x3-lang/compiler/src/ir.rs`: add typed trading operations and metadata.
- `x3-lang/compiler/src/lowering.rs`: dispatch trading items to focused lowering.
- `x3-lang/compiler/src/verify.rs`: invoke trading verifier.
- `x3-lang/compiler/src/lib.rs`: expose modules and preserve compiler entry points.
- `x3-lang/compiler/src/formatter.rs`: canonical formatting.
- `x3-lang/compiler/src/emitter.rs`: encode/decode the trading IR operations.
- `x3-lang/vm/src/executor.rs`: dispatch trading operations through the atomic journal.
- `x3-lang/vm/src/lib.rs`: expose trading types.
- `x3-lang/crates/x3-tools/src/bin/x3c.rs`: receipt inspection and production-mode errors.
- `x3-lang/spec/opcodes.yaml` and `x3-lang/spec/opcodes.rs`: synchronized opcode declarations.

## Task 1: Typed trading AST and tokens

**Files:**
- Create: `x3-lang/crates/x3-ast/src/trading.rs`
- Modify: `x3-lang/crates/x3-ast/src/ast.rs`
- Modify: `x3-lang/crates/x3-ast/src/lib.rs`
- Modify: `x3-lang/crates/x3-lexer/src/token.rs`
- Modify: `x3-lang/crates/x3-lexer/src/lexer.rs`
- Test: `x3-lang/crates/x3-ast/tests/trading_ast.rs`
- Test: `x3-lang/crates/x3-lexer/tests/trading_tokens.rs`

**Interfaces:**
- Produces: `AssetId`, `AssetDecl`, `TradeRiskPolicy`, `AtomicTradeDecl`, `TradeStmt`, `DebtId`, `RoundingMode`.
- Produces item variants: `Item::AssetDecl(AssetDecl)` and `Item::AtomicTrade(AtomicTradeDecl)`.
- Consumes: existing `VmFamily` naming conventions, `ChainRef`, `Expression`, `Symbol`, and `Spanned<Item>`.

- [ ] **Step 1: Add failing AST construction tests**

Create tests that construct these exact public values:

```rust
let asset = AssetId {
    vm_family: Symbol::from("evm"),
    chain: ChainRef::new(Symbol::from("ethereum")),
    canonical_id: Symbol::from("0xA0b8"),
    symbol: Symbol::from("USDC"),
    decimals: 6,
};
assert_eq!(asset.decimals, 6);

let trade = AtomicTradeDecl {
    name: Symbol::from("CrossDexArb"),
    risk_policy: Symbol::from("MainnetArb"),
    body: vec![
        TradeStmt::Borrow {
            amount: AmountExpr::literal(1_000_000_000_000, Symbol::from("USDC")),
            provider: Symbol::from("aave_v3"),
            debt: DebtId(Symbol::from("debt")),
        },
        TradeStmt::Repay { debt: DebtId(Symbol::from("debt")) },
        TradeStmt::RequireAllDebtsRepaid,
        TradeStmt::EmitReceipt,
    ],
};
assert_eq!(trade.body.len(), 4);
```

- [ ] **Step 2: Run the focused tests and record the expected unresolved-type failures**

Run: `cargo test -p x3-lang-ast --test trading_ast && cargo test -p x3-lang-lexer --test trading_tokens`

Expected: FAIL because the trading types and keywords do not exist.

- [ ] **Step 3: Implement the focused AST module**

Define:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssetId {
    pub vm_family: Symbol,
    pub chain: ChainRef,
    pub canonical_id: Symbol,
    pub symbol: Symbol,
    pub decimals: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DebtId(pub Symbol);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoundingMode { Down, Up, Exact }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmountExpr {
    pub value: Expression,
    pub asset: Symbol,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetDecl { pub name: Symbol, pub asset: AssetId }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRiskPolicy {
    pub name: Symbol,
    pub max_slippage_bps: u16,
    pub max_gas: AmountExpr,
    pub max_flash_fee_bps: u16,
    pub deadline: Expression,
    pub require_private_submission: bool,
    pub min_profit: Option<AmountExpr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomicTradeDecl {
    pub name: Symbol,
    pub risk_policy: Symbol,
    pub body: Vec<TradeStmt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TradeStmt {
    Borrow { amount: AmountExpr, provider: Symbol, debt: DebtId },
    Swap {
        binding: Symbol,
        input: AmountExpr,
        from_asset: Symbol,
        to_asset: Symbol,
        venue: Symbol,
        min_output: AmountExpr,
    },
    Repay { debt: DebtId },
    RequireMinNetProfit { amount: AmountExpr },
    RequireAllDebtsRepaid,
    EmitReceipt,
}
```

Use a lossless expression in `AmountExpr.value`; do not collapse source literals to `u128` before semantic decimal conversion.

- [ ] **Step 4: Add and map keyword tokens**

Add exact tokens for `asset`, `atomic`, `trade`, `using`, `borrow`, `from`, `as`, `swap`, `via`, `min_out`, `repay`, `net_profit`, `all_debts_repaid`, `receipt`, `bps`, and `private_submission`. Preserve existing `atomic swap` tokenization.

- [ ] **Step 5: Run AST and lexer suites**

Run: `cargo test -p x3-lang-ast --all-targets && cargo test -p x3-lang-lexer --all-targets`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add x3-lang/crates/x3-ast x3-lang/crates/x3-lexer
git commit -m "feat(x3-lang): add typed trading AST"
```

## Task 2: Parser and formatter

**Files:**
- Modify: `x3-lang/compiler/src/parser.rs`
- Modify: `x3-lang/compiler/src/formatter.rs`
- Create: `x3-lang/compiler/tests/test_trading_parser.rs`
- Create: `x3-lang/compiler/tests/fixtures/trading_core_v1.x3`
- Create: `x3-lang/compiler/tests/fixtures/trading_invalid_missing_min_out.x3`

**Interfaces:**
- Consumes: Task 1 trading AST types.
- Produces: `parse_asset_decl`, `parse_trade_risk_policy`, `parse_atomic_trade_decl`, and `parse_trade_stmt`.
- Produces canonical formatter output that reparses to an equivalent AST.

- [ ] **Step 1: Add failing parser tests**

Use the complete source example from the design spec. Assert one `AssetDecl` per asset, one `TradeRiskPolicy`, and one `AtomicTrade`. Add negative cases for missing `via`, missing `min_out`, duplicate debt names, and malformed basis points.

- [ ] **Step 2: Run the parser test**

Run: `cargo test -p x3-lang-compiler --test test_trading_parser -- --nocapture`

Expected: FAIL at the first `asset` declaration.

- [ ] **Step 3: Implement parser dispatch and productions**

Add top-level dispatch without changing existing `atomic swap` parsing:

```rust
TokenKind::Asset => self.parse_asset_decl().map(Item::AssetDecl),
TokenKind::Atomic if self.peek_kind(1) == TokenKind::Trade =>
    self.parse_atomic_trade_decl().map(Item::AtomicTrade),
```

Parse every mandatory field structurally. Missing venue or minimum output is a parser diagnostic, not a default.

- [ ] **Step 4: Implement canonical formatting**

Formatting must emit all semantic fields and use `min_out` consistently. Add a round-trip test:

```rust
let first = parse_source(SOURCE).unwrap();
let formatted = format_program(&first);
let second = parse_source(&formatted).unwrap();
assert_eq!(serde_json::to_value(first).unwrap(), serde_json::to_value(second).unwrap());
```

- [ ] **Step 5: Run parser, formatter, and existing atomic-swap tests**

Run: `cargo test -p x3-lang-compiler --test test_trading_parser --test test_atomic_swap_syntax`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add x3-lang/compiler/src/parser.rs x3-lang/compiler/src/formatter.rs x3-lang/compiler/tests
git commit -m "feat(x3-lang): parse and format atomic trades"
```

## Task 3: Asset registry and economic type checking

**Files:**
- Create: `x3-lang/compiler/src/trading_semantic.rs`
- Modify: `x3-lang/compiler/src/semantic.rs`
- Modify: `x3-lang/compiler/src/lib.rs`
- Create: `x3-lang/compiler/tests/test_trading_types.rs`

**Interfaces:**
- Consumes: parsed `AssetDecl`, `TradeRiskPolicy`, and `AtomicTradeDecl`.
- Produces:

```rust
pub struct TradingSymbols {
    pub assets: IndexMap<Symbol, AssetId>,
    pub policies: IndexMap<Symbol, TradeRiskPolicy>,
}

pub struct TypedAmount {
    pub base_units: u128,
    pub asset: AssetId,
}

pub fn analyze_trading(program: &Program, mode: CompilationMode)
    -> Result<TradingSymbols, Vec<Diagnostic>>;
```

- [ ] **Step 1: Add failing type tests**

Cover duplicate assets, invalid decimals above 38, wrong asset passed to a swap, output `min_out` in the input asset, profit in an unresolvable asset, literal precision loss, overflow, and explicit exact/up/down rounding.

- [ ] **Step 2: Run the type tests**

Run: `cargo test -p x3-lang-compiler --test test_trading_types -- --nocapture`

Expected: FAIL because `analyze_trading` is unavailable.

- [ ] **Step 3: Implement asset registration and literal conversion**

Implement:

```rust
pub fn decimal_to_base_units(
    literal: &str,
    decimals: u8,
    rounding: RoundingMode,
) -> Result<u128, TradingTypeError>;
```

Use checked integer arithmetic only. `Exact` rejects discarded fractional digits, `Down` truncates positive excess digits, and `Up` increments when discarded digits are nonzero.

- [ ] **Step 4: Implement typed statement checking**

Track binding asset types and debt declarations per atomic trade. Require the swap input and declared `from_asset` to agree; require `min_output.asset == to_asset`; require repayment to reference a declared debt.

- [ ] **Step 5: Integrate with existing semantic entry points**

Call `analyze_trading` from `check_source` and `check_source_with_mode`, translating failures to existing diagnostics with source spans.

- [ ] **Step 6: Run compiler semantic tests**

Run: `cargo test -p x3-lang-compiler --test test_trading_types --test test_diagnostics --test test_compiler_pipeline`

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add x3-lang/compiler/src/trading_semantic.rs x3-lang/compiler/src/semantic.rs x3-lang/compiler/src/lib.rs x3-lang/compiler/tests/test_trading_types.rs
git commit -m "feat(x3-lang): enforce trading asset types"
```

## Task 4: Risk policy and debt control-flow proof

**Files:**
- Modify: `x3-lang/compiler/src/risk.rs`
- Create: `x3-lang/compiler/src/trading_verify.rs`
- Modify: `x3-lang/compiler/src/verify.rs`
- Create: `x3-lang/compiler/tests/test_trading_verifier.rs`

**Interfaces:**
- Consumes: typed trading program and `CompilationMode`.
- Produces:

```rust
pub struct DebtFlowState {
    pub open: BTreeSet<DebtId>,
    pub closed: BTreeSet<DebtId>,
}

pub fn verify_atomic_trade(
    trade: &AtomicTradeDecl,
    symbols: &TradingSymbols,
    mode: CompilationMode,
) -> Vec<Diagnostic>;
```

- [ ] **Step 1: Write failing verifier tests**

Add one test per rejection rule: missing repayment, duplicate repayment, unknown debt, missing net-profit guard, missing all-debts guard, missing receipt, excessive policy values, absent deadline, and production policy requiring unavailable private submission.

- [ ] **Step 2: Run verifier tests**

Run: `cargo test -p x3-lang-compiler --test test_trading_verifier -- --nocapture`

Expected: FAIL because unsafe samples are accepted or the verifier does not exist.

- [ ] **Step 3: Implement debt state transitions**

`Borrow` inserts into `open`; duplicate IDs fail. `Repay` moves from `open` to `closed`; missing or already-closed IDs fail. A successful exit requires `open.is_empty()`. Structure the API for future branch-state intersection even though v1 trade syntax remains linear.

- [ ] **Step 4: Enforce policy bounds**

Require `max_slippage_bps <= 10_000`, nonzero deadline, `max_flash_fee_bps <= 10_000`, a typed gas ceiling, and a minimum-profit guard. In production mode, require the private-submission capability when the policy requests it; never downgrade the error to a warning.

- [ ] **Step 5: Wire verification into existing compiler verification**

Invoke trading verification after semantic success and before IR emission.

- [ ] **Step 6: Run verifier and risk tests**

Run: `cargo test -p x3-lang-compiler --test test_trading_verifier --test test_ir_verifier`

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add x3-lang/compiler/src/risk.rs x3-lang/compiler/src/trading_verify.rs x3-lang/compiler/src/verify.rs x3-lang/compiler/tests/test_trading_verifier.rs
git commit -m "feat(x3-lang): prove debt closure and trading risk"
```

## Task 5: Trading IR, lowering, and bytecode

**Files:**
- Modify: `x3-lang/compiler/src/ir.rs`
- Create: `x3-lang/compiler/src/trading_lowering.rs`
- Modify: `x3-lang/compiler/src/lowering.rs`
- Modify: `x3-lang/compiler/src/emitter.rs`
- Modify: `x3-lang/spec/opcodes.yaml`
- Modify: `x3-lang/spec/opcodes.rs`
- Create: `x3-lang/compiler/tests/test_trading_ir.rs`

**Interfaces:**
- Consumes: verified trading AST and `TradingSymbols`.
- Produces:

```rust
pub enum TradingOperation {
    BeginAtomicTrade { trade_id: String, policy_id: String },
    OpenDebt { debt_id: String, provider: String, asset: AssetKey, principal: u128 },
    ExecuteSwap {
        binding: String,
        venue: String,
        from: AssetKey,
        to: AssetKey,
        input: ValueRef,
        min_output: u128,
    },
    CloseDebt { debt_id: String },
    AssertMinNetProfit { settlement_asset: AssetKey, minimum: u128 },
    AssertAllDebtsClosed,
    EmitTradeReceipt,
    CommitAtomicTrade,
    AbortAtomicTrade,
}

pub fn lower_atomic_trade(
    trade: &AtomicTradeDecl,
    symbols: &TradingSymbols,
) -> Result<Vec<Operation>, LowerError>;
```

Embed `TradingOperation` in the existing `Operation` enum as `Operation::Trading(TradingOperation)`.

- [ ] **Step 1: Add failing IR sequence tests**

Assert exact operation order for the canonical example: begin, open debt, swap, swap, close debt, minimum-profit assertion, debt assertion, receipt, commit.

- [ ] **Step 2: Run the IR test**

Run: `cargo test -p x3-lang-compiler --test test_trading_ir -- --nocapture`

Expected: FAIL because `Operation::Trading` is absent.

- [ ] **Step 3: Implement stable typed IR**

Define `AssetKey` with VM family, chain, canonical ID, symbol, and decimals. Define `ValueRef` as either a literal or prior binding. Keep Serde field names stable and explicit.

- [ ] **Step 4: Implement lowering**

Resolve every symbol through `TradingSymbols`; never emit strings for unresolved assets. Automatically append `EmitTradeReceipt` only when required by borrowed capital and report normalization through diagnostics.

- [ ] **Step 5: Allocate and synchronize opcodes**

Assign unused opcode values in `opcodes.yaml`, regenerate or manually synchronize `opcodes.rs` using the repository's established generation method, and add encode/decode round-trip tests for every trading operation.

- [ ] **Step 6: Run IR, emitter, and conformance tests**

Run: `cargo test -p x3-lang-compiler --test test_trading_ir --test test_conformance --test test_compiler_pipeline`

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add x3-lang/compiler/src/ir.rs x3-lang/compiler/src/trading_lowering.rs x3-lang/compiler/src/lowering.rs x3-lang/compiler/src/emitter.rs x3-lang/spec x3-lang/compiler/tests/test_trading_ir.rs
git commit -m "feat(x3-lang): lower trading core to verified IR"
```

## Task 6: Atomic VM execution and capability boundary

**Files:**
- Create: `x3-lang/vm/src/trading.rs`
- Modify: `x3-lang/vm/src/executor.rs`
- Modify: `x3-lang/vm/src/lib.rs`
- Create: `x3-lang/vm/tests/trading_execution.rs`
- Create: `x3-lang/vm/tests/trading_properties.rs`

**Interfaces:**
- Consumes: decoded `TradingOperation`.
- Produces:

```rust
pub trait TradingHost {
    fn capabilities(&self) -> &CapabilityManifest;
    fn open_debt(&mut self, request: BorrowRequest) -> Result<BorrowResult, HostError>;
    fn swap(&mut self, request: SwapRequest) -> Result<SwapResult, HostError>;
    fn close_debt(&mut self, request: RepayRequest) -> Result<RepayResult, HostError>;
    fn execution_costs(&self) -> Result<Vec<CommittedCost>, HostError>;
}

pub enum CapabilityMode { Fixture, Production }

pub struct CapabilityManifest {
    pub mode: CapabilityMode,
    pub version: String,
    pub chain: String,
    pub state_commitment: [u8; 32],
    pub private_submission: bool,
    pub providers: BTreeSet<String>,
    pub venues: BTreeSet<String>,
}
```

- [ ] **Step 1: Add failing execution tests**

Cover success, output below `min_out`, excessive flash fee, expired deadline, state mismatch, unknown venue, missing private capability, low net profit, and host error. Assert state equality before and after every aborted execution.

- [ ] **Step 2: Run VM trading tests**

Run: `cargo test -p x3-lang-vm --test trading_execution -- --nocapture`

Expected: FAIL because the host and atomic journal do not exist.

- [ ] **Step 3: Implement the atomic journal**

Snapshot only the VM state touched by trading operations: balances, open debts, bindings, accumulated costs, and pending receipt. `CommitAtomicTrade` merges the journal; any error discards it. Do not clone or roll back unrelated external state silently; the host contract must guarantee prepare/commit/abort semantics for production capabilities.

- [ ] **Step 4: Implement checked accounting**

Use checked `u128` base-unit arithmetic. Record principal separately from fees. Reject host results with the wrong asset, wrong state commitment, fee above policy, or output below `min_output`.

- [ ] **Step 5: Enforce capability mode**

A `CapabilityMode::Fixture` manifest is accepted only in development/test mode. Production bytecode execution with fixture capabilities returns a hard `ExecError::NonProductionCapability`.

- [ ] **Step 6: Add property tests**

Generate sequences of borrow/swap/repay/guard operations and assert:

```rust
prop_assert!(!result.is_ok() || vm.trading_state.open_debts.is_empty());
prop_assert!(!result.is_err() || vm.trading_state.committed == before);
```

Also assert conservation: ending balances plus declared costs equal starting balances plus externally committed deltas.

- [ ] **Step 7: Run VM suites**

Run: `cargo test -p x3-lang-vm --all-targets --all-features`

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add x3-lang/vm/src x3-lang/vm/tests
git commit -m "feat(x3-lang): execute trades with atomic accounting"
```

## Task 7: Deterministic receipts and CLI inspection

**Files:**
- Modify: `x3-lang/vm/src/trading.rs`
- Modify: `x3-lang/crates/x3-tools/src/bin/x3c.rs`
- Create: `x3-lang/vm/tests/trading_receipts.rs`
- Modify: `x3-lang/crates/x3-tools/tests/cli.rs`

**Interfaces:**
- Produces:

```rust
pub struct TradeReceipt {
    pub format_version: u16,
    pub compiler_version: String,
    pub artifact_hash: [u8; 32],
    pub trade_id: String,
    pub policy_id: String,
    pub state_commitment: [u8; 32],
    pub operations: Vec<ReceiptOperation>,
    pub costs: Vec<CommittedCost>,
    pub debts: Vec<DebtReceipt>,
    pub deltas: Vec<AssetDelta>,
    pub realized_net_profit: Option<TypedReceiptAmount>,
    pub outcome: TradeOutcome,
    pub receipt_hash: [u8; 32],
}

pub fn canonical_receipt_bytes(receipt: &TradeReceipt) -> Result<Vec<u8>, ReceiptError>;
pub fn verify_receipt(receipt: &TradeReceipt) -> Result<(), ReceiptError>;
```

- [ ] **Step 1: Add failing receipt tests**

Test deterministic encoding, field-order independence where maps are involved, one-bit tampering, incorrect debt status, false realized profit on failure, and differing state commitments.

- [ ] **Step 2: Run receipt tests**

Run: `cargo test -p x3-lang-vm --test trading_receipts -- --nocapture`

Expected: FAIL because receipts are not defined.

- [ ] **Step 3: Implement canonical receipts**

Use ordered vectors and `BTreeMap`/sorted entries only. Hash canonical bytes with SHA-256 already available in the workspace. Compute with `receipt_hash` zeroed, then store the digest. A failed outcome must set `realized_net_profit: None`.

- [ ] **Step 4: Extend CLI**

Add:

```text
x3c receipt inspect <receipt.json>
x3c receipt verify <receipt.json>
```

`verify` exits nonzero on hash mismatch, malformed accounting, an open debt in a successful receipt, or realized profit in a failed receipt. Existing `run` output includes the receipt path/hash when a trading program executes.

- [ ] **Step 5: Run VM receipt and CLI tests**

Run: `cargo test -p x3-lang-vm --test trading_receipts && cargo test -p x3-tools --test cli`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add x3-lang/vm/src/trading.rs x3-lang/vm/tests/trading_receipts.rs x3-lang/crates/x3-tools
git commit -m "feat(x3-lang): emit verifiable trade receipts"
```

## Task 8: End-to-end example, production rejection, and documentation

**Files:**
- Create: `x3-lang/examples/trading_core_v1.x3`
- Create: `x3-lang/compiler/tests/test_trading_core_e2e.rs`
- Modify: `x3-lang/README.md`
- Modify: `x3-lang/examples/README.md`
- Modify: `x3-lang/spec/INDEX.md`

**Interfaces:**
- Consumes: complete parser-to-VM pipeline.
- Produces: canonical example and evidence commands for Trading Core v1.

- [ ] **Step 1: Add failing end-to-end tests**

The valid case must parse, type-check, verify, lower, encode, decode, execute through an explicitly marked fixture host, and verify its receipt. The production-negative case must use the same artifact with fixture capability mode and assert `NonProductionCapability`.

- [ ] **Step 2: Run the end-to-end test**

Run: `cargo test -p x3-lang-compiler --test test_trading_core_e2e -- --nocapture`

Expected: FAIL until the example and pipeline wiring are complete.

- [ ] **Step 3: Add the canonical example**

Use full-format asset identifiers accepted by the parser. Keep the example deterministic: explicit provider, venues, amounts, minimum outputs, policy, repayment, profit guard, and receipt.

- [ ] **Step 4: Document honest support boundaries**

Document syntax, type rules, verifier guarantees, fixture versus production capabilities, and the explicit statement that v1 does not provide live venue adapters or route discovery.

- [ ] **Step 5: Run focused end-to-end checks**

Run:

```bash
cargo test -p x3-lang-compiler --test test_trading_core_e2e
cargo run -p x3-tools --bin x3c -- check examples/trading_core_v1.x3 --mode dev
cargo run -p x3-tools --bin x3c -- audit examples/trading_core_v1.x3 --mode mainnet
```

Expected: the development check passes; the mainnet audit identifies missing real production capabilities rather than accepting a fixture.

- [ ] **Step 6: Commit**

```bash
git add x3-lang/examples x3-lang/compiler/tests/test_trading_core_e2e.rs x3-lang/README.md x3-lang/spec
git commit -m "docs(x3-lang): prove trading core v1 pipeline"
```

## Task 9: Full verification and delivery evidence

**Files:**
- Modify only files required to correct genuine failures introduced by Tasks 1–8.
- Create: `docs/evidence/x3-lang-trading-core-v1.md`

**Interfaces:**
- Produces exact-head verification evidence and a reviewable PR.
- Consumes all prior tasks.

- [ ] **Step 1: Format**

Run: `cargo fmt --all -- --check`

Expected: PASS. If it fails, run `cargo fmt --all`, inspect the diff, and rerun the check.

- [ ] **Step 2: Run all workspace tests**

Run: `cargo test --workspace --all-targets --all-features`

Expected: PASS with no ignored new Trading Core tests.

- [ ] **Step 3: Run Clippy as an enforcement gate**

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`

Expected: PASS.

- [ ] **Step 4: Run the repository mainnet-readiness gate**

Locate the gate that currently covers `x3-lang` in `.github/workflows` or repository scripts, record its exact command, and run it unchanged. Do not weaken, skip, or relabel enforcement.

Expected: PASS, or a documented external blocker unrelated to the change. A blocker means the PR remains draft/not-ready.

- [ ] **Step 5: Scan for prohibited evidence**

Run:

```bash
rg -n "mock|stub|fake|placeholder|todo!\(|unimplemented!\(" x3-lang docs/evidence/x3-lang-trading-core-v1.md
```

Classify every match. Test fixtures may remain only when clearly labeled and rejected by production mode. Runtime placeholders or production-path mocks are release blockers.

- [ ] **Step 6: Write exact evidence**

Record final commit SHA, commands, exit status, test counts, relevant workflow run URLs, known non-goals, and any external blockers in `docs/evidence/x3-lang-trading-core-v1.md`. Do not claim live DEX support.

- [ ] **Step 7: Review the complete branch diff**

Run:

```bash
git diff --check master...HEAD
git diff --stat master...HEAD
git diff master...HEAD -- x3-lang docs/superpowers docs/evidence
```

Check for unrelated edits, silent compatibility breaks, generated-file drift, missing negative tests, and claims stronger than evidence.

- [ ] **Step 8: Commit evidence**

```bash
git add docs/evidence/x3-lang-trading-core-v1.md
git commit -m "test(x3-lang): record trading core v1 evidence"
```

- [ ] **Step 9: Push and open a draft PR**

Push `codex/x3-trading-core-v1`, capture the exact head SHA, and open a draft PR targeting `master`. Include the specification, implementation tasks, verification results, non-goals, and exact-head evidence.

- [ ] **Step 10: Declare readiness only from exact-head checks**

Mark the PR ready only after required GitHub checks finish successfully on the same head SHA reviewed in Step 7.

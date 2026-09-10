# X3 Lang Trading Core v1 Design

**Status:** Approved design
**Date:** 2026-09-09
**Target:** `x3-lang` Rust compiler and VM workspace
**Branch:** `codex/x3-trading-core-v1`

## 1. Purpose

Trading Core v1 makes asset identity, debt, swaps, economic risk, net profit, atomic rollback, and execution receipts enforceable parts of X3 Lang. It extends the existing Rust parser, AST, semantic analysis, risk analysis, IR, verifier, VM, and CLI.

The work does not claim route discovery, cross-chain settlement, private-relay delivery, oracle consensus, or live DEX integration. Those require later adapters and integration evidence. Python mock RPC and simulator code is not production evidence.

## 2. Goals

1. Define chain-qualified assets with explicit decimals and identifiers.
2. Provide typed asset amounts and directed prices.
3. Add an `atomic trade` declaration with borrow, swap, repay, profit guard, and receipt operations.
4. Treat borrowed capital as a linear obligation that must be repaid exactly once on every successful path.
5. Enforce slippage, deadlines, fee ceilings, execution-mode requirements, and minimum net profit.
6. Lower the syntax into deterministic trading IR.
7. Verify debt closure and profit checks before executable output is accepted.
8. Execute trades atomically through a capability-controlled VM host interface.
9. Produce deterministic, tamper-evident success and failure receipts.
10. Prove the behavior with unit, conformance, end-to-end, and property tests.

## 3. Non-goals

Trading Core v1 does not implement:

- automatic or optimal route discovery;
- live Aave, Uniswap, SushiSwap, bridge, or private-relay adapters;
- cross-chain atomic settlement;
- oracle aggregation or price-source consensus;
- AI-selected routes or AI authority over funds;
- backtesting or historical data ingestion;
- hardware-accelerated route search;
- market making, liquidations, options, or portfolio strategies;
- claims of mainnet execution based on fixtures, mocks, or simulated providers.

An explicit, verifiable route is the only execution input in v1.

## 4. Architecture

The canonical implementation is the Rust workspace under `x3-lang`. Trading constructs follow the existing pipeline:

1. Lexer/parser creates a typed AST.
2. Semantic analysis resolves assets, types, debt ownership, and scopes.
3. Risk analysis validates the selected policy and mainnet constraints.
4. Lowering produces deterministic trading IR.
5. The IR verifier checks control-flow and economic invariants.
6. The VM executes through explicitly granted host capabilities.
7. The receipt builder commits the execution inputs and results.

The older Python parser, simulator, runner, and mock RPC are excluded from acceptance evidence.

## 5. Source model

### 5.1 Example

```x3
asset USDC = evm.ethereum.0xA0b8 {
    decimals: 6
}

asset WETH = evm.ethereum.0xC02a {
    decimals: 18
}

risk policy MainnetArb {
    max_slippage: 30 bps
    max_gas: 0.02 ETH
    max_flash_fee: 10 bps
    deadline: 2 blocks
    require_private_submission: true
}

atomic trade CrossDexArb using MainnetArb {
    borrow 1_000_000 USDC from aave_v3 as debt

    let weth = swap debt.amount USDC -> WETH
        via uniswap_v3
        min_out 410 WETH

    let returned = swap weth WETH -> USDC
        via sushiswap
        min_out 1_002_000 USDC

    repay debt

    require net_profit >= 1_000 USDC
    require all_debts_repaid
    emit receipt
}
```

The shortened identifiers in documentation are illustrative. Production-mode compilation requires identifiers that satisfy the target VM's address or mint format.

### 5.2 Core declarations

- `asset` binds a symbolic name to a VM family, chain, canonical identifier, and decimal count.
- `risk policy` defines bounded execution constraints.
- `atomic trade` defines an all-or-nothing economic transaction and names its risk policy.
- `borrow ... as <debt>` creates both an amount and a linear debt obligation.
- `swap` consumes a typed amount and produces another typed amount.
- `repay` consumes exactly one debt obligation.
- `require net_profit` sets the minimum acceptable normalized final balance delta.
- `require all_debts_repaid` explicitly asserts the obligation invariant.
- `emit receipt` requests the mandatory deterministic record. For borrowed-capital trades the compiler inserts this requirement if absent and reports that normalization.

## 6. Type and accounting model

### 6.1 Asset identity

An asset type is identified by:

`Asset<VmFamily, ChainId, CanonicalIdentifier, Decimals>`

Assets with different chain IDs, VM families, identifiers, or decimals are distinct. Symbol equality alone never establishes equivalence.

### 6.2 Amounts

`Amount<A>` stores an unsigned integer in the base units of asset `A`. Arithmetic is checked:

- addition and subtraction require the same asset type;
- subtraction cannot underflow;
- multiplication and division require an explicit rounding direction when precision can be lost;
- implicit floating-point arithmetic is forbidden;
- decimal literals are converted to base units at compile time when exact, otherwise rejected.

### 6.3 Prices

`Price<Base, Quote>` is directional. Inversion must be explicit and checked. Applying a price to `Amount<Base>` produces `Amount<Quote>` with an explicit rounding policy.

### 6.4 Debt

`Debt<Provider, Asset>` is a linear value. It cannot be copied, discarded, stored beyond the atomic trade, returned twice, or merged with unrelated obligations. Repayment includes principal and the provider fee reported by the authorized host operation, bounded by the selected risk policy.

### 6.5 Net profit

Net profit is the final balance delta in a declared settlement asset after:

- principal;
- flash-liquidity fees;
- venue fees;
- gas or execution cost;
- private-submission tips;
- declared bridge costs when cross-chain support is added later.

Gross output is never accepted as net profit. If costs cannot be represented in the settlement asset using an approved and committed conversion, net profit is unverifiable and execution is rejected.

## 7. Risk policy

Trading Core v1 supports these enforceable fields:

- `max_slippage` in basis points;
- `max_gas` as a typed asset amount;
- `max_flash_fee` in basis points;
- `deadline` in blocks or seconds, bound to one declared clock domain;
- `require_private_submission` as an execution capability requirement;
- `min_profit`, either supplied by policy or by the trade guard.

Production/mainnet mode requires bounded slippage, a deadline, a minimum net-profit guard, and a compatible execution capability. Unknown providers, venues, chains, or capabilities are hard errors.

## 8. Intermediate representation

The trading IR adds or formalizes operations equivalent to:

- `DeclareAsset`
- `BeginAtomicTrade`
- `OpenDebt`
- `ExecuteSwap`
- `AccrueCost`
- `CloseDebt`
- `AssertMinNetProfit`
- `AssertAllDebtsClosed`
- `EmitTradeReceipt`
- `CommitAtomicTrade`
- `AbortAtomicTrade`

Each operation carries typed asset information and source spans. Provider and venue calls reference capability identifiers, never arbitrary ambient host functions.

## 9. Verification rules

The IR verifier rejects a program when:

1. asset types are incompatible;
2. amount arithmetic can silently overflow, underflow, or lose precision;
3. a debt can reach a successful exit without being closed;
4. a debt may be repaid more than once;
5. repayment is conditional without closure on every successful branch;
6. borrowed assets escape the atomic region;
7. a swap lacks a venue or minimum output;
8. mainnet-mode execution lacks bounded slippage or a deadline;
9. net profit is absent, uses gross output, or cannot be normalized;
10. an unknown or unauthorized capability is required;
11. receipt generation can be skipped on a borrowed-capital path;
12. commit can occur before all guards and debt checks pass.

The verifier is conservative: inability to prove safety is rejection, not a warning.

## 10. VM and host boundary

The VM owns accounting, atomic state, guard evaluation, rollback, and receipt construction. External venue/provider behavior is exposed through capability-controlled host operations.

Before execution, the host supplies:

- capability identity and version;
- chain and state commitment;
- supported asset pairs;
- deterministic fee declaration or bounded fee result;
- execution-mode properties, including private submission when required.

The VM validates host results against the compiled types and policy. Output below `min_out`, fee above the ceiling, deadline expiry, state mismatch, malformed output, or net profit below the floor aborts the complete trade.

Fixtures may implement the host interface for deterministic tests but must be visibly marked non-production and cannot satisfy production capability attestation.

## 11. Receipts

A canonical receipt contains:

- language, compiler, IR, and capability-interface versions;
- source or artifact hash;
- trade and risk-policy identifiers;
- chain and pre-execution state commitment;
- ordered route operations;
- input and output asset amounts;
- provider, venue, gas, tip, and other costs;
- debt principal, fee, and repayment status;
- guard outcomes;
- final asset deltas and realized net profit;
- success or failure classification;
- deterministic receipt hash.

A failure receipt records observed execution and the abort reason but never reports realized profit. Receipt encoding is canonical so identical committed inputs and results produce the same hash. Tampering changes the hash and fails verification.

## 12. Diagnostics and failure behavior

Diagnostics must identify the source span, invariant, and corrective action. Required cases include:

- cross-chain or cross-asset arithmetic;
- decimal mismatch;
- missing or duplicate repayment;
- debt escape;
- missing minimum output;
- missing deadline or slippage bound;
- excessive fee or slippage setting;
- unsupported venue/provider capability;
- unverifiable profit currency;
- overflow, underflow, and invalid rounding;
- state-commitment mismatch;
- receipt validation failure.

Runtime failures abort atomically. No successful receipt is emitted for a rolled-back trade.

## 13. CLI behavior

The existing Rust CLI gains support for:

- parsing and formatting trading declarations;
- semantic and risk checking;
- emitting and inspecting trading IR;
- verifying executable artifacts;
- inspecting and verifying receipts;
- selecting development or production/mainnet verification mode.

Production verification prints the exact missing capability or evidence instead of falling back to fixtures.

## 14. Testing and acceptance

### 14.1 Required tests

- Lexer and parser acceptance/rejection coverage.
- Formatter round-trip coverage.
- Asset, amount, price-direction, decimals, and rounding type tests.
- Debt linearity and control-flow tests.
- Risk-policy validation tests.
- IR lowering and stable snapshot/conformance tests.
- IR verifier negative tests for every rule in section 9.
- VM success, guard failure, and atomic rollback tests.
- Receipt determinism and tamper-detection tests.
- Property tests for asset conservation.
- Property tests proving no successful execution contains open debt.
- CLI integration tests.
- Existing workspace regression suite.

### 14.2 Required commands

From `x3-lang`:

```bash
cargo fmt --all -- --check
cargo test --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Any repository-specific mainnet-readiness gate that covers `x3-lang` must also pass on the exact head commit.

### 14.3 Acceptance criteria

Trading Core v1 is complete only when:

1. `examples/trading_core_v1.x3` passes the complete Rust pipeline;
2. invalid examples fail at their intended compiler or verifier stage;
3. every successful borrowed-capital control-flow path closes its debt;
4. profit is computed net of all committed costs;
5. VM state fully rolls back after any failed guard or host constraint;
6. success and failure receipts verify and tampering is detected;
7. production mode refuses fixture or mock capabilities;
8. all required commands and applicable repository gates pass on the final head;
9. the PR documents exact command output and workflow evidence;
10. no live-integration or mainnet-readiness claim exceeds the available evidence.

## 15. Delivery structure

Implementation should be split into reviewable commits:

1. syntax, AST, and formatter;
2. semantic types and debt ownership;
3. trading IR and lowering;
4. verifier and risk-policy enforcement;
5. VM host boundary, atomic accounting, and receipts;
6. CLI integration, examples, conformance fixtures, and documentation;
7. final regression and proof evidence.

The PR remains a draft until the complete vertical slice passes. Partial parser-only support must not be advertised as Trading Core completion.

## 16. Future extensions

Later designs may add route discovery, oracle consensus, real venue/provider adapters, private relay transport, cross-chain settlement, deterministic backtesting, AI proposals constrained by compiled policy, strategy lifecycle controls, and X3 hardware acceleration. Each extension must preserve the v1 type, debt, atomicity, profit, capability, and receipt invariants.

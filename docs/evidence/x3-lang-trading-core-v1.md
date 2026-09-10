# X3 Lang Trading Core v1 — Verification Evidence

**Date:** 2026-09-09
**Branch:** `codex/x3-trading-core-v1`
**Verified code head before this evidence commit:** `811b46cee`
**Remote:** `origin/codex/x3-trading-core-v1`

## Scope delivered

- Typed, chain-qualified trading AST and lexer keywords.
- Asset, risk-policy, and atomic-trade parser/formatter support with fixtures.
- Asset registry, decimal conversion, swap/min-out/profit typing, and overflow checks.
- Linear debt-flow verification, guard requirements, policy bounds, and mainnet private-submission rejection.
- Deterministic trading IR, lowering, opcodes `0xB0..0xB8`, emitter/decode round-trips, and structural IR checks.
- Capability-controlled atomic VM execution with checked accounting and rollback.
- Canonical, hash-verified trade receipts plus `x3c receipt inspect` / `x3c receipt verify`.
- `examples/trading_core_v1.x3` and an end-to-end compiler→lower→VM→receipt test.

## Commands and results

All commands were run from `x3-lang` with:

```bash
CARGO_TARGET_DIR=/home/lojak/Desktop/xxxstar-main/x3-lang/target
```

| Command | Result |
|---|---|
| `cargo test --workspace --all-targets --all-features` | PASS |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS |
| `cargo fmt --all -- --check` | PASS |

Notable focused suites included in the workspace run:

- `test_trading_parser`: 8/8
- `test_trading_types`: 9/9
- `test_trading_ir`: 3/3
- `test_trading_core_e2e`: 3/3
- `trading_verifier`: 9/9
- `trading_execution`: 8/8
- `trading_properties`: 2/2
- `trading_receipts`: 6/6
- `x3-tools` CLI: 12/12 including receipt inspect/verify

## Repository readiness workflow

Located gate: `.github/workflows/x3-lang-readiness.yml`.

The workflow triggers on `x3-lang/**`, but its job commands target root-repo
crates (`x3-parser`, `x3-typeck`, `x3-opt`, `x3-verifier`, `x3-compiler`,
`x3-vm`, `x3-evm-integration`, `x3-svm-integration`). It does not execute the
nested `x3-lang` workspace commands that cover this change. The applicable
`x3-lang` workspace gates above were run unchanged and passed. The workflow
itself remains a root-workspace gate and was not weakened or relabeled.

## Prohibited evidence scan

Targeted scan over changed trading code:

```bash
rg -n "TODO|FIXME|stub|fake|placeholder|dummy|unimplemented!|todo!|panic!\(\"not implemented" \
  x3-lang/compiler/src/trading_*.rs x3-lang/compiler/src/ir.rs \
  x3-lang/compiler/src/lowering.rs x3-lang/compiler/src/emitter.rs \
  x3-lang/vm/src/trading.rs x3-lang/vm/src/executor.rs \
  x3-lang/crates/x3-tools/src/bin/x3c.rs x3-lang/examples/trading_core_v1.x3
```

Result: no matches.

A repo-wide scan finds only pre-existing documentation/test-fixture text and
unrelated placeholder machinery outside the Trading Core v1 diff.

## Honest non-goals and boundaries

- No live Aave, Uniswap, SushiSwap, bridge, oracle, or private-relay adapter.
- No route discovery, optimal routing, or cross-chain settlement.
- Fixture capabilities are rejected in production/mainnet execution mode.
- No mainnet-readiness or live-DEX claim is made from this evidence.
- Trading Core v1 is compiled, tested, and fixture-executable; real venue
  integration evidence must be added by later adapter work.

## Known operational blocker

The user-approved subagent-driven execution mode could not be used after Task 2:
newly spawned agents repeatedly received environment context without the task
payload. Tasks 3–9 were therefore completed inline in the root agent with the
same test-first and commit-per-task gates.

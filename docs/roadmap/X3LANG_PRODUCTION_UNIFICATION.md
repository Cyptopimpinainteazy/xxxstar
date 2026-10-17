# X3 Lang Production Unification

Branch: `codex/x3lang-production-unification`
Base: `master` @ `70423054e2d9560a212ab763d4f265f9cdc448d9` (verified 2026-10-17 by `git fetch`).

Goal: one `.x3` trading program compiles into canonical X3BC, is decoded and verified by the
production `no_std` code, executes through `crates/x3-vm` against a transactional production host
on chain, charges every cost before the final profit check, commits or rolls back completely, and
produces a signed receipt that independent replay re-derives without trusting its totals.

Nothing in this document is evidence. The test matrix at the end names the tests that are; a row
is claimed only once its test is green at the branch's exact remote head.

## 0. Decisions recorded for this branch

| Decision | Source | Effect |
|---|---|---|
| Root X3BC (`crates/x3-backend` format, `crates/x3-vm` engine) is the canonical production bytecode and VM. | Owner's mission prompt, 2026-10-17 | **Supersedes** `docs/x3-lang/architecture-authority.md` (2026-09-02), which assigned opcodes, verifier, gas and VM semantics to `x3-lang/`. That doc now carries a superseded banner pointing here. |
| The trading extension runs on chain (kernel pallet), shipped as a runtime upgrade with a `spec_version` bump. | Owner, 2026-10-17 ("get it running on chain") | The extension is not left refused on chain; it is wired through `pallet_x3_kernel`. |
| On-chain venues are `pallet_x3_dex` and `pallet_x3_flashloan`, made custody-real. | Owner, 2026-10-17 | `pallet_x3_dex::swap` today edits reserve numbers only and moves no funds; `pallet_x3_flashloan` lends only the native token, one loan chain-wide. Both are fixed to move `CanonicalLedger` balances. |

## 1. Pipeline map — root (production chain path)

| Stage | Location | Entry | no_std |
|---|---|---|---|
| Lex / parse | `crates/x3-lexer`, `crates/x3-parser` | `x3_parser::parse_program` | no |
| Resolve | `crates/x3-semantics/src/resolver.rs:58` | `Resolver::resolve` | no |
| Type check | `crates/x3-typeck/src/checker.rs:46` | `TypeChecker::check` (result discarded, `crates/x3-compiler/src/compiler.rs:87`) | no |
| HIR | `crates/x3-hir/src/lower.rs:28` | `HirLowerer::lower` (re-derives types from AST) | no |
| MIR | `crates/x3-mir/src/lower.rs:11` | `MirLowerer::lower` | no |
| Optimize | `crates/x3-opt/src/optimizer.rs:93,171` | `Optimizer::run` | no |
| Backend | `crates/x3-backend/src/mir_lower.rs:25` | `MirBytecodeCompiler::compile_with_options` | partly (format, opcodes yes; lowering no) |
| Driver | `crates/x3-compiler/src/compiler.rs:53` | `Compiler::compile` | no |
| Source → bytes | `crates/x3-integration/src/compiler_bridge.rs:45` | `compile_source[_with_policy]` | no |
| Container | `crates/x3-backend/src/bc_format.rs:358,408`; header consts `crates/x3-common/src/lib.rs:207` | `BytecodeModule::{to_bytes,from_bytes}` | yes |
| Verify | `crates/x3-vm/src/verifier.rs:94,131` | `Verifier::verify_module_bytes` (+ `VerifyOptions::on_chain`) | yes |
| Execute | `crates/x3-vm/src/vm.rs:453,607` | `VM::with_config_and_seeds`, `call_function(0)` | yes |
| Chain executor | `crates/x3-integration/src/executor.rs:391` | `X3Executor::execute_on_chain_with_context` | yes |
| Kernel | `pallets/x3-kernel/src/wasm_adapters.rs:327`, `lib.rs:1794` | `WasmX3Adapter`, `submit_comit_v2` | yes |
| Receipt | `pallets/x3-kernel/src/lib.rs` `ExecutionReceipt` → `X3ExecutionReceipts` | stored per comit | yes |

Host surface today: no host trait. `HostcallRegistry` closures (`crates/x3-vm/src/hostcall.rs:42`)
are reachable on chain by no opcode. Storage is VM-internal (`VmStorage`, snapshot stack +
journal); events are `EventBuffer`; `AtomicBegin/Commit/Rollback` snapshot registers, globals,
storage (depth 32). `AtomicRollback` restores and then aborts the whole execution.
Kernel atomicity is FRAME's implicit per-extrinsic layer; slot writes are applied only after every
check passes. **There are no trading, flash, swap or route opcodes in the root engine.**

## 2. Pipeline map — nested `x3-lang` (trading)

| Stage | Location |
|---|---|
| Parse | `x3-lang/compiler/src/parser.rs:66` (`atomic trade` :1048, `risk policy` :883, `asset` :831) |
| Trading semantics | `x3-lang/compiler/src/trading_semantic.rs:63` (ignores compilation mode) |
| Trading verifier | `x3-lang/compiler/src/trading_verify.rs:24` |
| Lowering | `x3-lang/compiler/src/trading_lowering.rs:39` → `ir::TradingOperation` (`ir.rs:804`, 11 variants) |
| IR verifier | `x3-lang/compiler/src/verify.rs:45,821` |
| Bytecode | `x3-lang/compiler/src/emitter.rs:1007` — `[op u8][u16 len][serde_json(TradingOperation)]` |
| Generic VM | `x3-lang/vm/src/executor.rs:701` — trading opcodes are **collected, not executed** |
| TradingVm | `x3-lang/vm/src/trading.rs:634` `execute_atomic` / `simulate_atomic` |
| Receipt | `trading.rs:1591` `TradeReceipt`, `build_receipt` :2549, Ed25519 `sign_receipt` :2230 |
| Replay | `verify_receipt_economics` :1824 — checks the receipt's own figures; does not re-execute |
| Driver | `x3-lang/crates/x3-tools/src/bin/x3c.rs:3339` with `NeutralFixtureHost` (Fixture mode) |

No production `TradingHost` exists. No dependency in either direction between `x3-lang/*` and root
crates.

## 3. Duplicates

| Kind | Canonical after this branch | Duplicate | Action |
|---|---|---|---|
| Opcode table | `x3_backend::opcode::Opcode` (moved to `crates/x3bc-core`, re-exported) | `crates/x3-vm/src/bytecode.rs` (14-op stack ISA, own "X3BC" magic, `u16` version) | **delete** (unreferenced) |
| Opcode table | same | `x3-lang/spec/opcodes.rs` (~87 ops; trading `0xB0–0xBA` collide with root EVM `0xB0–0xB9`) | keep as reference-oracle ISA only; trading lowers to root `0xE0–0xEA` |
| Decoder | `Verifier::decode_all_instructions` + inline `vm.rs` fetch | `crates/x3-integration/src/mini_x3.rs` | keep as differential oracle (already off the chain path) |
| Decoder | same | nested `emitter::decode_trading_program` (serde_json) | oracle only |
| Gas | `x3_vm::verifier::opcode_gas_cost` (moves into the shared spec table) | `crates/x3-vm/src/gas.rs` (`cost::*`, `GasMeter`) | **delete** (unreferenced) |
| Gas | same | `x3-vm/src/gas_metering_audit.rs`, `x3-verifier/src/rules.rs` gas map, `X3GasConfig`, mini_x3 flat 1/op, nested `base_gas_cost` | documented non-authoritative; spec-table exhaustiveness test pins the canonical one |
| Version consts | `x3_common::bytecode::{VERSION,…}` (moved to core) | `x3-vm/src/bytecode.rs::X3BC_VERSION`; nested `BYTECODE_VERSION_*` | delete / oracle |
| Revert enum | `VMErrorKind` | `crates/x3-vm/src/revert.rs` | **delete** (unreferenced) |
| Receipt | kernel `ExecutionReceipt` + new trading receipt | nested `TradeReceipt` | nested becomes oracle |

## 4. Trading constructs

Implemented end to end in the nested reference (TradingVm): asset declarations, risk policy
(slippage, max cost, flash-fee bps, deadline, private submission, min profit, oracle deviation,
cumulative loss, quote freshness, price impact, MEV leakage), `borrow`, `swap … min_out`, `bridge`,
`repay`, `require net_profit`, `require all_debts_repaid`, `invariant solvent`, `emit receipt`.

Documented or parsed but **not executable** anywhere: solver markets (lowered to nothing,
`lowering.rs:823`), relayer/attestor quorum, `principal_preserved`, hedging, unrealized profit,
flash capital inside `arb`/`hyperarb` (refused), everything in `x3-lang/spec/x3-profit-primitives.md`
and `examples/legacy/*.x3`.

Root engine: none of the above.

## 5. Side effects and transaction boundaries (target)

Every economic effect is a ledger entry in the VM's trading journal, snapshotted by the same
`AtomicBegin` that snapshots registers/globals/storage. Nothing leaves the VM until the execution
succeeds; the kernel then applies the resulting delta inside a storage transaction and checks
per-asset conservation before committing.

| Effect | Journaled in VM | Applied by kernel |
|---|---|---|
| Trader balances per asset | yes | `CanonicalLedger` |
| Flash pool liquidity, debt principal/fee/repayment | yes | `pallet_x3_flashloan` asset pools |
| Venue reserves | yes | `pallet_x3_dex` pools + pool custody balances |
| Costs (gas, flash, venue, protocol, settlement, relay, oracle, cross-domain) | yes, before profit | fee collector balances |
| Events, storage | existing journal | existing `X3ContractStorage` path |
| Route/solver selection, host transcript, nonces | yes | receipt |

## 6. Bytecode extension

* Opcodes `0xE0–0xEA` (trading family) in the canonical table, each with operand widths, gas,
  verifier class and trap identity in the spec table.
* A `TRADING_EXTENSION` feature bit and a trading section after metadata (canonical length-prefixed
  binary, no serde): chain id, policy, capability requirements with versions, assets, operations.
* Modules using it are written as format `1.1.0` with `min_version = 1.1.0`. Readers on master
  (`1.0`) refuse them; base modules are byte-identical to today.

## 7. Implementation plan (commits)

1. Phase A — this document; superseded banner on `architecture-authority.md`.
2. Phase B — RED: `crates/x3-integration/tests/trading_production_e2e.rs` and negative-case suites,
   nested compiler bridge test; push; record the failing run.
3. Phase C — `crates/x3bc-core` (no_std, no Substrate): header, opcode table + spec table, format;
   delete dead x3-vm modules; exhaustiveness tests.
4. Phase D — trading section format + stateful static verifier in core; nested compiler lowers
   `TradingOperation` → X3BC through core; unsupported constructs get explicit diagnostics.
5. Phase E — x3-vm trading ledger + opcodes, journaled under atomic windows.
6. Phase F — costs, receipt, attestor signing, independent replay.
7. Phase G — named E2E + paired rollback test.
8. On-chain — dex/flashloan custody, kernel `submit_trade`, runtime `spec_version` bump.
9. Phase H — workflows, exact-head proof, independent review, PR.

## 8. Feature-to-test matrix

Filled in as tests land. Status values: NOT RUN / RUNNING / PASS / FAIL / BLOCKED.

| Feature | Test | Status |
|---|---|---|
| `.x3` → X3BC → decode → verify → x3-vm → commit → signed receipt → replay | `crates/x3-integration/tests/x3lang_trading_production_e2e.rs::x3lang_trading_source_to_signed_replayed_receipt_on_production_path` | RED |
| Trap after a state-changing swap rolls back everything | `…production_e2e.rs::x3lang_trading_trap_after_swap_rolls_back_everything` | RED |
| Compile-time refusals (unresolved binding/debt, overflow, off-chain asset, asset id width) | `crates/x3-integration/tests/x3lang_trading_negative.rs` | RED |
| Artifact refusals (truncated, malformed section, unsupported version, 1.0 reader) | `x3lang_trading_negative.rs` | RED |
| Runtime refusals (chain, capability version, missing capability, gas, oracle stale/missing/divergent/deviation, slippage, flash fee, profit floor, cost ceiling, host refusal, failure before repay, overflow, deadline, private route, checked conversion) | `x3lang_trading_negative.rs` | RED |
| Signed receipt, forged/modified receipt, unknown/impostor/revoked attestor, artifact binding, canonical encoding | `crates/x3-integration/tests/x3lang_trading_receipt_replay.rs` | RED |
| Commit releases every journaled effect; failure at every step leaves nothing; nested windows | `crates/x3-integration/tests/x3lang_trading_rollback.rs` | RED |
| Production engine vs nested reference oracle | `crates/x3-integration/tests/x3lang_trading_differential.rs` | RED |
| One opcode table: decoder, verifier, gas, execution, traps, differential coverage; dead tables removed | `crates/x3-vm/tests/opcode_spec_exhaustive.rs` | RED |
| Nested compiler lowers to canonical X3BC; explicit diagnostics for unsupported constructs; mode propagation | `x3-lang/compiler/tests/test_x3bc_lowering.rs` | RED |
| Root/nested fmt, check, clippy `-D warnings`, tests; no_std core and engine builds | `.github/workflows/x3lang-production-unification.yml` | NOT RUN |

## 9. Baseline CI at `70423054`

| Workflow | Run | Result |
|---|---|---|
| X3 Compiled Storage Build | 38903018066 | success |
| X3Lang Canonical Compiler Bridge | 38903018023 | success |
| X3Lang Control Flow Parity | 38903018005 | cancelled after ~24h queued (never ran) |
| Local CI | 38903017999 | cancelled after ~24h queued (never ran); previous master `7bb25149` reported `x3/local-ci: 9 gate(s) failed` |

All workflows run on self-hosted `[self-hosted, Linux, X64, x3]` runners. `x3-lang-trading-core.yml`
is `workflow_dispatch` only, and this session's GitHub API token is invalid, so it can only run when
someone dispatches it; push-triggered workflows are the provable path.

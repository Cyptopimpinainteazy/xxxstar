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
| `.x3` → X3BC → decode → verify → x3-vm → commit → signed receipt → replay | `crates/x3-integration/tests/x3lang_trading_production_e2e.rs::x3lang_trading_source_to_signed_replayed_receipt_on_production_path` | GREEN locally (exact-head CI pending) |
| Trap after a state-changing swap rolls back everything | `…production_e2e.rs::x3lang_trading_trap_after_swap_rolls_back_everything` | GREEN locally (exact-head CI pending) |
| Compile-time refusals (unresolved binding/debt, overflow, off-chain asset, asset id width) | `crates/x3-integration/tests/x3lang_trading_negative.rs` | GREEN locally (exact-head CI pending) |
| Artifact refusals (truncated, malformed section, unsupported version, 1.0 reader) | `x3lang_trading_negative.rs` | GREEN locally (exact-head CI pending) |
| Runtime refusals (chain, capability version, missing capability, gas, oracle stale/missing/divergent/deviation, slippage, flash fee, profit floor, cost ceiling, host refusal, failure before repay, overflow, deadline, private route, checked conversion) | `x3lang_trading_negative.rs` | GREEN locally (exact-head CI pending) |
| Signed receipt, forged/modified receipt, unknown/impostor/revoked attestor, artifact binding, canonical encoding | `crates/x3-integration/tests/x3lang_trading_receipt_replay.rs` | GREEN locally (exact-head CI pending) |
| Commit releases every journaled effect; failure at every step leaves nothing; nested windows | `crates/x3-integration/tests/x3lang_trading_rollback.rs` | GREEN locally (exact-head CI pending) |
| Production engine vs nested reference oracle | `crates/x3-integration/tests/x3lang_trading_differential.rs` | GREEN locally (exact-head CI pending) |
| One opcode table: decoder, verifier, gas, execution, traps, differential coverage; dead tables removed | `crates/x3-vm/tests/opcode_spec_exhaustive.rs` | GREEN locally (exact-head CI pending) |
| Nested compiler lowers to canonical X3BC; explicit diagnostics for unsupported constructs; mode propagation | `x3-lang/compiler/tests/test_x3bc_lowering.rs` | GREEN locally (exact-head CI pending) |
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

## Status (2026-10-18)

### On chain

- `pallet_x3_kernel::submit_trade` (call 44) runs a compiled trading module on the production
  engine against chain state and settles it on `CanonicalLedger`; `set_trading_config` (call 45,
  governance) sets the cost schedule and fee collector, and trading is refused until it is set.
  Asset decimals come from `AssetRegistry`; a program declaring others is refused. A rolled-back
  trade moves nothing and its receipt is recorded; a settlement the chain's state disagrees with
  fails the extrinsic and writes nothing.
- The runtime's `X3TradingHost` exposes `pallet_x3_dex` pools (venues), `pallet_x3_flashloan`
  asset pools (flash provider) and `pallet_x3_oracle` medians. `mainnet-rc1` has none of those
  pallets, so there `TradingHost = ()` and every trade is refused. `spec_version` 22 → 23.
- No private submission channel exists on chain, so a program that demands one (the flagship
  `x3chain_flash_arb.x3` does) is refused; the same program with public submission commits.
- Weight: `submit_trade` is charged up front for the largest trade the section limits allow and
  refunded to what it did (`TradeMeter`), from the same per-item read/write counts; a module
  refused before the interpreter runs pays the engine's base cost plus one gas unit per byte.
  The flagship pays under a tenth of the bound.
- Trades are refused while the supply ledger's transfer halt is set. Receipts are carried in
  the trade's event; state keeps only the digest, trader, block and outcome.
- Independent review (2026-10-18): no fund-theft path found; its high and medium findings and
  the low ones listed in commit `791f555d` are fixed, each with a test.

### Verified locally (Rust stable 1.97; the repo pins 1.90; wasm target unavailable here)

| Gate | Result |
|---|---|
| `pallet-x3-kernel` tests (incl. 15 `tests_trading`) | 276 pass |
| `x3-chain-runtime` tests (incl. 3 `tests_trading` on real dex/flash/oracle pallets: commit, rollback, halt) | 116 pass |
| `pallet-x3-dex` + `pallet-x3-flashloan` (all features) | 48 pass |
| settlement-engine, atomic-trade-engine, x3-coin (kernel mocks changed) | 341 pass |
| trading suites, x3bc-core, x3-vm, x3-integration, x3-backend, x3-common, x3-opt, x3-bench | pass |
| clippy `-D warnings --no-deps`: kernel, dex, flashloan, runtime (default, `mainnet-rc1`, `runtime-benchmarks`) | clean |
| nested `x3-lang` fmt / clippy / tests | pass |
| Local CI python gates | no new failures vs master (7 fail on master too) |

Not proven yet: exact-head CI on the self-hosted runners (they use 1.90 and build the wasm
runtime); independent review; runtime wasm hash re-recording (`check-runtime-hash-freshness` is
already failing on master: its recorded revision is not in the repository).

### Behaviour changes to know about

- `x3-dex` pool ids are sequential from 1 (the old hash collided for every pool on one chain).
- `x3-flashloan::borrow` (native) is refused: it never enforced repayment inside the extrinsic.
- Launchpad graduation's pool creation is refused (`TokenNotOnLedger`): token-factory balances
  are not on `CanonicalLedger`, so the pool could not custody anything. It previously recorded an
  empty pool under a wrong id (0).

### Security findings from the audit

- Fixed: kernel `WasmX3Adapter` dropped every X3 event; they are now delivered as
  `X3LogEmitted` events (not kept in state: `Emit` is priced per event, not per byte).
- Fixed: `x3-verifier::verify_bytecode` accepted everything; it is removed, and the std
  `X3Executor::estimate_gas` now verifies with the chain's verifier and returns the chain's estimate.
- Fixed (fail closed): the atomic kernel's `CompositeReverter` wrote every revert to keys no
  pallet reads (crate names instead of runtime pallet names, two nonexistent storage items, the
  wrong EVM hasher) and reported `Reverted`. Its diffs come from an unsigned, unauthenticated
  extrinsic, so correcting the keys would have let anyone choose what a rollback writes. It now
  refuses every non-empty diff (`UnauthenticatedStateDiff`) and the pallet reports
  `IncompleteVmRevert`.
- Fixed (spec 24): legs now execute on chain. The bundle's executor calls the signed
  `execute_bundle_leg` (call 9); the runtime's `RuntimeLegExecutor` runs an X3 program on the
  kernel engine or a `CanonicalLedger` transfer of the executor's own funds, and the pallet stores
  the journal and its hash (`ChainJournal`). Rollback applies a journal only when its hash matches,
  newest leg first, all or nothing per leg; a slot is restored only while it still holds the
  leg's value. The unsigned `record_leg_execution_receipt` refuses every call and
  `validate_unsigned` drops it. The node gateway submits `execute_bundle_leg` and refuses legs the
  chain cannot run (EVM/SVM bytecode, transfers from another account or another VM's balances).
  Tests: `pallet-x3-atomic-kernel` (rollback, forgery, conflict, executor-funds-only) and
  `runtime` `chain_executed_legs_are_undone_exactly_by_a_rollback`.
- Fixed after independent review (same spec 24):
  - a bundle finalized with legs never executed: finalization now requires every declared leg
    executed with its recorded journal (`LegNotExecuted`), so a bundle declaring EVM/SVM legs
    cannot finalize at all;
  - a rollback that undid some legs and still reported `RolledBack`: rollback is one storage
    transaction over every leg and refuses (`LegRevertFailed`) rather than half-apply; expiry
    retries such a bundle next block instead of marking it rolled back;
  - what made a revert fail in the first place: transfer legs pay into a per-bundle escrow (paid
    out on finalize, back on rollback), so a recipient never holds revocable funds; X3 slots a leg
    writes are held for the bundle (`X3SlotHolds`) and no other program may write them;
  - unbounded finishing work: a bundle's legs hold at most 64 journal entries together,
    `rollback`/`finalize` are charged for all of them, and `on_initialize` undoes at most 4 expired
    bundles per block, carrying the rest;
  - legs now refuse during an economic halt and after the bundle's deadline; finalized bundles
    drop their receipts and journals.
  - Launchpad: a launch whose soft cap cannot seed a pool is refused at creation; any account may
    graduate a successful launch; governance may cancel an ungraduated successful launch so
    contributors are refunded.
- Open: what a leg executes is not bound to the `BundleLeg` it fills (token, amount, access); the
  executor is a genesis-authorized gateway, so this is an integrity gap rather than an open
  exploit. Binding it needs a declared-execution commitment in the submission (a call-encoding
  change).
- Open: the token factory's "ledger balances sum to the `X3Native` supply leg" holds only while
  `x3-cross-vm-router`'s gateway debit of that leg is not used on a factory asset.
- Fixed: `Emit` charges 20 gas plus 8 per data byte (`x3bc_core::spec::EMIT_GAS_PER_DATA_BYTE`).
- Left unchanged: `x3-flashloan::pool_account` (native pool) decodes its account with `.expect`;
  changing the derivation would move an account that already holds funds.

### `submit_trade` benchmark

`pallet_x3_kernel` benchmark `submit_trade` runs the compiled public flagship
(`pallets/x3-kernel/src/benchmark_trade.x3bc`, held to the compiler by
`tests_trading::the_benchmark_trade_is_the_compiled_flagship`) against the market
`KernelTradingHost::benchmark_market` builds through the dex, flash-loan and oracle pallets, and
fails unless the trade commits. Verified locally: the pallet benchmark suite (mock) and the runtime
test `the_benchmark_market_commits_the_benchmark_trade` (`--features runtime-benchmarks`). Not yet
measured on reference hardware; on the server:

```bash
cargo build --release -p x3-chain-node --features runtime-benchmarks
./target/release/x3-chain-node benchmark pallet --chain dev --pallet pallet_x3_kernel \
  --extrinsic submit_trade --steps 2 --repeat 20
```

Compare the result with `Pallet::trade_weight()` (the charged upper bound); it is a model of
measured `x3_execute` plus per-structure database weight and does not price verification and
settlement compute separately.

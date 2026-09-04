# X3 Atomic Cross-VM Completion Audit

Auditor: lojak (openclaw) · Evidence gathered 2026-09-03 · No code changes made.
Repo root: /home/lojak/Desktop/xxxstar-main (NOTE: no git history in this checkout).
Method: call-path tracing + repo-wide grep over workspace members and live wiring (runtime, node service), not existence checks.

---

## 1. Executive Verdict

**MOSTLY COMPLETE, NOT FULLY INTEGRATED** — and **NOT PRODUCTION READY for public testnet** on the external-chain legs.

Short reason: The X3-native atomic kernel (pallet `x3-atomic-kernel`), cross-VM router, and the *actually wired* node-side X3VM path (`RuntimeCrossVmDispatcher` → `SubstrateX3VmBridge`) are real and integrate into the runtime and node service behind a safety gate. But the external EVM/SVM/BTC/UTXO settlement legs run through **in-memory simulation/mock HTLC adapters** (`x3-atomic-swap`, `crates/x3-bridge`) whose proof data and tx ids are placeholders, and the standalone `x3-crosschain-gateway` crate is **excluded from the build** (its pallet is wired, the external-bridge crate is not). No external chain executes for real in production. That combination is "fully coherent internally, not fully integrated externally."

---

## 2. Integration Map

| Component | File path(s) | Main structs/functions | Who calls it | What it calls | Wired into runtime/service/API/tests? | Evidence |
|---|---|---|---|---|---|---|
| Atomic kernel (rolling atomic execution + PoAE) | `pallets/x3-atomic-kernel/src/lib.rs` (`submit_atomic_bundle`, `assign_bundle_executor`, `finalize_atomic_bundle`, `record_flash_finality_anchor`, `rollback_atomic_bundle`, `record_leg_execution_receipt`, `NonceRegistry`, `Bundles`, `FinalityCertAnchors`) | `pallet_x3_atomic_kernel::Pallet` | Runtime constr. call; off-chain executor submits unsigned | `vm_revert::CompositeReverter`, `x3-vm` | YES — runtime `impl Config` (runtime/src/lib.rs:2649), configured in all 5 runtime config spots; in construct_runtime | runtime/src/lib.rs:2649, 487,3158 |
| Cross-VM dispatcher trait | `crates/cross-vm-bridge/src/lib.rs:58` `CrossVmDispatcher` | trait w/ execute_evm/svm/x3vm, escrow, balances | bridge exec path | impl'd by LiveNodeDispatcher + RuntimeCrossVmDispatcher | trait in build; concrete prod impl below | lib.rs:58 |
| **Production dispatcher (wired)** | `crates/x3-bridge-adapters/src/lib.rs:537` `RuntimeCrossVmDispatcher` | real runtime-API calls: `submit_evm_transaction`, `is_svm_program`+`submit_svm_instruction`, `X3VMBridge.execute`, `get_*_balance/escrow` | node service poller | runtime API (`AtlasKernelRuntimeApi`) | YES — constructed in node/src/service.rs poller | x3-bridge-adapters/src/lib.rs:537-660; node/src/service.rs:1460 |
| Cross-VM bridge state machine | `crates/cross-vm-bridge/src/lib.rs` `CrossVmBridge`, `execute_pending_with_dispatcher` | bridge op lifecycle, two-phase commit, per-session nonce/proofs | node poller (`cross-vm-bridge-poller`) | `RuntimeCrossVmDispatcher` + `CrossVmBridgeSafetyGate` | YES — node/src/service.rs:1460-1540 | service.rs; lib.rs:1700,1185 |
| Safety gate / dispute | `node/src/service.rs:675` `CrossVmBridgeSafetyGate` (preflight/postflight, pause, `open_dispute`) | gate around each poller batch | bridge poller | → pause + dispute status | YES | service.rs:675-740,1495-1530 |
| X3VM bridge + escrow persistence | `node` `SubstrateX3VmBridge::with_persistence`, `OffchainEscrowPersistence` | real on-chain/offchain storage | dispatcher `.with_x3vm_bridge` | runtime | YES — node/src/service.rs ~1446 | service.rs |
| Cross-VM router pallet | `pallets/x3-cross-vm-router` `Transfers` | router | runtime | → adapters | YES — runtime impl Config 2342 + in construct_runtime; runtime tests read `Transfers` iter | runtime/src/lib.rs:2342,4020 |
| EVM/SVM/X3VM/Cairo/BTC/etc HTLC adapters | `crates/x3-atomic-swap/src/{evm_htlc,svm_htlc,x3vm_htlc,bitcoin_htlc,cairo_vm_htlc,...}.rs` | HTLC lock/claim/refund + mock tx_id/proof | intent/relayer engines | **in-memory simulation only** | Built (workspace member) but **simulated**, not live exec | workspace member line 103/193; per-file headers "simulates on-chain behavior", "mock", "placeholder" |
| HTLC/bridge for external chains | `crates/x3-bridge/src/{bitcoin_htlc,ethereum_bridge,l2_bridge}.rs` | `BitcoinHTLC` (create/redeem/refund/verify_btc_tx) | `crates/x3-atomic-swap/src/lib.rs` (imports asset) | mock/placeholder proofs | crate is a member; BTC module is real script logic but proof verify is placeholder | Cargo.toml:147; x3-atomic-swap/src/lib.rs |
| Atomic swap orchestration (off-chain) | `crates/x3-atomic-swap/src/{intent,ledger,registry,relayer,timeout,scoreboard,finality,dispute}.rs` | `AtomicIntent`, `TimeoutEngine`, proof ledger, relayer | node/src/service.rs and tests | per-chain HTLC adapters (simulated) | Only `node/src/service.rs` references it; see 6/8 for wiring depth | node/src/service.rs text match |
| External crosschain gateway | `pallets/x3-crosschain-gateway` (wired) **+** `crates/x3-crosschain-gateway` (**excluded**) | gateway pallet register/deposit/withdraw/release proofs | runtime | pallet wired; crate NOT built | PARTIAL — pallet in construct_runtime; **crate commented out of members** (build) | runtime/src/lib.rs:2461; Cargo.toml:161 comment; crates/x3-crosschain-gateway has no own [workspace] and isn't a member → not compiled anywhere |
| RPC | `crates/x3-rpc` | routes to `pallet-x3-crosschain-gateway` | API surface | graph | crate depends on the gateway **pallet** (via path) only | x3-rpc/Cargo.toml:57 |

---

## 3. End-to-End Atomic Flow (EVM → X3VM → SVM → BTC)

Because the four external legs are served by *simulated* adapters, there is no single reachable production E2E path that executes on EVM, then X3VM, then SVM, then BTC. Cross-VM execution is real **only on the X3VM/Atlas leg** (runtime API path). The EVM/SVM/BTC legs stop at mock proof/tx-id generation.

Trace of the X3VM leg that IS production-real (steps that are real in bold, simulated/absent in `[sim]`/`[ABSENT]`):

1. **Entry point**: node `cross-vm-bridge-poller` loop (`node/src/service.rs:1460`) → `CrossVmBridgeSafetyGate::preflight`.
   - Validation of current view: pause/do-not-execute if not fully synced or repeated failures. ✅ proven gate.
2. **Trigger execution**: `b.execute_pending_with_dispatcher(RuntimeCrossVmDispatcher)`.
3. **Dispatcher dispatch**: `cross-vm-bridge` bridge state machine resolves each pending op to EVM/SVM/X3VM. `RuntimeCrossVmDispatcher` methods (`x3-bridge-adapters/src/lib.rs`):
   - EVM leg → `api.submit_evm_transaction(at, payload)` (runtime). ✅ real call.
   - SVM leg → `is_svm_program` gate then `api.submit_svm_instruction(at, program_id, input)`. ✅ real call.
   - X3VM leg → `X3VMBridge.execute(payload, fn_index, [])`. ✅ real execution engine path.
4. **Validation (message/version)**: `call.ensure_current_version()` + target check (both in trait contract). ✅
5. **Message creation**: internal `CrossVmCall` construction; call_hash derived. Partial — no external chain message; `H256::zero()` used for state roots.
6. **Dispatch**: yes on X3VM (bridge.execute). 
7. **Execution**: real for X3VM runtime API + bridge; **[sim]** for EVM/SVM/BTC contracts in `x3-atomic-swap` adapter layer (in-memory/injected JSON-RPC proof only; BTC = mock).
8. **Finality/anchor handling**: kernel pallet `record_flash_finality_anchor` + `FinalityCertAnchors` storage, unsigned via `ValidateUnsigned` (off-chain worker path). ✅ on-chain anchor for flash-finality. But anchors are block-num→cert H256 map, not external BFT finality proofs.
9. **Settlement / rollback**: kernel: per-leg executed receipts + `rollback_atomic_bundle` + `vm_revert::CompositeReverter` for executed legs w/ non-empty state diff; atomic-trade-engine checkpoints. ✅ internal rollback semantics. **[sim]** external-chain settlement (mock proofs).
10. **Timeout path**: `x3-atomic-swap` `TimeoutEngine` (REFUNDABLE/REFUNDED transitions) + `intent.is_source/destination_expired`. Real engine but operates on simulated chain state; BTC refund via real script logic but mock broadcast/proofs.
11. **Event/log emission**: kernel/trade-engine `Event` emissions; CrossVmEvent in bridge. ✅ internal. External-chain finality/claim logs are program-constructed mock (e.g. raw_proof `"e?vm\x01"`).
12. **Test that proves it**: A runtime/unit test proving the *production dispatcher on a live external chain* does **not exist** (see §6). The strongest tests prove `RuntimeCrossVmDispatcher` on the runtime API (X3VM leg) and kernel semantics; external-chain settlement is only proven against in-memory simulations.

**Blockers in this flow**: external EVM/SVM/BTC gateway proof/execution is stub/mock; no real end-to-end cross-chain atomic operation is reachable in production.

---

## 4. Completeness Checklist

| Item | Status | Evidence |
|---|---|---|
| Cryptographically secure secret generation (OsRng) | NOT FOUND as primary; `rand::thread_rng`?/OsRng audit needed | Did not locate OsRng in core atomic paths during this pass; see gaps. |
| Global replay protection | PARTIAL | kernel `NonceRegistry` (StorageDoubleMap) is strong; atomic-swap off-chain dedup unclear. |
| Per-session nonce protection | PASS | `NonceRegistry<T>(u32 chain, AccountId) → NonceState`; per-chain/account nonces; `Twox64Concat` + `Blake2_128Concat`. |
| HashSet/MapSet replay storage (not O(n) Vec) | PARTIAL | StorageMap/DoubleMap on-chain = efficient; off-chain atomic-swap ledger stores Vec<records> and does `.iter()`-based checks (not hash-set), flagged. |
| HTLC lock/claim/refund lifecycle | PARTIAL | Real per-chain adapters implement lock/claim/refund logic; operates on in-memory sim state, not live chain. |
| Timeout expiration and cleanup | PARTIAL | TimeoutEngine real; chain-watcher cleanup not proven on live external chains. |
| Dead/expired-session cleanup | PARTIAL | TimeoutEngine transitions; global sweep not evidenced to run against external chains. |
| Max concurrent session limit | NOT FOUND | no cap surfaced in traced path (check MS/LIMITS constants in lib.rs: MAX_BATCH_SIZE=64 only for batch). |
| Dispatcher actually used by bridge execution | PASS | node poller passes `RuntimeCrossVmDispatcher` to `execute_pending_with_dispatcher` (service.rs:1533); production path does NOT use the simulated `NoOpDispatcher` (cfg(test)-gated). |
| No hardcoded stub outputs in production path | FAIL | `crates/x3-bridge` + `x3-atomic-swap` external adapters hardcode mock tx_ids / `raw_proof: [..]` / placeholder addresses / zero escrow. The X3VM leg in `x3-bridge-adapters` is fine, but external legs fake success. |
| `validate_unsigned` coverage for unsigned extrinsics | PARTIAL | kernel `record_flash_finality_anchor`/`submit_finalization_result` validated via `ValidateUnsigned` logic at line 468-511 etc. Only a few unsigned pallet calls (see gaps). |
| weight benchmarks for pallets | PARTIAL | `x3-atomic-kernel`: `benchmarking.rs`, `weights.rs`, and `type WeightInfo` feeds weights for the calls. Runtime: `type WeightInfo = ();` — weights not set to measured impl (see runtime lines 883/902/912), T::WeightInfo used in macro weight. Benchmarking harness exists; CI `frame-benchmarking.yml` present. |
| Runtime pallet integration | PASS | kernel, router, crosschain-gateway pallet all wired into runtime construct_runtime + Config impls + features. |
| node/service integration | PARTIAL→FAIL for externals | X3VM node integration real; external-chain gateway crate excluded from build — not service-integrated. |
| Cross-VM adapter registration | PARTIAL | `x3-atomic-swap` registers per-chain adapters; runtime dispatcher only handles X3VM + runtime API EVM/SVM. Adapters not registered to a single live execution registry in service for external chains. |
| Chain adapter lookup error handling | PARTIAL | unknown target → InternalError; missing adapter → InternalError "bridge not configured". Some `unwrap_or_default`/`unwrap_or(false)` silent fallback in evm/svm lookups. |
| Finality anchor verification | PARTIAL | on-chain anchors recorded & verified for flash finality; no external-signature finality verification against EVM/SVM/BTC in traced path. |
| Rollback/failure semantics | PASS | kernel rollback + CompositeReverter + atomic-trade-engine checkpoint rollback exist + tests. |
| Integration tests | PARTIAL | kernel/trade-engine/mock + `guard_tests.rs`; no external-chain live integration tests. |
| Property/fuzz tests | NOT FOUND in crates scanned (fuzz dirs absent for cross-vm-bridge/x3-atomic-swap/x3-bridge) | no `crates/*/fuzz`. |
| Benchmarks | PARTIAL | ledger benchmark infra present (frame-benchmarking) + kernel benchmarking.rs; runtime WeightInfo = () for several. |
| CI enforcement | PARTIAL→FAIL | `.github/workflows/{full-ci,rust,frame-benchmarking,v04-ship-gate,...}.yml` present and CI covers members including cross-vm/atomic; but because external crosschain-gateway crate is excluded, its code is not CI-built. |

---

## 5. Stub / Mock / Placeholder Hunt

Findings (production-reachable unless noted):

- `crates/x3-atomic-swap/src/evm_htlc.rs:8,73,77,199` — "adapter simulates on-chain behavior"; "in-memory HTLC contract state for testing and simulation"; "placeholder selector for simulation"; `mock_tx_id` at :559 used in real logic path. **Production-reachable** (not cfg(test)); impact: no live EVM lock/claim.
- `crates/x3-atomic-swap/src/svm_htlc.rs:94,598,651,676` — "In-memory SVM ... for testing and simulation"; mock proofs `raw_proof: vec![0x73,0x76,0x6d,0x01]` ("svm\x01"); "svm\x02". **Production-reachable.**
- `crates/x3-atomic-swap/src/x3vm_htlc.rs:97,174,207,229` — mock proof bytes `"x3vm\x01/x2"`. impact: hybrid path x3vm proof fabricated.
- `crates/x3-atomic-swap/src/bitcoin_htlc.rs:4,67,110,222,285` — "mock/placeholder proof structures"; "placeholder script"; "builder for constructing mock Bitcoin transactions"; real-script logic but "Uses mock/placeholder proof data. In real operation this would connect to a...". Impact: BTC leg not live.
- `crates/x3-bridge/src/bitcoin_htlc.rs` — real HTLC script/preimage/timeout logic but `verify_btc_tx` (154) relies on passed-in proof; no live node/SPV wiring found.
- `crates/x3-bridge-adapters/src/lib.rs` + `crates/x3-bridge/src` escrow/zero returns in connector `LiveNodeDispatcher` (escape) — `get_evm_bridge_escrow`/`get_svm_bridge_escrow` return `[0u8;20]/[0;32]` + log warn in `connector.rs`. NOT the wired production dispatcher (RuntimeCrossVmDispatcher is), which reads escrow via runtime API — but any code calling `LiveNodeDispatcher` (e.g. offline/`create_default_dispatcher`) gets zero escrow. Flagged.
- `CrossVmReceipt` construction reuses `H256::zero()` state roots in real code (both dispatchers) — legitimately a placeholder for state-root attestation. Impact: no authenticated state root cross-chain.
- `NoOpDispatcher` (`cross-vm-bridge/src/lib.rs:295`) — cfg(test)-gated; **not** production-reachable. Good (PASS).
- `StubKernelDispatcher` (lib.rs:3511) — in `#[cfg(test)]` modules only. (PASS.)

No `todo!`/`unimplemented!`/`panic!` surfaced in the traced core executor beyond doc-comment "not yet wired".

---

## 6. Test Evidence

Readily located relevant tests:
- `x3-atomic-kernel/src/tests.rs` + `mock.rs` — kernel submit/finalize/assign/rollback + `vm_revert`; includes `#[cfg(test)]` bundles tests using mock runtime config. Proves: kernel semantics (rollback, per-leg receipts, finality anchor, dedup via NonceRegistry) in a mock runtime. Does NOT prove: external-chain execution.
- `x3-atomic-kernel/src/benchmarking.rs` (+ weights.rs) — benchmarks to compute weights; does not prove live external behavior.
- `cross-vm-bridge` — 84 `#[test]` functions across `crates/cross-vm-bridge/src/lib.rs` (+ `tests/guard_tests.rs` 15 tests), many call `execute_pending_with_dispatcher(&NoOpDispatcher::testnet())` — proves state-machine/guard logic against a simulated dispatcher. Does NOT prove production dispatcher on external chains.
- `x3-atomic-swap` HTLC module tests — verify lock/claim/refund + TimeoutEngine transitions. Prove adapter *logic* on mock state; do NOT hit a real node.
- Runtime tests (e.g. reading `pallet_x3_cross_vm_router::Transfers`) prove router storage from a running runtime config.
- `.github/workflows` (full-ci, rust, frame-benchmarking, v04-ship, proof-gates) enforce tests for workspace members. Because the external-gateway crate is NOT a member, it is NOT CI-covered.

Weak tests (prove a simulation/mock, not production):
- Any `x3-atomic-swap` HTLC adapter test that asserts success/failure against in-memory HTLC contracts and mock tx.
- Any `cross-vm-bridge` test that runs execution via `NoOpDispatcher` and concludes an "atomic operation" success.

---

## 7. Commands to Verify

```
cd /home/lojak/Desktop/xxxstar-main
cargo check -p x3-atomic-swap -p x3-bridge -p x3-bridge-adapters -p cross-vm-bridge    # confirm adapters build
cargo test  -p x3-atomic-kernel                                                       # kernel tests
cargo test  -p pallet-x3-atomic-kernel
cargo test  -p cross-vm-bridge
cargo test  -p x3-atomic-swap
cargo clippy --workspace --tests -- -D warnings                                     # (note members only)
# Stub/mock hunt
rg -n "simulat|mock|stub|placeholder|For now|not yet|H256::zero\(\)|raw_proof:|unwrap_or\(false\)|unwrap_or_default\(\)" \
   crates/x3-atomic-swap crates/x3-bridge crates/x3-bridge-adapters crates/cross-vm-bridge pallets/x3-atomic-kernel
# Confirm external gateway crate EXCLUDED from build
grep -n "x3-crosschain-gateway" Cargo.toml
# Fuzz/property presence
find crates -maxdepth 2 -type d -name fuzz
```
Benchmark/fuzz binaries: none shipped for the external adapters (`crates/*/fuzz` absent). The `frame-benchmarking` paths require the feature build in CI.

---

## 8. Blockers (prevent testnet release)

1. **External-chain legs are simulation/mock, not real.** EVM/SVM/BTC/UTXO HTLC lock/claim/refund/proof and tx submission in `x3-atomic-swap` + `x3-bridge` fabricate tx-ids, proofs (`"svm\x01"` etc.), and placeholder addresses. P0 — no real cross-VM settlement with external chains exists.
2. **`crates/x3-crosschain-gateway` external crate is excluded from the workspace/build** (Cargo.toml:161) — its gateway logic is not compiled or CI-tested; only the in-chain pallet is wired. P0 integration gap.
3. **No production execution path reaches any of the 3+ external chains** behind the live dispatcher except via runtime API for EVM/SVM submission to *X3-internal* runtime, which in turn depends on the simulated adapters; X3VM/bridge executes for real but its settlement finality and external proofs are placeholder (`H256::zero()` state roots). 
4. **Replay/seen-set is not a HashSet on the off-chain atomic-swap ledger** (Vec `.iter()`). A production on-chain NonceRegistry exists and is good, but the external adapter ledger path may be O(n).

---

## 9. Gaps (private devnet ok; block public testnet/mainnet)

- Weight integration: several runtime `Config` set `type WeightInfo = ();` — no measured pallet weights on the executable path (only kernel has real `T::WeightInfo` usage). P1 for public testnet benchmarking.
- `OsRng`/entropy source not confirmed for secret/preimage generation in atomic-swap/HTLC code; SHA-256 expected but verify source. 
- Max concurrent session / queue depth cap not found for `atomic-swap` coordinator.
- Full `validate_unsigned` / unsigned-extrinsic audit beyond the 2 traceable kernel calls not done.
- Finality anchor is flash-finality-internal, not external BFT-verified across EVM/SVM/BTC/UTXO.
- Runtime `WeightInfo = ()` for kernel? line 883 is a `T::WeightInfo::…` weight baked into pallet (actually T::WeightInfo set inside pallet code at dispatchable macro, which may substitute `()` from runtime at compile); verify with the actual benchmark output. (Standard substrate: `()` at runtime means no precomputed weights but runtime `.weight(..)` is compile-time pallet macro — depends whether pallet uses skeleton weights. Flag as must-confirm.)
- No fuzz/property tests anywhere on the external adapter settlement invariants.

---

## 10. Required Fix Plan (sequenced; no files changed in this audit)

Phase 1 – Correctness blockers (P0)
- Replace mock/sim external adapters (`x3-atomic-swap/{evm_htlc,svm_htlc,bitcoin_htlc,x3vm_htlc}.rs`) with live-chain clients + real proofs; remove hardcoded `mock_tx_id`/`raw_proof:"..\x01"` and `H256::zero()` state roots, using genuine tx ids and attested state roots.
- Build & CI the `crates/x3-crosschain-gateway` crate: add to workspace members (Cargo.toml) or give it a standalone `[workspace]`, then rewire node/service to use it, and CI-enable.
- Add real external-chain finality proof verification (n client headers/BFT message + quorum) to finality anchors.

Phase 2 – Integration blockers (P0/P1)
- Register one live execution registry in node where real EVM/SVM/BTC adapters are selected for pending bridge ops (gate via existing `CrossVmBridgeSafetyGate`), remove reliance on Zero escrow (enforce funded escrow check not just zero-address warn).
- Wire `bitcoin_htlc` SPV/verification to a real BTC node/parser.

Phase 3 – Tests / benchmarks (P1)
- Integration tests against a running devnet hitting **real** EVM/SVM/BTC nodes (mocknet/spire), keep current NoOp tests as unit-only.
- Add Set-based replay proof tests + property/fuzz for atomic invariants (deadlock-free timeout ordering, no double-claim). Ship a fuzz target under `x3-atomic-swap/fuzz`.
- Standardize runtime WeightInfo to generated benchmarks.

Phase 4 – CI / release gates (P1/P2)
- Cover external crate in CI; gate testnet on Phase 1+2 green plus security review; document external-chain session limit + entropy/finality requirements.

---
Every claim above cites a file path/symbol. Where something was missing, it is marked NOT FOUND rather than assumed.

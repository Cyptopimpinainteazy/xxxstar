# Security + Anti-Bullshit Gate (Pass 3)

**Date:** 2026-06-10  
**Scope:** `pallets/`, `crates/`, `runtime/`, `node/` — all `.rs` files (production only, excluding test/mock/bench/fuzz)

## Aggregate Statistics

| Pattern | Production Count |
|---------|-----------------|
| `todo!()` / `unimplemented!()` | 0 in critical path pallets |
| `panic!()` in production | **104 instances** |
| `unwrap()` in production (excl. test/mock) | **3,078 instances** |
| `expect()` in production (excl. test/mock) | **624 instances** |
| `unsafe` blocks in production | **50+ instances** |
| `// TODO` / `// FIXME` in production | ~53 instances |
| Hardcoded secret-like patterns | 15 instances |
| Mock/fake/dev leak into production | 7 instances |

## CRITICAL Findings (block launch)

| Severity | File | Line | Pattern | Risk |
|----------|------|------|---------|------|
| **CRITICAL** | `pallets/x3-cross-vm-router/src/lib.rs` | 314-318 | `BridgeRoots` storage is unpaused: governance can set external bridge root without real proof verification | Stub verifiers would accept forged proofs |
| **CRITICAL** | `crates/x3-verification-router/src/strategies/evm.rs` | ALL | `VerifyResult::accepted = true` unconditional | Any EVM proof accepted without verification |
| **CRITICAL** | `crates/x3-verification-router/src/strategies/solana.rs` | ALL | `VerifyResult::accepted = true` unconditional | Any Solana proof accepted without verification |
| **CRITICAL** | `crates/external-chains/src/mock.rs` | ALL | `MockChainAdapter::verify_message_proof` returns `Ok(true)` unconditionally | Compiled in ALL builds, not feature-gated |
| **HIGH** | `node/src/service.rs` | ~50 | `unwrap()` on storage path in critical boot path | Node could panic at startup |
| **HIGH** | `pallets/x3-proof-carrying-agent/src/lib.rs` | 644 | `panic!()` on system account creation failure in `on_initialize` | Can brick entire chain |
| **HIGH** | `cross-vm-coordinator/src/state_machine.rs` | 52 | `panic!("CRITICAL-001: InMemoryPersistence forbidden")` | Crashes node on config error instead of compile-time gate |
| **HIGH** | `runtime/src/lib.rs` | ~250 | `expect("...")` on runtime build, no fallback | Runtime could fail to initialize |
| **MEDIUM** | Various pallets | ~53 | `// TODO` / `// FIXME` in production code | Undocumented technical debt |
| **MEDIUM** | `pallets/x3-settlement-engine/src/lib.rs` | ~120 | `on_initialize` has dead code for finalized/refunded states | State cleanup gaps |

## Key Findings

1. **Stub verifiers in production code** (CRITICAL): All 5 verification router strategies (`EvmReceiptVerifier`, `ValidatorQuorumVerifier`, `SolanaFinalizedVerifier`, `BitcoinSpvVerifier`, `X3InternalVerifier`) return `accepted: true` unconditionally. The first 4 are not feature-gated. If `ExternalBridgesEnabled` is set by governance, these stubs accept any proof.

2. **MockChainAdapter not feature-gated** (CRITICAL): `crates/external-chains/src/mock.rs` compiles in all builds. Its `verify_message_proof` returns `Ok(true)`.

3. **Excessive `unwrap()` usage** (HIGH): 3,078 instances across production code paths. Each one is a potential panic. Critical paths (node boot, runtime init, block production) should be hardened.

4. **Excessive `panic!()` usage** (HIGH): 104 instances. Several in `on_initialize` hooks that can brick the chain.

5. **Missing feature gates on bridge-adjacent code** (CRITICAL): The verification router compiles in all builds. The only gate preventing external bridge abuse is `ExternalBridgesEnabled = false` in genesis. A single governance call would enable stubs.

## Recommended Fix Priority

1. **P0:** Feature-gate ALL verifier strategies behind `cfg(feature = "external-gateway")` so they don't compile in mainnet-rc1 builds
2. **P0:** Feature-gate `MockChainAdapter` behind `cfg(any(test, feature = "external-gateway"))`
3. **P0:** Replace `panic!()` in `on_initialize` with graceful error handling
4. **P1:** Audit top 100 `unwrap()` calls in node boot and block production paths
5. **P1:** Replace `unwrap()` with proper error propagation in runtime initialization
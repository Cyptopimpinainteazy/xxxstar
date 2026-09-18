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

---

## Status update — 2026-09-18

Findings 1 and 5 above (stub verifiers, missing feature gates) were the same root
cause and are resolved in behaviour, not documentation:

| finding | status | evidence |
| --- | --- | --- |
| 1 — stub verifiers accept any proof (CRITICAL) | **resolved** | see below |
| 5 — verification router not feature-gated | **superseded** | fail-closed default; permissive path requires a feature that cannot coexist with `production` (compile-time guard) |
| 2 — `MockChainAdapter` not feature-gated (CRITICAL) | **unchanged** | still compiles in all builds |
| 3 / 4 — `unwrap()` / `panic!()` counts (HIGH) | **unchanged** | 3,078 / 104 at last count |

**Finding 1, per strategy:**

- `EvmReceiptVerifier`, `ValidatorQuorumVerifier`, `SolanaFinalizedVerifier` — the
  permissive behaviour is gone (#265). They return
  `VerificationError::NotImplemented` unless the `test-verifier` feature is
  enabled, and `test-verifier + production` is a `compile_error!`. The real EVM
  verifier (`evm_receipt::ProductionEvmReceiptVerifier`) was already implemented.
- `SolanaFinalizedVerifier` — now performs real verification (#268): Ed25519
  attestations over `BLAKE2b-256(slot_le || blockhash)`, restricted to a
  governance-controlled validator set with a per-chain threshold (#271). With no
  set installed it refuses every proof, in every feature configuration.
- `BitcoinSpvVerifier` — was never a stub: it checks header-chain linkage
  (`sha256d`), the merkle proof against the last header's root, and the
  confirmation count. What it did *not* do was enforce the vault-signer policy it
  documented; those fields are removed (see below).
- `X3InternalVerifier` — an internal-only pass-through by design (the kernel is
  the proof for X3-internal transfers); it is registered only for
  `VerificationStrategy::X3Internal`.

The repository's own acceptance test for this finding,
`audit-artifacts/mainnet-readiness/2026-09-05-6a24d8cf-audit/audit-harness/proof`
(which builds the router with `features = ["production"]`), **failed 3/3 before
these changes and passes 3/3 now**.

**Finding 5 is superseded rather than implemented as written.** The strategies
are not gated behind a new `external-gateway` feature; instead the stubs fail
closed by default, so a governance call that enables `ExternalBridgesEnabled` no
longer exposes an accept-anything path. `test-verifier` exists for plumbing tests
and cannot be combined with `production`.

**#267 — `BitcoinSpvVerifier`'s `vault_threshold` / `vault_total_signers`**
claimed that SPV-verified deposits must be backed by that many vault signers and
were never read. They have been removed. Counting approvals would not have made
the claim true: `BtcVault::add_signer_approval` stores signature bytes **without
verifying them**.

**New finding (#272, HIGH)** — `BtcVault::add_signer_approval` accepts
`(signer_pubkey, signature)` for any signer in `config.signers` and increments the
approval count without verifying the signature; at `threshold` approvals the
deposit becomes `Approved`. Today `BtcVault` has no consumers outside its own
crate, and the verification router no longer claims a vault-signer policy, but
the API invites treating stored approvals as consent. The method signature now
names the parameter `unverified_signature_bytes` and the docs state the
requirement; a real scheme (canonical message + verification) is tracked in #272.

---

## Status update — 2026-09-18 (later)

**Finding 2 — `MockChainAdapter` compiled in all builds (CRITICAL) — resolved.**

The type now lives behind `#[cfg(any(test, feature = "mock-adapters"))]` in
`crates/external-chains/src/adapter.rs` and the crate declares a non-default
`mock-adapters` feature. Proven by a downstream probe: a crate depending on
`x3-external-chains` with default features fails with
`error[E0432]: unresolved import x3_external_chains::adapter::MockChainAdapter`
("no `MockChainAdapter` in `adapter`"), and compiles once the feature is enabled.
In-crate tests still use the double, since `cfg(test)` enables it.

Reaching that proof required fixing why the crate could not be built at all: it
was in neither `members` nor `exclude`, so nothing compiled it. It is a workspace
member now, its 62 tests run in `cargo test --workspace`, and the same repair
brought `cross-chain-position-manager` (51 tests) and `custody-service`
(14 tests) in — the latter had two hard compile errors, an ungated `MockHSM`
import in `service.rs` and a missing `Sha256` path in `hsm.rs`.

**New finding (#274, HIGH)** — 59 crates sit in that same members/exclude gap, so
their code and tests are compiled nowhere. `scripts/check-workspace-membership.py`
is now a local-CI gate with a checked-in baseline
(`.ai/workspace-membership-baseline.txt`) that fails when a crate appears in
neither list or when a repaired one stays in the baseline.

Findings 3 and 4 (`unwrap()` / `panic!()` counts) remain open and are unchanged
by either update.

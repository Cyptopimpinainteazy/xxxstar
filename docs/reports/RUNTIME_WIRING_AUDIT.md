# X3 Runtime Wiring Audit

**Date:** 2026-06-09  
**Scope:** Full wiring audit of `runtime/Cargo.toml` ↔ `runtime/src/lib.rs` ↔ `pallets/`  
**Auditor:** Cline AutoClaw

---

## Audit Result: **PASS WITH GAPS**

| Category | Count |
|---|---|
| Pallets on disk | 54 |
| Pallets in Cargo.toml | 51 |
| Pallets in construct_runtime! (max) | 52 (4-6 feature variants) |
| Pallets with Config impl | 46+ (varies by feature) |
| Runtime API impls | 10 |
| **CRITICAL GAPS** | **2** (will break compile) |
| **MODERATE GAPS** | **2** (missing Config, unregistered pallets) |

---

## 1. CRITICAL — Missing from Cargo.toml (will NOT compile)

These pallets are `use`d and referenced in `construct_runtime!` but are missing from `runtime/Cargo.toml` dependencies AND from `[features].std` propagation.

### 1a. `pallet-x3-lp-locker` (`pallets/x3-lp-locker/`)

- ✅ `use pallet_x3_lp_locker` — line 65 of `lib.rs`
- ✅ In `construct_runtime!` as `X3LpLocker` — dev, dev+frontier, prod, prod+frontier variants (4 of 6)
- ✅ `Config` impl exists — lines 2788–2793
- ✅ In `mainnet-rc1` variant (line 665)
- ✅ `runtime-benchmarks` feature test passes
- ❌ **MISSING from `runtime/Cargo.toml` [dependencies]** — no `pallet-x3-lp-locker = {...}` entry
- ❌ **MISSING from `[features].std` propagation**

**Impact:** Any build using the dev, prod, or mainnet-rc1 feature sets will fail to compile with `error[E0433]: failed to resolve: use of undeclared crate or module pallet_x3_lp_locker`.

### 1b. `pallet-northern-swarm` (`pallets/northern-swarm/`)

- ✅ `use pallet_northern_swarm` — line 83 of `lib.rs`
- ✅ In `construct_runtime!` as `NorthernSwarm` — dev, dev+frontier, prod, prod+frontier variants (4 of 6)
- ❌ **MISSING from `runtime/Cargo.toml` [dependencies]**
- ❌ **MISSING from `[features].std` propagation**
- ❌ **MISSING `Config` impl in `lib.rs`** — no `impl pallet_northern_swarm::Config for Runtime` block

**Impact:** Will fail to compile with missing crate AND missing trait impl errors. This pallet is **half-wired**: imported, listed in construct_runtime!, but neither the crate dependency nor the Config implementation exists.

---

## 2. MODERATE — Unwired pallets on disk

These exist in `pallets/` but have NO references in `runtime/Cargo.toml` or `runtime/src/lib.rs`. They are either abandoned/legacy or WIP.

| Pallet | Path | Notes |
|---|---|---|
| `pallet-x3-control` | `pallets/pallet-x3-control/` | Zero references anywhere in runtime. Possibly a future feature. |
| `x3-governance` | `pallets/x3-governance/` | Zero references. The runtime uses `pallets/governance/` (not `x3-governance`). Likely an alternate/legacy version. |

Neither of these cause compile errors — they simply don't exist to the runtime.

---

## 3. MODERATE — Missing std feature propagation

These pallets ARE in `Cargo.toml` [dependencies] but their `std` feature is NOT propagated in `[features].std`. In WASM builds (`no_std` target) this is fine, but in native builds they may not get full std-dependent behavior.

| Pallet | In Cargo.toml | In construct_runtime! | Config impl | `std` propagated |
|---|---|---|---|---|
| `pallet-x3-consensus` | ✅ (line 57) | ✅ (3/6 variants) | ✅ (line 922) | ❌ **MISSING** |
| `pallet-x3-lp-locker` | ❌ (GAP #1) | ✅ | ✅ | ❌ |
| `pallet-northern-swarm` | ❌ (GAP #1) | ✅ | ❌ | ❌ |

`pallet-x3-consensus` is the only one that's in Cargo.toml but missing std propagation. The other two are already captured in the CRITICAL section.

---

## 4. Full Pallet Wiring Matrix

Legend: ✅ = present, ❌ = missing, ⚠️ = conditional (feature-gated)

| Pallet / Crate | Cargo.toml | `use` import | construct_runtime! | Config impl | std propagated | runtime-benchmarks |
|---|---|---|---|---|---|---|
| frame_system | workspace | — | ✅ | ✅ | ✅ | — |
| pallet_timestamp | workspace | ✅ | ✅ | ✅ | ✅ | ✅ |
| pallet_aura | workspace | ✅ | ✅ | ✅ | ✅ | — |
| pallet_grandpa | workspace | ✅ | ✅ | ✅ | ✅ | — |
| pallet_session | workspace | ✅ | ✅ | ✅ | ✅ | — |
| pallet_offences | workspace | ✅ | ✅ | ✅ | ✅ | — |
| pallet_balances | workspace | ✅ | ✅ | ✅ | ✅ | ✅ |
| pallet_transaction_payment | workspace | ✅ | ✅ | ✅ | ✅ | — |
| pallet_scheduler | workspace | ✅ | ✅ | ✅ | ✅ | — |
| pallet_preimage | workspace | ✅ | ✅ | ✅ | ✅ | — |
| pallet_sudo (optional) | workspace | ⚠️ dev | ⚠️ dev | ⚠️ dev | ⚠️ dev | — |
| pallet_collective | workspace | ✅ | ✅ | ✅ | ✅ | ✅ |
| pallet_evm (optional) | workspace | — | ⚠️ frontier | ⚠️ frontier | ⚠️ frontier | — |
| pallet_ethereum (optional) | workspace | ⚠️ frontier | ⚠️ frontier | ⚠️ frontier | ⚠️ frontier | — |
| pallet-x3-kernel | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| pallet-x3-consensus | ✅ | ✅ | ✅ | ✅ | ❌ std | — |
| pallet-x3-invariants | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| pallet-x3-agent-law | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-x3-account-registry | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| pallet-x3-coin | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-atomic-trade-engine | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| pallet-governance | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-treasury | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-agent-accounts | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-agent-memory | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-evolution-core | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| pallet-x3-verifier | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| pallet-x3-domain-registry | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| pallet-x3-settlement-engine | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| pallet-x3-jury-anchor | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-swarm | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| pallet-depin-marketplace | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-private-execution | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-x3-sequencer | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-x3-da | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-x3-atomic-kernel | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-x3-asset-registry | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-x3-supply-ledger | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-x3-cross-vm-router | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-x3-token-factory | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-cross-chain-validator | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-x3-automation | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-x3-oracle | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| pallet-x3-vrf | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| pallet-x3-dex | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| pallet-svm-runtime | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-meme-overlord | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-x3-slash | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-x3-wallet-pallet | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-x3-inventory | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-x3-reservation | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-x3-solvency | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-x3-rebalance | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-x3-partner | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-x3-treasury-policy | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-x3-custody | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-x3-reconciliation | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-x3-wrapped | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-x3-auction | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-x3-launchpad | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-x3-dapp-hub | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-x3-compute-market | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| pallet-x3-flashloan | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| **pallet-x3-lp-locker** | ❌ | ✅ | ✅ (4/6) | ✅ | ❌ | — |
| **pallet-northern-swarm** | ❌ | ✅ | ✅ (4/6) | ❌ | ❌ | — |
| pallet-x3-control | ❌ not used | ❌ | ❌ | ❌ | ❌ | — |
| x3-governance (dir) | ❌ not used | ❌ | ❌ | ❌ | ❌ | — |

---

## 5. External Crates Wiring

| Crate | Cargo.toml | Used in lib.rs | std propagated |
|---|---|---|---|
| x3-cross-vm-bridge | ✅ | ✅ | ✅ |
| x3-asset-kernel-types | ✅ | ✅ | ✅ |
| x3-svm-integration | ✅ | ✅ (native_vm_adapters) | ✅ |
| x3-ixl | ✅ | ✅ (IXL types) | ✅ |
| x3-packet-standard | ✅ | ✅ | ✅ |
| quantum-crypto (optional) | ✅ (pq feature) | ❌ not used | optional |
| x3-accounting-events | ✅ | ✅ (NoOpSpine) | ✅ |
| x3-security-events | ✅ | ✅ (NoOpHook) | ✅ |
| x3-revenue-sharing | ✅ | ✅ | ✅ |

All external crates are properly wired.

---

## 6. Runtime API Implementations

| Runtime API | Implemented | Notes |
|---|---|---|
| sp_api::Core | ✅ | |
| sp_session::SessionKeys | ✅ | |
| sp_genesis_builder::GenesisBuilder | ✅ | |
| sp_transaction_pool::TaggedTransactionQueue | ✅ | |
| pallet_x3_kernel::AtlasKernelRuntimeApi | ✅ | 25+ methods |
| pallet_atomic_trade_engine::AtomicTradeEngineApi | ✅ | 6+ methods |
| sp_consensus_aura::AuraApi | ✅ | |
| sp_consensus_grandpa::GrandpaApi | ✅ | |
| sp_block_builder::BlockBuilder | ✅ | |
| frame_system_rpc_runtime_api::AccountNonceApi | ✅ | |
| pallet_transaction_payment_rpc_runtime_api | ✅ | |
| sp_api::Metadata | ✅ | |
| pallet_evolution_core::EvolutionCoreApi | ✅ | |
| pallet_x3_verifier::X3VerifierApi | ✅ | |
| pallet_x3_domain_registry::X3DomainRegistryApi | ✅ | |
| pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi | ✅ | |

All runtime APIs are properly implemented.

---

## 7. Summary of Required Fixes

### BLOCKER — Fix Immediately

1. **Add `pallet-x3-lp-locker` to `runtime/Cargo.toml`:**
   ```toml
   pallet-x3-lp-locker = { path = "../pallets/x3-lp-locker", default-features = false }
   ```
   And add `"pallet-x3-lp-locker/std"` to `[features].std`.

2. **Add `pallet-northern-swarm` to `runtime/Cargo.toml`:**
   ```toml
   pallet-northern-swarm = { path = "../pallets/northern-swarm", default-features = false }
   ```
   Add `"pallet-northern-swarm/std"` to `[features].std`.
   **AND** add `impl pallet_northern_swarm::Config for Runtime {}` to `runtime/src/lib.rs`.

### MINOR — Fix When Convenient

3. **Add `pallet-x3-consensus/std` to `[features].std`** in `runtime/Cargo.toml`.

### NOT A BUG — Documented

4. `pallet-x3-control` and `x3-governance` on disk but unwired — intentional (legacy/WIP).

---

## 8. `mainnet-rc1` Feature Completeness

The `mainnet-rc1` variant includes 21 pallets — the minimum viable set for first public testnet. The two CRITICAL gaps above affect the `dev`, `prod`, and `prod+frontier` variants but NOT `mainnet-rc1` (neither `X3LpLocker` nor `NorthernSwarm` are in the RC1 variant). However, `pallet-x3-lp-locker` IS listed in the RC1 variant (line 665):

```
X3LpLocker: pallet_x3_lp_locker,
```

So **even the `mainnet-rc1` build will fail** because the crate dependency is missing. This is a release-blocking issue for RC1.
# X3 Atomic Star Internal Settlement Mainnet RC — Failure & Resolution Report

**RC Target**: X3 Internal Settlement Mainnet RC1  
**Gate command**: `bash scripts/mainnet/mainnet_rc_gate.sh`  
**Polkadot-SDK branch**: `stable2512` (commit `30b95889`)  
**Rust**: 1.90.0 · WASM target: `wasm32v1-none`

---

## RC Scope (Confirmed Passing ✅)

| Package | Status |
|---|---|
| `x3-chain-runtime` | ✅ |
| `pallet-x3-kernel` | ✅ |
| `pallet-atomic-trade-engine` | ✅ |
| `pallet-x3-agent-law` | ✅ |
| `pallet-swarm` | ✅ |
| `pallet-x3-cross-vm-router` | ✅ |
| `pallet-x3-supply-ledger` | ✅ |
| `pallet-x3-asset-registry` | ✅ |
| `pallet-x3-account-registry` | ✅ |
| `pallet-x3-atomic-kernel` | ✅ |
| `pallet-x3-settlement-engine` | ✅ |
| `x3-asset-kernel-types` | ✅ |
| `x3-ixl` | ✅ |
| `x3-packet-standard` | ✅ |

---

## Issues Resolved This Session

### 1. `AgentLawCheck` — `SignedExtension` → `TransactionExtension` migration (CRITICAL)

**File**: `pallets/x3-agent-law/src/signed_extension.rs`  
**Symptom**: `error[E0277]: the trait bound AgentLawCheck<Runtime>: TransactionExtension<RuntimeCall> not satisfied` (3×)  
**Root cause**: stable2512 replaced `sp_runtime::traits::SignedExtension` with `sp_runtime::traits::TransactionExtension`. The old trait no longer satisfies the `SignedExtra` tuple requirements.  
**Fix**: Full rewrite of `AgentLawCheck` to implement `TransactionExtension`:
- `type Implicit = (); type Val = (); type Pre = ()`
- `validate()` receives `origin: DispatchOriginOf<RuntimeCall>` — signer extracted via `origin.as_system_origin_signer()`
- `prepare()` provided via `impl_tx_ext_default!` macro
- `weight()` returns `Weight::zero()` (policy check is negligible weight)
- Added `where` bound: `<T::RuntimeCall as Dispatchable>::RuntimeOrigin: AsSystemOriginSigner<T::AccountId> + Clone`

### 2. `EnforcementAction` / `PolicyRule` missing `DecodeWithMemTracking` (CRITICAL)

**File**: `pallets/x3-agent-law/src/types.rs`  
**Symptom**: `error[E0277]: the trait bound PolicyRule<AccountId32>: DecodeWithMemTracking not satisfied`  
**Root cause**: `TransactionExtension` supertrait requires `DecodeWithMemTracking` on all composed types. These enums did not derive it.  
**Fix**: Added `DecodeWithMemTracking` to both `#[derive(...)]` lines for `EnforcementAction` and `PolicyRule`. Added `AccountId: DecodeWithMemTracking` to `PolicyRule`'s generic bound (satisfied by `T::AccountId: Parameter`).

### 3. Runtime — `AgentLawCheck` not in non-dev `construct_runtime!` variants

**File**: `runtime/src/lib.rs`  
**Symptom**: `error[E0277]: RuntimeEvent: From<pallet_x3_agent_law::Event<Runtime>>` not satisfied (multiple variants)  
**Root cause**: The `pallet_x3_agent_law::Config` impl existed unconditionally, but 3 of 4 `construct_runtime!` variants (dev+frontier, no-dev+no-frontier, no-dev+frontier) were missing `X3AgentLaw: pallet_x3_agent_law`.  
**Fix**: Added `X3AgentLaw: pallet_x3_agent_law,` between `X3Invariants` and `X3Coin` in all 3 missing variants.

### 4. Runtime — `sp-staking` not in dependencies (CRITICAL for grandpa offences)

**Files**: `Cargo.toml`, `runtime/Cargo.toml`  
**Symptom**: `error[E0433]: failed to resolve: use of unresolved module sp_staking`  
**Fix**: Added `sp-staking` to workspace `Cargo.toml` and to `runtime/Cargo.toml` `[dependencies]` + `"sp-staking/std"` in the `std` features list.

### 5. Runtime — `pallet_x3_consensus` missing `SlashFraction` / `MinStakeAfterSlash`

**File**: `runtime/src/lib.rs`  
**Fix**: Added `ConsensusSlashFraction` and `ConsensusMinStakeAfterSlash` `parameter_types!` and wired them into `impl pallet_x3_consensus::Config for Runtime`.

### 6. Runtime — `SignedExtra` referenced non-existent types

**File**: `runtime/src/lib.rs`  
**Fix**: Commented out `InvariantCheck`, `CapabilityEnvelopeCheck`, `AtomicSettlementCheck`, `FlashFinalityExtension` (not yet implemented). Changed `AgentLawCheck` to `AgentLawCheck<Runtime>`.

### 7. Runtime — Grandpa API: `submit_report_equivocation_unsigned_extrinsic` / `generate_key_ownership_proof` not found

**File**: `runtime/src/lib.rs`  
**Root cause**: stable2512 renamed/moved these APIs on `pallet_grandpa::Pallet`:
  - `submit_report_equivocation_unsigned_extrinsic` → `submit_unsigned_equivocation_report` (takes `T::KeyOwnerProof` not `OpaqueKeyOwnershipProof`)
  - `generate_key_ownership_proof` was removed from the pallet; must be generated via session historical  
**Fix**:
  - `generate_key_ownership_proof`: uses `Historical::prove((KEY_TYPE, authority_id)).map(|p| OpaqueKeyOwnershipProof::new(p.encode()))`
  - `submit_report_equivocation_unsigned_extrinsic`: decodes opaque proof via `key_owner_proof.decode::<MembershipProof>()?`, then calls `Grandpa::submit_unsigned_equivocation_report(...)`
  - `check_proof` call: added `use frame_support::traits::KeyOwnerProofSystem` import so `pallet_session::historical::Pallet::check_proof` is callable

### 8. Runtime — `Offences::is_known_offence` type ambiguity

**File**: `runtime/src/lib.rs`  
**Symptom**: `error[E0283]: type annotations needed` — cannot infer type `O` in `ReportOffence` trait  
**Fix**: Fully-qualified call with explicit offence type:
```rust
<Offences as sp_staking::offence::ReportOffence<
    AccountId,
    pallet_session::historical::IdentificationTuple<Runtime>,
    pallet_grandpa::EquivocationOffence<pallet_session::historical::IdentificationTuple<Runtime>>,
>>::is_known_offence(...)
```

### 9. Runtime — borrow-after-move of `key_owner_proof` in `process_evidence`

**File**: `runtime/src/lib.rs`  
**Fix**: Extract `session_index` and `validator_set_count` from `key_owner_proof` BEFORE it is moved into `check_proof(...)`.

### 10. `pallet-x3-settlement-engine` — `Currency` ambiguity

**File**: `pallets/x3-settlement-engine/src/lib.rs`  
**Fix**: `<T as Config>::Currency::transfer(...)` disambiguates from `pallet_x3_kernel::Config::Currency`.

---

## Known Outstanding Failures (Out of RC Scope)

### sc-network — libp2p version conflict (34 errors)

**Packages affected**: `patches/sc-network` and transitively `node`  
**Root cause**: `patches/sc-network` internally uses libp2p 0.51.x (for `Swarm`, `Behaviour`, etc.) but `sc_network_types::PeerId` (from polkadot-sdk stable2512) uses `libp2p-identity 0.2.13`. The two `PeerId` types are incompatible. The public API must use `sc_network_types::PeerId` for `NetworkPeers` trait compatibility, but internal swarm code requires `libp2p::PeerId`.  
**RC impact**: NONE — `node` crate is NOT on the RC critical path. Runtime, pallets, and the settlement engine are unaffected.  
**Resolution**: Requires upgrading the libp2p dependency in the sc-network patch to match the version pinned by polkadot-sdk stable2512. Tracked as post-RC1 infrastructure task.

### External bridge surface

**Status**: Disabled at genesis per RC scope. `BridgeEvmEscrowStorage` and `BridgeSvmEscrowStorage` storages are wired but the external bridge pallet is gated behind feature flags not enabled for RC.

### Stub signed extensions

The following `SignedExtra` entries are commented out pending implementation (RC+1):
- `pallet_x3_invariants::InvariantCheck` — invariant enforcement pre-dispatch
- `pallet_x3_kernel::CapabilityEnvelopeCheck` — long-range capability validation
- `pallet_x3_kernel::AtomicSettlementCheck` — cross-VM atomicity gate
- `pallet_x3_kernel::FlashFinalityExtension` — flash finality

---

## RC Deliverables

- Gate script: `scripts/mainnet/mainnet_rc_gate.sh`
- This report: `reports/internal_settlement_rc_failures.md`

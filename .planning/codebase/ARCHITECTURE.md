# Architecture

**Analysis Date:** 2026-05-19

## Pattern Overview

**Overall:** Substrate FRAME blockchain node with multi-VM runtime, x3-lang compiler stack, and off-chain swarm services.

**Key Characteristics:**
- Single Rust workspace at repo root defining the chain binary, runtime WASM blob, and 53 FRAME pallets
- Runtime has four conditional `construct_runtime!` variants (dev/mainnet × frontier-EVM/no-frontier-EVM)
- Universal Asset Kernel (UAK) pattern: cross-VM state invariant enforced via `pallet-x3-supply-ledger` + `pallet-x3-atomic-kernel`
- x3-lang custom language compiler lives in a separate sub-workspace at `.kilo/worktrees/silky-petalite/`
- EVM integration via Frontier (pallet-ethereum + pallet-evm); SVM via `pallets/svm-runtime`
- Off-chain services (gateway, swarm, solvency sidecar) are separate Rust binaries in `crates/` and `services/`

## Layers

**Chain Node:**
- Purpose: Substrate node binary — networking, block production, RPC server, storage
- Location: `node/src/`
- Contains: `service.rs` (node setup, Aura/GRANDPA), `chain_spec.rs` (genesis), `rpc.rs` + `rpc_frontier.rs` (JSON-RPC), `flash_finality.rs`, `main.rs`
- Depends on: `runtime/`, polkadot-sdk, `pallets/`
- Used by: chain operators

**Runtime (WASM):**
- Purpose: State transition function — encodes all business logic as deterministic WASM blob
- Location: `runtime/src/lib.rs`, `runtime/src/precompiles.rs`, `runtime/src/tests.rs`, `runtime/src/fraud_proofs/`
- Contains: 4 `construct_runtime!` blocks; pallet configuration types; FRAME Executive
- Block variants:
  - Lines 403–471: dev + no-frontier (60 pallets)
  - Lines 473–542: dev + frontier (adds EVM/Ethereum/BaseFee pallets)
  - Lines 544–616: mainnet-rc1 + no-frontier
  - Lines 618+: mainnet-rc1 + frontier
- Depends on: all 53 `pallets/*/`, polkadot-sdk FRAME pallets
- Used by: node binary; wasm executor

**Universal Asset Kernel (UAK):**
- Purpose: Cross-VM supply invariant enforcement — canonical_supply == native + evm + svm + external_locked + pending
- Key pallets:
  - `pallets/x3-supply-ledger/` — source of truth for all domain totals; enforces invariant on every state change
  - `pallets/x3-atomic-kernel/` — orchestrates atomic multi-step cross-VM ops with rollback
  - `pallets/x3-asset-registry/` — registers assets, domains, routes, limits
  - `pallets/x3-cross-vm-router/` — initiates/settles cross-VM transfers; nonce-based replay protection
  - `pallets/x3-token-factory/` — creates OmniTokens registered across all three VMs simultaneously
  - `pallets/x3-settlement-engine/` — finalizes cross-VM transfers; issues receipts
- Trait abstractions in: `crates/x3-asset-kernel-types/` (NOT present in `crates/` — lives in `.rc4-worktrees/old/crates/x3-asset-kernel-types/`)

**Pallets (53 total):**
- Purpose: Individual FRAME modules implementing specific chain capabilities
- Location: `pallets/*/src/lib.rs`
- Full list:
  - Core UAK: `x3-supply-ledger`, `x3-atomic-kernel`, `x3-cross-vm-router`, `x3-asset-registry`, `x3-token-factory`, `x3-settlement-engine`, `x3-account-registry`
  - Finance/DEX: `x3-dex`, `x3-launchpad`, `x3-flash-loan`, `x3-rebalance`, `x3-solvency`, `x3-custody`, `x3-reconciliation`, `x3-wrapped`, `x3-partner`, `x3-treasury-policy`
  - Governance: `governance`, `x3-governance`, `council`, `x3-jury-anchor`, `x3-slash`
  - Swarm/AI: `swarm`, `agent-accounts`, `agent-memory`, `evolution-core`, `x3-verifier`, `x3-agent-law`, `x3-invariants`, `northern-swarm`, `meme-overlord`
  - Infrastructure: `x3-kernel`, `atomic-trade-engine`, `x3-oracle`, `x3-vrf`, `x3-sequencer`, `x3-da`, `x3-consensus`, `x3-cross-chain-validator`, `fraud-proofs`, `private-execution`, `x3-automation`
  - Registry/Discovery: `x3-domain-registry`, `x3-asset-registry`, `x3-compute-market`, `depin-marketplace`, `x3-dapp-hub`
  - SVM: `svm-runtime`
  - EVM bridge: `x3-coin` (acts as EVM bridge pallet)
  - User-facing: `x3-wallet-pallet`, `x3-inventory`, `x3-reservation`, `x3-auction`, `x3-launchpad`
  - System: `pallet-x3-control`

**x3-lang Compiler Stack:**
- Purpose: Custom smart contract language compiling to x3-IR for multi-VM execution
- Location: `.kilo/worktrees/silky-petalite/x3-lang/crates/`
- Crates (separate Cargo workspace):
  - `x3-common` — shared utilities (CLEAN)
  - `x3-ast` — AST including cross-chain declarations (CLEAN after fix)
  - `x3-ir` — X3IR types, GPU proof jobs, receipts (CLEAN after fix)
  - `x3-parser` — parser module (NOT VERIFIED)
  - `x3-lexer` — lexer (BROKEN: missing `cursor.rs`, `lexer.rs`)
  - `x3-tools` — utilities (NOT VERIFIED)
  - `x3vm` — VM execution (BROKEN: 49× E0117 orphan trait violations)
  - `compiler` — depends on broken vm (BROKEN)
- Separate `rust-toolchain.toml` inside the sub-workspace (1.90.0)
- **Not wired into main workspace build** — completely separate build system

**Off-Chain Services:**
- Purpose: API gateway, swarm orchestration, solvency sidecar
- Location: `crates/x3-gateway/src/` (REST API + DB), `services/x3-swarm-api/`, `services/x3-swarm-worker/`, `services/x3-solvency-sidecar/`
- Gateway backend: `crates/x3-gateway/src/rest.rs` + `db.rs`; PostgreSQL; serves `/api/v1/` and `/api/public/funding-swarm/`
- Swarm: `swarm_infrastructure/` (Python) — full swarm agent OS with GPU bridge, causal reasoning, jury system
- Off-chain: separate from runtime, not consensus-critical

**EVM Layer (Frontier):**
- Purpose: Ethereum-compatible RPC and EVM execution inside Substrate
- When active: `frontier` Cargo feature enabled
- Key pallets: `pallet-evm`, `pallet-ethereum`, `pallet-base-fee` (from polkadot-sdk frontier)
- Precompiles: `runtime/src/precompiles.rs`
- RPC: `node/src/rpc_frontier.rs`
- EVM contracts: `X3-contracts/evm/` (Foundry project with `interfaces/` and `flashloan/` contracts)

**SVM Layer:**
- Purpose: Solana Virtual Machine execution embedded in X3 runtime
- Key pallet: `pallets/svm-runtime/` (1076-line pallet)
- Anchor programs: `programs/` (amm, cross-vm-adapter, htlc, staking, token-escrow) and `X3-contracts/svm/`
- **Status:** Has known dependency conflict (`solana-address`); see CONCERNS.md

## Data Flow

**Cross-VM Transfer (Happy Path):**
1. User submits extrinsic to `pallet-x3-cross-vm-router::initiate_transfer`
2. Router validates route, limits (min/max amount, pending_limit), replay protection (nonce/message hash)
3. Router calls `SupplyLedger::debit_source_domain` — decrements source domain total
4. Router emits `TransferInitiated` event; sets transfer to `Pending` state
5. Validator observes event; generates proof
6. `pallets/x3-settlement-engine::settle_transfer` called with proof
7. Settlement credits destination domain via `SupplyLedger::credit_destination_domain`
8. Supply invariant checked: canonical_supply unchanged throughout
9. `TransferSettled` event emitted; nonce/message-hash committed to `UsedMessages`

**Timeout / Refund Path:**
1. `on_initialize` in router checks expired pending transfers
2. Expired transfer: calls `SupplyLedger::refund_source_domain` (reverses debit)
3. Transfer moved to `Expired` state; `TransferExpired` event emitted

**x3-lang Compile Path (not yet wired to runtime):**
1. Source code → `x3-lexer` → token stream
2. Token stream → `x3-parser` → AST (`x3-ast` types)
3. AST → type-checker → typed AST
4. Typed AST → `x3-ir` emission → X3IR instructions (including cross-chain operations)
5. X3IR → VM-specific emitters (EVM bytecode, SVM instructions, native WASM)
6. **Not yet integrated:** no `pallet-x3-vm-dispatch` or equivalent to execute X3IR on-chain

## Key Abstractions

**SupplyLedgerWrite / SupplyLedgerInspect (traits):**
- Purpose: The only API for modifying domain balances; enforces invariant after every mutation
- Definition: In `.rc4-worktrees/old/crates/x3-asset-kernel-types/src/traits.rs` (not at root `crates/`)
- Used by: `pallet-x3-cross-vm-router`, `pallet-x3-token-factory`, `pallet-x3-atomic-kernel`

**RouteLimits:**
- Purpose: Per-route transfer amount constraints
- Fields: `min_amount`, `max_amount`, `pending_limit`, `daily_limit`, `per_wallet_daily_limit`
- Enforced: min/max/pending_limit enforced in router; **daily_limit and per_wallet_daily_limit NOT enforced** (Gap 1)

**LaunchState:**
- Purpose: Records a token presale on-chain
- Location: `pallets/x3-launchpad/src/lib.rs`
- Disconnected from: token factory, DEX (Gap 3)

## Entry Points

**Chain Node Binary:**
- Location: `node/src/main.rs`
- Triggers: `cargo build --release -p x3-chain-node`; invoked as `x3-chain-node --chain=dev`

**Runtime WASM:**
- Location: `runtime/src/lib.rs` (impl_runtime_apis! at bottom)
- Triggers: compiled by `cargo build -p x3-chain-runtime --target wasm32-unknown-unknown`

**x3-lang Compiler (sub-workspace):**
- Location: `.kilo/worktrees/silky-petalite/x3-lang/crates/compiler/src/main.rs` (if exists)
- Triggers: `cargo build` inside sub-workspace

**REST API Gateway:**
- Location: `crates/x3-gateway/src/rest.rs` — Axum server
- Triggers: `cargo run -p x3-gateway`

## Error Handling

**Strategy:** FRAME DispatchError pattern throughout pallets

**Patterns:**
- Pallets return `DispatchResult` / `DispatchResultWithPostInfo`
- Custom error enum per pallet via `#[pallet::error]`
- `ensure!()` macro for precondition checks (returns typed error on failure)
- `checked_add` / `checked_sub` for arithmetic in supply-critical paths
- 3 bare `unwrap()` calls found in critical paths (cross-vm-router, atomic-kernel, supply-ledger) — see CONCERNS.md

## Cross-Cutting Concerns

**Logging:** `log::info!` / `log::debug!` / `log::warn!` via `log` crate; `tracing` in node binary
**Validation:** FRAME `ensure!()` + explicit typed errors; input validation at extrinsic boundary
**Authentication:** Substrate `origin` system — `ensure_signed()`, `ensure_root()`, `ensure_none()`
**Replay Protection:** `UsedMessages: StorageMap<MessageHash, ()>` + monotonic `NextNonce` in cross-vm-router

---

*Architecture analysis: 2026-05-19*

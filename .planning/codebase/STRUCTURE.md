# Codebase Structure

**Analysis Date:** 2026-05-19

## Directory Layout

```
X3_ATOMIC_STAR_corrupt_backup_20260518-225645/
├── Cargo.toml              # Workspace root — PARTIALLY BROKEN (90+ phantom crate members)
├── rust-toolchain.toml     # Rust 1.90.0 pinned
├── Makefile                # Build shortcuts
├── pallets/                # 53 FRAME pallets — ALL PRESENT AND REAL CODE
├── runtime/                # Chain runtime WASM (4 construct_runtime! variants)
├── node/                   # Chain node binary (networking, RPC, block production)
├── crates/                 # Only 4 real crates: gpu-swarm.backup, invariant-macros, northern-swarm, x3-gateway
├── .rc4-worktrees/old/crates/   # 123 ARCHIVED crates (source for missing ones above)
├── .kilo/worktrees/silky-petalite/   # x3-lang compiler sub-workspace (separate Cargo.toml)
├── X3-contracts/           # EVM (Foundry) + SVM (Anchor) smart contracts
├── contracts/              # Hardhat project (atomic-swap-sdk, CrossChainGovernance.sol, etc.)
├── programs/               # Solana/Anchor programs (amm, htlc, staking, cross-vm-adapter, token-escrow)
├── apps/                   # TypeScript frontends (Next.js)
├── services/               # Off-chain Rust services (solvency-sidecar, swarm-api, swarm-worker)
├── swarm_infrastructure/   # Python swarm agent OS
├── integration-tests/      # Rust integration test files (loose .rs files, no Cargo.toml)
├── tests/                  # Additional test files
├── tests_core/             # Core test suite
├── tests_phase4/           # Phase 4 test files
├── launch-gates/           # Proof execution guides, audit packs, genesis ceremony runbooks
├── .github/workflows/      # CI workflows (18 files) — will fail due to missing crates
├── k8s/                    # Kubernetes deployment manifests
├── deployment/             # Deployment scripts and configs
├── infra/                  # Infrastructure configs
├── monitoring/             # Monitoring/observability configs
├── governance/             # CrossChainGovernance.sol
├── staking/                # StakingPool.sol
├── treasury/               # Treasury.sol
├── x3-lang/                # Mirror/symlink of x3-lang (within silky-petalite worktree)
├── docs/                   # Planning docs, gap reports, implementation plans
└── plans/                  # Phase plans
```

## Directory Purposes

**`pallets/` (53 subdirectories):**
- Purpose: FRAME pallet implementations — all actual business logic
- Contains: Each subdirectory has `Cargo.toml`, `src/lib.rs`, `src/mock.rs` (most), `src/tests.rs` (most)
- Key files: `pallets/x3-cross-vm-router/src/lib.rs` (1164 lines), `pallets/x3-kernel/src/lib.rs` (4075 lines), `pallets/x3-supply-ledger/src/lib.rs` (806 lines)
- All 53 pallets are buildable (dependencies may be broken due to missing root crates)

**`runtime/src/`:**
- Purpose: Runtime WASM blob — wires all pallets together via `construct_runtime!`
- Key files:
  - `runtime/src/lib.rs` — main runtime file (1600+ lines); 4 `construct_runtime!` blocks
  - `runtime/src/precompiles.rs` — EVM precompile registry
  - `runtime/src/tests.rs` — runtime-level tests
  - `runtime/src/fraud_proofs/` — fraud proof verification

**`node/src/`:**
- Purpose: Node binary — bootstraps chain services, starts networking and RPC
- Key files:
  - `node/src/service.rs` — Substrate service setup, Aura/GRANDPA wiring
  - `node/src/chain_spec.rs` — genesis config for dev/testnet/mainnet
  - `node/src/rpc.rs` — JSON-RPC handlers (no-frontier)
  - `node/src/rpc_frontier.rs` — JSON-RPC handlers (with EVM frontier)
  - `node/src/flash_finality.rs` — flash finality extension
  - `node/src/main.rs` — binary entry point

**`crates/` (4 real crates):**
- `crates/x3-gateway/src/` — REST API gateway (`rest.rs` Axum server + `db.rs` PostgreSQL)
- `crates/invariant-macros/` — proc macros for invariant checking
- `crates/northern-swarm/src/` — chain watcher + executor + result submitter
- `crates/gpu-swarm.backup/` — GPU swarm backup (not active in main build)
- **WARNING:** ~90 other crate directories listed in `Cargo.toml` do NOT exist. See CONCERNS.md.

**`.rc4-worktrees/old/crates/` (123 archived crates):**
- Purpose: Backup/archive of prior workspace state; many contain the missing crate implementations
- Contains: All the crates missing from `crates/` — x3-liquidity-core, x3-asset-kernel-types, x3-sdk, x3-common, x3-lexer, x3-bridge, flash-finality, poh-generator, x3-rpc, x3-indexer, etc.
- Key restoration candidates: `x3-liquidity-core` (referenced by tests/e2e and cross-vm-router), `x3-asset-kernel-types` (traits used by UAK pallets)
- Status: Archived but not deleted — restoring to `crates/` makes them available to workspace build

**`.kilo/worktrees/silky-petalite/` (x3-lang sub-workspace):**
- Purpose: x3-lang custom programming language compiler — lexer, parser, AST, IR, VM
- Key path: `.kilo/worktrees/silky-petalite/x3-lang/crates/`
- Has its own `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`
- Build separately: `cd .kilo/worktrees/silky-petalite && cargo build -p x3-lang-common`

**`X3-contracts/evm/` (Foundry project):**
- Purpose: EVM Solidity contracts (flashloan, interfaces)
- Key paths: `X3-contracts/evm/contracts/flashloan/`, `X3-contracts/evm/contracts/interfaces/`
- Build: `cd X3-contracts/evm && forge build`

**`X3-contracts/svm/` (Anchor workspace):**
- Purpose: SVM/Solana smart programs via Anchor framework
- Key path: `X3-contracts/svm/programs/`
- Build: `cd X3-contracts/svm && anchor build`

**`programs/` (Anchor programs):**
- Purpose: Additional Solana programs (AMM, HTLC, staking, cross-VM adapter, token escrow)
- Subdirs: `amm/`, `cross-vm-adapter/`, `htlc/`, `staking/`, `token-escrow/`
- Each has `Cargo.toml` and `src/`

**`apps/` (TypeScript frontends):**
- `apps/wallet/` — Next.js wallet app; full `src/`, Next.js config, Tailwind
- `apps/dex/` — Next.js DEX app (App Router); has `app/`, `CLAUDE.md`, `PRPs/`
- `apps/explorer/` — Block explorer; **SPARSE** (only `package.json`, `package-lock.json`, `node_modules/`)
- `apps/dashboard/` — Dashboard app
- `apps/analytics/` — Analytics service
- `apps/validators/` — Validator UI
- `apps/shared/` — Shared UI components
- Build each: `cd apps/wallet && npm install && npm run build`

**`services/` (Off-chain Rust services):**
- `services/x3-swarm-api/` — Swarm API service
- `services/x3-swarm-worker/` — Swarm worker daemon
- `services/x3-solvency-sidecar/` — Off-chain solvency monitor

**`swarm_infrastructure/` (Python agent OS):**
- Purpose: Full AI swarm agent system — goal genome, jury, self-improvement, GPU bridge
- Subdirs: `agents/`, `api_server.py`, `auth.py`, `causal/`, `core/`, `gpu_bridge/`, `governance/`, `jury/`, `social/`, `telemetry/`, `tripwire/`
- Not consensus-critical; does not affect chain state directly

**`integration-tests/`:**
- Loose `.rs` files (not a Cargo workspace member): `cross-vm-atomic-test.rs`, `cross-vm-pallet-test.rs`, `parallel-proposer-integration.rs`, `svm-counter-test/`
- **WARNING:** These are standalone files — not compiled as part of any workspace crate. Must be checked for wiring.

**`launch-gates/`:**
- Contains audit runbooks, proof execution guides, multi-node testnet proof scripts
- Key files: `MAINNET_AUDIT_WORKFLOW.md`, `GENESIS_CEREMONY_RUNBOOK.md`, `invariants.yaml`, `proofs.yaml`

**`.github/workflows/` (18 CI workflow files):**
- `ci.yml` — primary critical-path gate (cargo fmt, check, test, clippy, binary build)
- `full-ci.yml` — extended CI
- `proof-gates.yml`, `v04-ship-gate.yml` — launch gates
- `economic-attack-tests.yml`, `formal-verification.yml` — security
- **WARNING:** All workflows requiring `cargo check --workspace` or `cargo build --workspace` will fail until 90+ missing crates are restored.

## Key File Locations

**Entry Points:**
- `node/src/main.rs` — chain binary entry point
- `node/src/service.rs` — Substrate service setup
- `runtime/src/lib.rs` — runtime WASM entry point
- `crates/x3-gateway/src/rest.rs` — REST gateway entry point

**Configuration:**
- `Cargo.toml` — workspace manifest (WARNING: 90+ phantom members)
- `rust-toolchain.toml` — Rust 1.90.0 pin
- `TESTNET_FEATURE_FLAGS.toml` — feature gate configuration
- `FEATURE_REGISTRY.toml` — feature readiness registry
- `MAINNET_RC1_SCOPE.md` — authoritative RC-1 scope definition
- `hardhat.config.ts` — Hardhat EVM deploy config (placeholder values only)

**Core Logic:**
- `pallets/x3-cross-vm-router/src/lib.rs` — cross-VM transfer router
- `pallets/x3-supply-ledger/src/lib.rs` — supply invariant enforcement
- `pallets/x3-atomic-kernel/src/lib.rs` — atomic multi-VM operations
- `pallets/x3-settlement-engine/src/lib.rs` — transfer settlement + receipt emission
- `pallets/x3-token-factory/src/lib.rs` — OmniToken creation
- `pallets/x3-launchpad/src/lib.rs` — token presale (disconnected from token factory)

**Testing:**
- `pallets/*/src/tests.rs` and `pallets/*/src/mock.rs` — per-pallet unit tests
- `tests/` — workspace-level tests
- `integration-tests/` — integration test files (NOT currently part of any workspace crate)

## Naming Conventions

**Files:**
- Pallets: snake_case directory names (e.g., `x3-cross-vm-router`, `agent-accounts`)
- Rust source: `lib.rs` for library crates, `main.rs` for binaries
- Test mocks: `mock.rs` (co-located with `tests.rs` inside `src/`)

**Directories:**
- Pallets: `pallet-` prefix convention in crate names (e.g., `pallet-x3-cross-vm-router`) but directories use `x3-cross-vm-router`
- Crates: `x3-` prefix for X3-specific crates; no prefix for generic ones

## Where to Add New Code

**New Pallet:**
- Implementation: `pallets/<pallet-name>/src/lib.rs`
- Mock: `pallets/<pallet-name>/src/mock.rs`
- Tests: `pallets/<pallet-name>/src/tests.rs`
- Weights: `pallets/<pallet-name>/src/weights.rs`
- Cargo.toml: `pallets/<pallet-name>/Cargo.toml` with `pallet-<pallet-name>` as package name
- Wire into runtime: `runtime/src/lib.rs` — add to all 4 `construct_runtime!` blocks as needed

**New Crate (utility/SDK):**
- Create: `crates/<crate-name>/src/lib.rs` + `crates/<crate-name>/Cargo.toml`
- Add to workspace members in root `Cargo.toml`
- OR: restore from `.rc4-worktrees/old/crates/` if the crate exists there

**New REST Endpoint:**
- Add to: `crates/x3-gateway/src/rest.rs`
- DB schema: add migration file to gateway's migrations directory

**New Frontend Feature:**
- Location: `apps/<app-name>/src/` following Next.js App Router conventions
- Shared components: `apps/shared/`

## Special Directories

**`.rc4-worktrees/old/`:**
- Purpose: Full prior workspace backup — 123 crates, pallets, runtime
- Generated: No (manually preserved backup)
- Committed: YES (appears to be in the git history)

**`target/`:**
- Purpose: Cargo build artifacts
- Generated: YES by cargo
- Committed: NO (in .gitignore)

**`.planning/codebase/`:**
- Purpose: These codebase mapping documents for GSD agent use
- Generated: YES by mapping agents
- Committed: Recommended YES for continuity

---

*Structure analysis: 2026-05-19*

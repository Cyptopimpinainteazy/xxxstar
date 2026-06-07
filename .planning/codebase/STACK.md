 # Technology Stack

**Analysis Date:** 2026-05-19

## Languages

**Primary:**
- Rust 1.90.0 — blockchain runtime, pallets, node binary, crates, integration tests
- TypeScript — frontend apps (Next.js), Hardhat EVM deploy scripts
- Solidity 0.8.24 — EVM smart contracts (flashloan, governance, staking, treasury)

**Secondary:**
- Python 3 — swarm infrastructure agents (`swarm_infrastructure/`)
- Rust/Anchor — SVM/Solana programs (`programs/`, `X3-contracts/svm/`)
- HTML/CSS/Chart.js — simple static sites (`site/funding-swarm.html`)

## Runtime

**Environment:**
- Rust (chain binary): `cargo build --release`
- Node.js: each `apps/` sub-app, version from `package.json` (not pinned at root)
- Python 3: `swarm_infrastructure/` (requirements.txt present)

**Package Manager:**
- Rust: Cargo (workspace resolver = "2"); `Cargo.lock` present in sub-workspace
- Node.js: npm (package-lock.json present in apps/)
- Python: pip (requirements.txt)

**Lockfile:**
- `Cargo.lock`: present in `.kilo/worktrees/silky-petalite/` (x3-lang sub-workspace)
- Root `Cargo.lock`: absent or stale (workspace is broken — cannot regenerate)

## Frameworks

**Core (Blockchain):**
- `polkadot-sdk` (`stable2512`, pinned rev `948fbd2`) — FRAME, Substrate, sp-* crates
  - Provides: `frame_support`, `frame_system`, `sp_runtime`, `sp_core`, `sp_io`, pallet-session, pallet-grandpa, pallet-aura, pallet-balances, pallet-timestamp, pallet-scheduler, pallet-preimage, pallet-treasury, pallet-collective
  - Frontier (EVM): `pallet-evm`, `pallet-ethereum`, `pallet-base-fee`, `fp-rpc`, `fp-evm`, `fc-rpc`, `fc-db`, `fc-storage`
  - Also provides: `polkadot-consensus-beefy`, `pallet-beefy`, `pallet-beefy-mmr`

**Testing:**
- `frame-support` test externalities: `sp_io::TestExternalities` — all pallet unit tests
- `ink_e2e` / substrate-test-utils where used

**Build/Dev:**
- `rustfmt` — formatting (`cargo fmt --all`)
- `clippy` — linting (`cargo clippy --workspace -- -D warnings`)
- `cargo-deny` — dependency audit (`deny.toml` present)

**Frontend:**
- Next.js (App Router) — `apps/wallet/`, `apps/dex/`
- Tailwind CSS — `apps/wallet/tailwind.config.ts`, `apps/dex/`
- React — via Next.js

**EVM Tooling:**
- Foundry (forge/anvil) — `X3-contracts/evm/foundry.toml`
- Hardhat — `hardhat.config.ts` at root; `@nomiclabs/hardhat-waffle`, `@nomiclabs/hardhat-ethers`

**SVM Tooling:**
- Anchor framework — `programs/Anchor.toml`, `X3-contracts/svm/Anchor.toml`
- Solana CLI toolchain

## Key Dependencies

**Critical (Rust workspace):**
- `polkadot-sdk` stable2512 — everything blockchain
- `parity-scale-codec` 3.6.5 — on-chain serialization (SCALE codec)
- `scale-info` 2.11.1 — runtime type metadata
- `tokio` 1.0 — async runtime for node binary and gateway
- `anyhow` — error handling in off-chain code
- `serde` + `serde_json` — off-chain serialization
- `tracing` + `tracing-subscriber` — observability in node
- `axum` — REST API in x3-gateway

**Infrastructure (Rust workspace):**
- `sp-core` — cryptographic primitives, AccountId, hash types
- `sp-runtime` — runtime types (Block, DispatchError, etc.)
- `frame-executive` — block execution engine
- `internment` (with `serde` feature) — interned strings in x3-ir

## Configuration

**Environment:**
- Chain: configured via `chain_spec.rs` (dev/testnet/mainnet)
- Feature gates: `TESTNET_FEATURE_FLAGS.toml` — 16 flags controlling enabled features
- Feature registry: `FEATURE_REGISTRY.toml` — readiness scores and blockers per feature
- Off-chain services: env vars (see INTEGRATIONS.md)

**Build:**
- `Cargo.toml` — workspace root; WARNING: 90+ phantom member crate entries
- `rust-toolchain.toml` — `channel = "1.90.0"`, `targets = ["wasm32-unknown-unknown"]`
- `MAINNET_RC1_SCOPE.md` — authoritative feature gate for RC-1

## Platform Requirements

**Development:**
- Rust 1.90.0 (use `rustup show` inside repo — toolchain file enforces this)
- `wasm32-unknown-unknown` target installed: `rustup target add wasm32-unknown-unknown`
- Node.js 18+ for frontend apps
- Foundry installed for EVM contract work
- Anchor CLI + Solana CLI for SVM program work
- PostgreSQL for x3-gateway (env: `DATABASE_URL`)

**Production:**
- Linux x86_64 node binary (cross-compile: Linux ARM possible but not configured)
- Kubernetes manifests available at `k8s/`
- Docker: `Dockerfile.validator`, `Dockerfile.indexer`, `Dockerfile.mainnet-check`

---

*Stack analysis: 2026-05-19*

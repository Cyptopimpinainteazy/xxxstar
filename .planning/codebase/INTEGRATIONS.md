# External Integrations

**Analysis Date:** 2026-05-19

## APIs & External Services

**OpenRouter AI API:**
- What it's used for: Swarm agent inference via `swarm_infrastructure/`; x3-gateway funding-swarm webhook
- Client: `openrouter_client.py` in swarm_infrastructure + direct HTTP in gateway
- Auth env var: `OPENROUTER_API_KEY`
- Note: `StubAiProvider` is used when `OPENROUTER_API_KEY` is not set — swarm degrades gracefully

**GitHub Actions:**
- What it's used for: CI/CD pipeline (18 workflow files in `.github/workflows/`)
- Auth: GitHub Actions automatic token
- Status: CI workflows exist but will fail on `cargo check --workspace` until 90+ missing crates are resolved

## Data Storage

**Databases:**
- On-chain: Substrate RocksDB (via `sc-client-db` from polkadot-sdk) — block history, state trie
  - Connection: filesystem path set via `--base-path` CLI arg to node binary
  - Client: Substrate native storage layer; no ORM

- Off-chain PostgreSQL (x3-gateway funding swarm):
  - Connection env var: `DATABASE_URL` or `X3_GATEWAY_TEST_DATABASE_URL`
  - Client: SQLx (`db.rs` in `crates/x3-gateway/src/`)
  - Migrations: in `crates/x3-gateway/migrations/` (SQLx migration pattern)
  - Tables: funding swarm state, transactions, agent state

**File Storage:**
- Local filesystem: Substrate base path for chain data
- No cloud file storage detected in production code

**Caching:**
- Frontier (EVM): `fc-db` for Ethereum block mapping cache
- No Redis or other cache layer detected

## Authentication & Identity

**Chain Identity:**
- SS58-encoded addresses; Ed25519 keys for GRANDPA validators; Sr25519 for Aura
- Authority set: managed by `pallet-session` + `pallet-aura` + `pallet-grandpa`

**REST Gateway (`crates/x3-gateway/`):**
- Auth: Bearer token header (`Authorization: Bearer <token>`)
- Admin endpoints protected separately
- No OAuth or third-party SSO

**Hardhat RPC credentials:**
- Status: PLACEHOLDER ONLY — `hardhat.config.ts` contains `<RPC_ENDPOINT_1>` through `<RPC_ENDPOINT_103>` and `<PRIVATE_KEY>` literals
- These are not functional credentials; Hardhat deploy is not operational

## Monitoring & Observability

**Prometheus:**
- `node/src/metrics.rs` — custom Prometheus metrics registered
- Prometheus exposition port: default Substrate `/metrics` endpoint
- Monitoring configs: `monitoring/` directory contains dashboards

**Grafana:**
- Dashboards defined in `monitoring/` (Grafana JSON)

**Logs:**
- `node/src/logging.rs` — logging setup using `tracing`
- `tracing-subscriber` formats for dev (pretty) and production (JSON)
- Swarm: Python `logging` module + telemetry in `swarm_infrastructure/telemetry/`

**Error Tracking:**
- None detected in production paths (no Sentry, Rollbar, etc.)

## CI/CD & Deployment

**Hosting:**
- Chain: Kubernetes (`k8s/` manifests)
- Frontend: Vercel (from `apps/dex/CLAUDE.md` and deploy scripts; deploy-dashboard.yml CI)

**CI Pipeline:**
- GitHub Actions with 18 workflows:
  - `ci.yml` — primary gate (fmt, check, test, clippy, binary build)
  - `proof-gates.yml` — launch gate proofs
  - `v04-ship-gate.yml` — v0.4 ship gate
  - `economic-attack-tests.yml` — economic security tests
  - `formal-verification.yml` — formal proofs
  - `codeql.yml`, `semgrep.yml`, `snyk.yml`, `trivy.yml`, `osv-scan.yml` — security scanning
  - `rust-clippy.yml` — dedicated clippy run
  - `deploy-dashboard.yml` — dashboard deploy
  - `x3fronend-gpu-swarm.yml` — frontend + GPU swarm integration
  - `release-candidate-rehearsal.yml` — RC release rehearsal
- **Current status:** ALL workflows requiring cargo will fail until missing crates are restored

**Docker:**
- `Dockerfile.validator` — validator node image
- `Dockerfile.indexer` — indexer image
- `Dockerfile.mainnet-check` — mainnet readiness check container

## EVM Integration (Frontier)

- Framework: Frontier from polkadot-sdk stable2512
- Activation: `with-frontier` Cargo feature in runtime
- Pallet stack: `pallet-evm` → `pallet-ethereum` → `pallet-base-fee` + `pallet-dynamic-fee`
- Precompiles: `runtime/src/precompiles.rs`
- RPC: `node/src/rpc_frontier.rs` (Ethereum JSON-RPC compatibility)
- Contracts: `X3-contracts/evm/contracts/flashloan/` + `interfaces/` (Foundry project)
- Status: `with-frontier` feature gate functional in dev+frontier and mainnet-rc1+frontier build variants

## SVM Integration

- Framework: Solana/Anchor `v0.30.1`
- Pallet: `pallets/svm-runtime/src/lib.rs` (1076 lines) — Substrate pallet wrapping SVM execution
- Programs: `programs/amm/`, `programs/htlc/`, `programs/staking/`, `programs/cross-vm-adapter/`, `programs/token-escrow/`
- External workspace: `X3-contracts/svm/` (separate Anchor workspace)
- Status: **DISABLED** — `DISABLED_BLOCKED` in `TESTNET_FEATURE_FLAGS.toml`; known `solana-address` crate dependency conflict prevents compilation in workspace

## BTC Integration

- Status: `btc_mainnet_gateway = "DISABLED_BLOCKED"` in `TESTNET_FEATURE_FLAGS.toml`
- BTC fortress gateway: `btc_fortress_gateway = "SIM_TESTNET"` (simulation only)
- No on-chain BTC pallet visible in runtime (gated out completely for RC-1)

## GPU Validator

- Crate: `crates/gpu-swarm.backup/` — GPU swarm acceleration backup
- Python bridge: `swarm_infrastructure/gpu_bridge/`, `swarm_infrastructure/gpu_compute/`
- Status: `gpu-acceleration` is gated out of RC-1 per `MAINNET_RC1_SCOPE.md`
- `cross-chain-gpu-validator` and `confidential-gpu` exist in `.rc4-worktrees/old/crates/`

## Webhooks & Callbacks

**Incoming:**
- REST gateway: `POST /api/public/funding-swarm/*` — swarm result callbacks
- REST gateway: `POST /api/v1/*` — internal swarm API

**Outgoing:**
- None detected in active production code paths
- Swarm infrastructure may make outgoing HTTP requests to OpenRouter

## Environment Configuration

**Required env vars (chain node):**
- None strictly required — chain uses CLI flags
- `RUST_LOG` — log level (default: info)
- `RUST_BACKTRACE` — set to 1 for debugging

**Required env vars (x3-gateway):**
- `DATABASE_URL` — PostgreSQL connection string
- `OPENROUTER_API_KEY` — optional; swarm uses StubAiProvider if absent
- `X3_GATEWAY_SECRET` — API auth secret
- `X3_GATEWAY_TEST_DATABASE_URL` — test database for integration tests

**Secrets location:**
- `.env` files not committed (in .gitignore)
- Secrets injected via environment in Kubernetes / GitHub Actions secrets

---

*Integration audit: 2026-05-19*

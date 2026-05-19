# X3 Atomic Star — Current Mainnet Status

> **Last updated:** 2025 (auto-generated from codebase state)
> **Target:** v0.4 internal-only Mainnet RC-1

---

## TL;DR

| Dimension | Status |
|-----------|--------|
| Node binary builds | ✅ `cargo build --release -p x3-chain-node` |
| Consensus (Aura + GRANDPA) | ✅ Real — not simulated |
| Internal cross-VM routing (6 routes) | ✅ Tested, supply invariant proven |
| External bridges (Ethereum/Solana mainnet) | 🔒 Frozen at genesis — governance-gated |
| 3-validator local testnet | ✅ `scripts/testnet-full-launch.sh` |
| Supply ledger invariant | ✅ Enforced on every operation |
| Settlement engine | ✅ State machine implemented; OCW stub is testnet-only |
| CI hard gates | ✅ `.github/workflows/ci.yml` (10 required jobs) |
| Public testnet | 🚧 Pre-launch checks in progress |
| Mainnet | 🔴 Not yet — pending public testnet validation |

---

## What Is Working (Production-Quality)

### Consensus Layer
- **Aura block production** with real slot assignment
- **GRANDPA finality** with real voting and equivocation protection
- Node binary: `target/release/x3-chain-node`
- Dev chain: `x3-chain-node --chain=dev --tmp`
- 3-validator testnet: `scripts/testnet-full-launch.sh`

### Internal Cross-VM Execution
- **X3Native ↔ X3Evm ↔ X3Svm** — all 6 internal routes implemented
- Atomic source-debit / destination-credit semantics
- Replay protection: `UsedMessages` + `UsedNonces` stores
- Expiry + cancel: `cancel_expired_xvm_transfer` returns pending supply to source
- Supply invariant: `represented_total ≤ canonical_supply` enforced on every operation
- Scope freeze: external bridges disabled by default; require governance to open

### Supply Ledger
- `pallet-x3-supply-ledger`: canonical supply accounting per asset
- `check_invariant()` is called at every supply-changing operation
- Historical proofs retained for `HISTORICAL_PROOF_RETENTION_BLOCKS = 1,000` blocks

### Settlement Engine
- State machine: `MATCH → ASSETS_LOCKED_X3 → EXTERNAL_EXECUTION → PROOF_SUBMITTED → FINALIZE_X3`
- Refund path: `→ REFUND_X3` on timeout or failure
- Atomic locks and escrow implemented
- Settlement timeout checker runs via `on_idle()`

---

## What Is TESTNET_ONLY

These features work on testnet but have known limitations for mainnet:

| Feature | Limitation | Ships When |
|---------|------------|------------|
| Settlement OCW | Stub — logs that hook is wired; no auto-finalization | Phase 1c (post-RC1) |
| Relayer authorization | Governance-approved list (not decentralized) | Post-RC1 |
| Emergency pause | Available but governed; not fully autonomous | Post-RC1 |
| GPU validator sidecar | Optional health check; not required for consensus | Post-RC1 |

---

## What Is DISABLED_POST_RC1

These are explicitly frozen and MUST NOT be enabled until audited:

| Feature | Scope Guard | How to Enable (post-audit) |
|---------|-------------|---------------------------|
| External bridge to Ethereum mainnet | `ExternalBridgesEnabled = false` at genesis | `set_external_bridge_audit_gate(true)` → `set_external_bridges_enabled(true)` |
| External bridge to Solana mainnet | Same kill-switch | Same governance flow |
| Parallel executor | Compile-time `compile_error!` if `mainnet-rc1 + parallel-executor` | Remove scope guard after audit |
| AppZone factory | Compile-time `compile_error!` if `mainnet-rc1 + appzone-factory` | Remove scope guard after audit |
| PQ cryptography (experimental) | Compile-time guard | Remove scope guard after audit |
| AI optimizer | Compile-time guard | Remove scope guard after audit |
| Advanced DEX | Compile-time guard | Remove scope guard after audit |

---

## Honest Gap Analysis

### Gaps Before Public Testnet
1. **Block explorer**: `apps/explorer/` exists but needs connection to real node RPC
2. **Faucet**: No automated testnet faucet deployed
3. **Documentation**: Deployment guide needs to be complete (see `docs/deployment/DEPLOYMENT_GUIDE.md`)
4. **Settlement OCW**: Full off-chain worker for automated finalization is a stub

### Gaps Before Mainnet
1. All DISABLED_POST_RC1 features above need external security audits
2. Decentralized relayer set for settlement engine
3. Slashing conditions fully implemented and tested on testnet
4. Economic model validated (staking, inflation, fee parameters)
5. Emergency response playbook tested on public testnet

---

## How to Verify the Node Is Real (Not Mock)

```bash
# Build the real node
cargo build --release -p x3-chain-node

# Start a dev chain
./target/release/x3-chain-node --chain=dev --tmp --rpc-port 9933

# Or use the start script (exits with error if binary missing)
./scripts/start-x3-chain.sh

# Start 3-validator testnet
./scripts/testnet-full-launch.sh

# Run mock RPC only (explicitly dev-only)
./scripts/start-mock-rpc-dev.sh
```

### How to distinguish real from mock:
- Real node: responds with real block hash, increments finalized block height, runs GRANDPA
- Mock: fixed fake responses from `scripts/mock-rpc-server.js`; explicitly labeled DEV ONLY

---

## CI Status

All merges to `main` require the `x3 / critical-path-all-pass` check in:
`.github/workflows/ci.yml`

Required gates:
- `cargo fmt --all -- --check`
- `cargo check -p x3-chain-runtime`
- `cargo check -p x3-chain-node`
- `cargo test -p pallet-x3-cross-vm-router` (8 named production-proof tests)
- `cargo test -p pallet-x3-supply-ledger`
- `cargo test -p pallet-x3-settlement-engine`
- `cargo test -p pallet-x3-atomic-kernel`
- `cargo clippy --workspace --all-targets -- -D warnings`

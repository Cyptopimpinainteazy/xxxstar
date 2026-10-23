# X3 Atomic Star — Mainnet Status

**Updated: 2026-10-23 — 62.74% registry-scored readiness across 23 entries, derived from `FEATURE_REGISTRY.toml`. This is not a mainnet-ready claim, and nothing is deployed.**

> `FEATURE_REGISTRY.toml` is the single canonical source. Every number below is read from it; none is hand-maintained.
>
> Two gates guard this file:
> - `scripts/check-readiness-consistency.sh` fails when a document claims **more** than the registry allows.
> - `scripts/check-readiness-status-current.py` recomputes the count, the average and every per-feature
>   score from the registry and fails when this document disagrees — the direction the first gate never checked.
>   It runs as the `readiness status current` gate in `scripts/local-ci.sh`.
>
> A stored average is a copy of a derivable value. Change a `readiness_score` and this file fails the gate
> until it is updated in the same commit.

## Deployment reality (2026-10-23)

| Check | Result |
| --- | --- |
| `rpc.testnet.x3-chain.io` | does not resolve |
| `faucet.testnet.x3-chain.io` | does not resolve |
| `bootnode.testnet.x3-chain.io` | does not resolve |
| `docs/reports/TESTNET_DEPLOYMENT_CHECKLIST.md` | 168 boxes, 0 checked |
| `.github/workflows/testnet-deploy.yml` | has never run |
| `deployment/keys/bootnode-info.txt` | three `127.0.0.1` loopback addresses |
| Running `x3-chain-node` processes | 0 |

All testnet evidence in this repository is **loopback evidence**: multiple validators on one host, finalizing and agreeing. That proves the binary works. It does not prove a network.

## System Completion Scoreboard

```
Atomic Router (pallets/x3-cross-vm-router)           █████████░   88%  LIVE_TESTNET — External-bridge path disabled at genesis — governance gate, not code gap
Triforge Runtime (runtime)                           ████████░░   80%  GUARDED_TESTNET — the rehearsal starts from this runtime's genesis, not a previous runtime's exported state, so cross-version migration of historical state is unverified
AXE DEX (pallets/x3-dex)                             ████████░░   75%  GUARDED_TESTNET — DEX uses EconomicHalt runtime governance; no compile-time feature gate removal needed
Trading Core v1 (x3-lang)                            ████████░░   75%  GUARDED_TESTNET — No multi-validator or live-network execution of a trading-core-v1 program has been performed — all 131 passing tests are single-process compiler/VM/CLI-level
X3 Forge (pallets/x3-token-factory)                  ████████░░   75%  GUARDED_TESTNET — Token factory uses EconomicHalt runtime governance (not compile-time gates)
BTC Fortress Gateway (pallets/x3-settlement-engine)  ███████░░░   72%  SIM_TESTNET — A header source exists now, but a public chain does not run it: the header chain bootstraps from a checkpoint (spec_version 14/15) and `anchor_btc_checkpoint`/genesis `btc_checkpoints` pins one, yet no public network has one pinned, the relayer is a script an operator runs rather than a node feature, and nothing bonds it for withholding — TICKET-095 stays open until there is more than one
Atomic Trade Engine (pallets/atomic-trade-engine)    ███████░░░   70%  GUARDED_TESTNET — No multi-validator or live-network run — 48 passing tests are all mock-runtime unit tests
State Snapshot (crates/x3-state-snapshot)            ███████░░░   70%  GUARDED_TESTNET — the exporter consumes `export-state` output rather than reading the database directly
SVM Atomic Swap HTLC (programs/svm/x3_atomic_swap)   ███████░░░   70%  LIVE_TESTNET — Deployed to a local validator only — no devnet or mainnet deployment has been performed
Atomic Lock (pallets/x3-lp-locker)                   ███████░░░   68%  LIVE_TESTNET — Cross-pallet integration (launchpad → LP locker) not verified in CI
Atomic Gateway (pallets/x3-cross-vm-router)          ██████░░░░   65%  GUARDED_TESTNET — Gateway is guarded — ExternalBridgesEnabled = false at genesis (governance gate)
X3 Wrapped (pallets/x3-wrapped)                      ██████░░░░   65%  GUARDED_TESTNET — External bridge deposits are disabled at genesis (ExternalBridgesEnabled=false per LAUNCH_SCOPE.md) — wrapped assets can only be minted via governance-gated RPC for testing, not from a real external-chain deposit
X3 HTLC (pallets/x3-settlement-engine)               ██████░░░░   60%  GUARDED_TESTNET — No external audit of the settlement engine's custody path
Northern Swarm Reactor (crates/northern-swarm)       ██████░░░░   58%  GUARDED_TESTNET — No live-node integration test. The executor/chain contract is proven at the boundary only - encoder/decoder, signer selection, restart recovery - never against a running node. The crate cannot currently reach a live chain in CI, and the row stays below 50 for that reason
Repo Scanner Agent (scripts/swarm/swarm_scan.sh)     ██████░░░░   58%  LIVE_TESTNET — Dev-ops tooling — not part of blockchain runtime
Atomic Kernel (pallets/x3-atomic-kernel)             ██████░░░░   57%  LIVE_TESTNET — the unsigned finalization extrinsic was removed 2026-09-23 (TICKET-097): finalization is signed only, and `RuntimeOrigin::none()` cannot reach it. Independent audit and public-network proof remain outstanding
X3 Wallet Pallet (pallets/x3-wallet-pallet)          ██████░░░░   55%  LIVE_TESTNET — Biometric template and recovery logic have no security review
X3 Swarm Core (crates/x3-swarm-core)                 █████░░░░░   52%  GUARDED_TESTNET — excluded from the root workspace and absent from the runtime dependency graph: the swarm control plane is not on the deployed blockchain path
X3 Reactor (crates/x3-bench)                         █████░░░░░   49%  LIVE_TESTNET — Benchmark infrastructure not in CI critical path
Launch Gate (scripts/mainnet/mainnet_rc_gate.sh)     █████░░░░░   47%  LIVE_TESTNET — both RC gates fail closed, but cannot pass without the 7 physical servers, a 72-hour stability window, and a public explorer/faucet
X3 Sentinel (pallets/x3-sentinel)                    █████░░░░░   47%  GUARDED_TESTNET — The guard is inert while nothing can be frozen, so its refusal path (`AuthorityFrozenBySentinel`) has never run on a chain; the drill takes that branch only when a privileged path exists
SVM X3 HTLC Program (X3-contracts/svm/programs/x3_htlc) ████░░░░░░   45%  SIM_TESTNET — Not wired into any live cross-VM route — no caller in the atomic router or crates/cross-vm-coordinator; the live SVM route runs the sibling programs/svm/x3_atomic_swap program
Tauri OS (apps/tauri-os)                             ████░░░░░░   42%  GUARDED_TESTNET — Desktop UI only — not part of blockchain runtime
```

**Average readiness: 62.74%** — 23 tracked features.

## What is verified

| Capability | Evidence |
| --- | --- |
| Multi-validator agreement | `node/tests/supply_invariant_distributed.rs`, `flash_finality_network.rs`, `x3lang_network_receipt.rs`, `x3vm_live_lifecycle.rs` — run as `local-ci.sh --cross` gates |
| Atomic kernel finalization authorization | `RuntimeOrigin::none()` cannot reach finalization; a named regression test asserts it (TICKET-097, closed 2026-09-23) |
| Registry integrity | `scripts/check-readiness-consistency.sh` resolves every `required_tests` name against a real `fn` |
| Secret hygiene in tracked files | `scripts/check-no-provider-secrets.sh`, wired into `make guard` |
| Local gate suite | `scripts/local-ci.sh` — fmt, guards, check, lint, unit |

## What is NOT verified

| Gap | Severity | Detail |
| --- | --- | --- |
| Credential revocation | 🔴 Open | TICKET-102. Four provider keys and one wallet private key were in tracked source; removal from HEAD does not revoke them. No revocation is proven. |
| Tracked build/state artifacts | 🔴 Open | TICKET-103 names one package; **337 `dist/` files across 11 packages** are actually tracked, plus `crawler_state.json`. |
| Real network deployment | 🔴 Not started | See deployment reality above. |
| `mainnet-rc1` build | ⚪ BLOCKED | Cannot be compiled on this host: filesystem at 100%, 1 MB free. Not a code verdict — the gate did not run. |
| BTC signer quorum | 🔴 Missing | Withdrawals have no production multi-signer policy. |
| Biometric security review | 🔴 Pending | Wallet pallet biometric and recovery logic unaudited. |

## Score movement since 2026-09-24

The previous revision of this file recorded 55.29% across 21 entries, `atomic_kernel` at 35%, and described `mainnet-rc1` as having a pre-existing compile error. All three were stale:

- **21 → 23 entries, 55.29% → 62.74%.** Two rows were added and several re-scored in the registry without this document following.
- **`atomic_kernel` 35% → 57%.** The unauthorized-finalization hole was closed 2026-09-23 by deleting the unsigned extrinsic, not signing it.
- **`mainnet-rc1` "pre-existing compile error"** — `get_wrapped_balance` exists at `runtime/src/lib.rs:5028`, so that specific break is gone. The current build attempt is blocked by disk, not by code.

Nine fictional `atomic_kernel` invariant tests (CRITICAL-TOK-1, 2026-09-06) were removed and replaced with names that resolve to real functions.

## Historical evidence

### 2026-09-07 Atomic lifecycle

- Dev-chain submission reached `Finalized`.
- A held submission reached `Executing`, then forced rollback reached `RolledBack`.
- `cargo audit`: 0 blocking Rust vulnerabilities, 33 allowed warnings.
- Root `pytest tests`: 181 passed.

## Open blockers, in priority order

1. **Credential revocation** (TICKET-102) — unproven, and not fixable by code alone.
2. **Tracked build artifacts** (TICKET-103) — 337 files; the ticket understates its own scope.
3. **Real operator deployment** — DNS, bootnodes, a ticked checklist, and a named operator.
4. **A written definition of "testnet accepted"** — 168 unticked boxes with no stated criterion for ticking one.

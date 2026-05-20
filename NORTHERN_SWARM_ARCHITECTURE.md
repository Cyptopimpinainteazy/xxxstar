# Northern Swarm Architecture

> **Status:** RC1 off-chain executor scaffolded · RC2 on-chain pallet scaffolded · RC3–RC5 planned
>
> `pallets/swarm` is **legacy-only reference** — do not add new production dependencies to it.

---

## Overview

Northern Swarm is a three-layer system for verified off-chain compute on X3:

```
┌─────────────────────────────────────────────────────────┐
│  X3 Runtime (on-chain)                                  │
│  └── pallets/northern-swarm  (RC2 registry pallet)      │
│       stake / tasks / result commits / slash-reward     │
├─────────────────────────────────────────────────────────┤
│  Off-chain executor network                             │
│  └── crates/northern-swarm  (RC1 executor binary)       │
│       chain watcher → task fetch → execute → submit     │
├─────────────────────────────────────────────────────────┤
│  Payload layer                                          │
│  └── IPFS / inline hex URIs                             │
│       task body, params, model weights                  │
└─────────────────────────────────────────────────────────┘
```

---

## RC Roadmap

### RC1 — Off-chain executor skeleton (`crates/northern-swarm`) ✅

**Goal:** prove the off-chain execution loop end-to-end on a local testnet.

| Component | File | Status |
|-----------|------|--------|
| Binary entry point | `src/main.rs` | ✅ done |
| Crate root / re-exports | `src/lib.rs` | ✅ done |
| Canonical types | `src/types.rs` | ✅ done |
| Deterministic executor | `src/executor.rs` | ✅ done |
| Chain watcher (polling) | `src/chain_watcher.rs` | ✅ done (poll stub) |
| Result submitter | `src/result_submitter.rs` | ✅ done (local proof store) |

**RC1 limitations (explicitly stubbed):**
- `poll_pending_tasks()` returns an empty vec — no live RPC connection yet.
- `fetch_payload()` only supports the `hex:<hex>` URI scheme.
- `submit()` writes proof files to `./proofs/` instead of calling the chain.

---

### RC1.5 — Live chain watcher (planned)

**Goal:** connect the executor to a running node via `jsonrpsee` or `subxt`.

- Subscribe to `NorthernSwarm::TaskSubmitted` events via WebSocket.
- Implement IPFS payload fetch (`ipfs://` URI scheme).
- Submit result hashes via `NorthernSwarm::submit_result` extrinsic.

---

### RC2 — On-chain registry pallet (`pallets/northern-swarm`) ✅ (scaffold)

**Goal:** minimal FRAME pallet to anchor executor stake and result hashes.

| Storage | Key → Value |
|---------|-------------|
| `Executors` | `AccountId → ExecutorRecord` |
| `Tasks` | `Hash → TaskRecord` |
| `ResultCommits` | `(Hash, AccountId) → Hash` |
| `ClaimedTaskCount` | `AccountId → u32` |

| Extrinsic | Index | Who |
|-----------|-------|-----|
| `register_executor` | 0 | Any |
| `deregister_executor` | 1 | Self |
| `release_stake` | 2 | Self (after cooldown) |
| `submit_heartbeat` | 3 | Executor |
| `submit_task` | 4 | Any |
| `claim_task` | 5 | Executor |
| `submit_result` | 6 | Executor |
| `slash_executor` | 7 | Root |

**RC2 TODO before mainnet:**
- [ ] Wire pallet into `runtime/src/lib.rs` — add `Config` impl, add to `construct_runtime!`
- [ ] Add `pallet-northern-swarm` to `runtime/Cargo.toml`
- [ ] Benchmark all extrinsics and replace placeholder `Weight::from_parts` constants
- [ ] Add mock runtime and unit tests (`pallets/northern-swarm/src/tests.rs`)

---

### RC3 — Quorum verification (planned)

**Goal:** require M-of-N executors to independently commit identical result hashes
before finalising a task and releasing the reward.

- `on_finalize` hook scans `ResultCommits` and checks for quorum threshold.
- Non-matching executors are auto-slashed (`SlashReason::QuorumMismatch`).
- Winning executors receive reward split from the task bond.
- `MaxClaimedTasksPerExecutor` becomes meaningful for task assignment fairness.

---

### RC4 — X3 Lang job compiler (planned)

**Goal:** compile X3 Lang job definitions into `TaskPayload` bytecode executed
by `TaskExecutor::run_deterministic()`.

- Replace the RC1 stub `run_deterministic()` with a WASM-sandboxed evaluator.
- `TaskKind::X3LangAgent` dispatches to the X3 interpreter.
- Proof generation includes WASM execution trace hash.

---

### RC5 — Full Northern Swarm launch (planned)

- GPU-accelerated `TaskKind::AiInference` dispatch.
- Integration with `gpu-swarm/` scheduling layer.
- Validator set key registration (executor ECDSA key on-chain).
- Dispute resolution protocol (RC3 jury vote removed — replaced by ZK proof).

---

## Legacy reference

`pallets/swarm/src/lib.rs` is preserved **read-only** for reference.
It was deprecated in `//! **DEPRECATED**` at the top of that file.

Do not add new runtime imports, storage migrations, or extrinsics to `pallet-swarm`.
The Northern Swarm pallet is its sole replacement.

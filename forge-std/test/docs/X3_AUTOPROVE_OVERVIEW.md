# X3 Autoprove YOLO Mode Overview

**Internal name:** `X3 Autoprove`
**Public name:** *X3 Atomic Star — The Chain That Builds Itself*

The Autoprove system is a self‑sustaining CI/CD pipeline that **builds, tests, breaks, fixes, proves, reports and markets** the X3 blockchain automatically.  It follows a strict **YOLO** philosophy for fast‑path actions (build, test, wire, report) while enforcing **hard approval gates** for any change that touches critical runtime components such as bridges, BTC gateway, tokenomics, genesis, validator keys or main‑net deployment.

---

## Build Phases
| Phase | Description | Key Artifacts |
|------|-------------|---------------|
| **0 – Repo Truth System** | Generates a feature truth table and test‑feature flags. | `docs/FEATURE_REGISTRY.toml`, `docs/TESTNET_FEATURE_FLAGS.toml` |
| **1 – Swarm Core** | Implements the autonomous swarm agents that scan the repo, generate tasks and apply safe patches. | `crates/x3-swarm-core`, `services/x3-swarm-api` |
| **2 – X3 Readiness / Proof Engine** | Runs the suite of proof commands that produce the canonical readiness reports. | `crates/x3-readiness` |
| **3 – Launch Gate** | Test‑net and main‑net RC gate scripts that enforce the hard‑gate policy. | `scripts/testnet/testnet_rc_gate.sh`, `scripts/mainnet/mainnet_rc_gate.sh` |
| **4 – Tauri OS Wiring** | The **Atomic Console** Tauri application that surfaces feature mode, health, last test status, chain events and blockers. | `apps/tauri-os` (to be added) |
| **5 – Core Proofs** | Atomic Kernel & Router invariants, AXE/Forge/Lock/Sentinel proofs. | – |
| **6 – Gateway & BTC Fortress** | Guarded external bridge audit and simulated BTC regtest/signet gateway. | – |
| **7 – Reactor Benchmark** | GPU‑accelerated benchmark jobs that feed the proof engine. | – |
| **8 – Marketing / Grantsmith** | Generates marketing copy and grant drafts **only** from verified reports. | – |
| **9 – Public Testnet Candidate** | Final gate that runs `yolo_autoprove.sh` and validates all required reports exist. | – |

---

## Core Components
### 1. Feature Registry (`docs/FEATURE_REGISTRY.toml`)
Defines every X3 feature, its **mode** (`LIVE_TESTNET`, `GUARDED_TESTNET`, `SIM_TESTNET`, …), the crate or service that implements it, required tests, health endpoint and the proof report that must be generated.  The registry is the single source of truth for the **Readiness Engine**.

### 2. Testnet Feature Flags (`docs/TESTNET_FEATURE_FLAGS.toml`)
A lightweight flag file used by the swarm agents to decide which features are allowed to be auto‑patched during a YOLO run.  Features marked `DISABLED_BLOCKED` are never auto‑edited.

### 3. Swarm Core (`crates/x3-swarm-core`)
* Provides the data model for agents, tasks, policies and memory entries.
* Enforces **permission tiers** and **forbidden path guards** (e.g. `/.env`, `/private_keys`).
* Supplies default policies for each agent kind (RepoScanner, TestBuilder, BuildFixer, ApprovalGate, …).

### 4. Swarm API (`services/x3-swarm-api`)
A tiny Actix‑Web service exposing:
* `GET /health` – health check.
* `GET /agents` – list of
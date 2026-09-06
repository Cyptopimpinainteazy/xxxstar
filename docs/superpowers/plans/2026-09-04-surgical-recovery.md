# X3 Surgical Recovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore proven capabilities lost during repository consolidation without reintroducing obsolete, fake, placeholder, or insecure donor implementations.

**Architecture:** Recover one independently verifiable subsystem at a time. Preserve `xxxstar` as the source of truth, use donor repositories only as historical evidence, and adapt recovered behavior to current X3 interfaces instead of overwriting newer code.

**Tech Stack:** Rust 2021, Axum 0.8, SQLx/PostgreSQL, Redis, async-graphql, GitHub Actions, Substrate/FRAME.

**Spec:** Repository audit in the 2026-09-04 ChatGPT work session comparing `xxxstar`, `x3-atomic-star`, `atlas-sphere`, `atlas-sphere-master`, `atlas-sphere1`, and `atlas-sphere-`.

## Global Constraints

- Work only on `codex/surgical-recovery`; never commit directly to `main`.
- No fake adapters, relayers, proofs, no-op paths, placeholder logic, or silent security fallbacks.
- Donor code must be reconciled with current `xxxstar` behavior and tests.
- Every behavior change follows a recorded red-green cycle.
- Each subsystem must compile and pass its focused tests before the next subsystem begins.
- Do not restore generated output, vendored dependencies, environment files, malformed paths, or duplicate test trees.

---

### Task 1: Restore the X3 Gateway Entrypoint

**Files:**
- Create: `crates/x3-gateway/src/cache.rs`
- Create: `crates/x3-gateway/src/config.rs`
- Create: `crates/x3-gateway/src/error.rs`
- Create: `crates/x3-gateway/src/graphql.rs`
- Create: `crates/x3-gateway/src/orchestra.rs`
- Modify: `crates/x3-gateway/src/main.rs`
- Test: `crates/x3-gateway/tests/gateway_wiring.rs`

**Interfaces:**
- Consumes: existing `Database`, `create_router`, `ControlPlaneClient`, and current migration `0006_funding_swarm_public_ledger.sql`.
- Produces: a real `x3-gateway` process with configuration loading, database connection, optional Redis, REST/GraphQL routes, control-plane relay, bind validation, and graceful shutdown.

- [ ] **Step 1: Write a failing wiring regression test**

```rust
#[test]
fn gateway_binary_wires_the_http_service() {
    let source = include_str!("../src/main.rs");
    for required in ["Database::connect", "create_schema", "create_router", "axum::serve"] {
        assert!(source.contains(required), "gateway entrypoint is missing {required}");
    }
}
```

- [ ] **Step 2: Run the regression test and confirm it fails because the current entrypoint only logs**

Run: `cargo test -p x3-gateway --test gateway_wiring gateway_binary_wires_the_http_service -- --exact`

- [ ] **Step 3: Restore the five required modules and reconcile the entrypoint with current `db.rs`, `rest.rs`, and migration 0006**

The entrypoint must declare all modules, load `GatewayConfig`, connect `Database`, build `AppSchema`, construct optional `RedisCache` and `ControlPlaneClient`, call `create_router`, bind a validated `SocketAddr`, and serve with Ctrl-C/SIGTERM shutdown.

- [ ] **Step 4: Run formatting, the focused test, package tests, and package check**

Run: `cargo fmt --check -- crates/x3-gateway`

Run: `cargo test -p x3-gateway --test gateway_wiring`

Run: `cargo test -p x3-gateway`

Run: `cargo check -p x3-gateway`

- [ ] **Step 5: Commit the gateway recovery**

```bash
git add crates/x3-gateway
git commit -m "fix(gateway): restore production service entrypoint"
```

### Task 2: Reconcile Gateway Database Migrations

**Files:**
- Create: `crates/x3-gateway/migrations/0001_benchmark_reports.sql`
- Create: `crates/x3-gateway/migrations/0002_benchmark_report_workload_profile.sql`
- Create: `crates/x3-gateway/migrations/0003_benchmark_jobs.sql`
- Create: `crates/x3-gateway/migrations/0004_orchestra_workflows.sql`
- Create: `crates/x3-gateway/migrations/0005_vote_window_tally.sql`
- Preserve: `crates/x3-gateway/migrations/0006_funding_swarm_public_ledger.sql`
- Test: `crates/x3-gateway/tests/migration_chain.rs`

**Interfaces:**
- Consumes: SQL queries in current `db.rs`.
- Produces: an ordered, append-only migration chain containing every table and column referenced by gateway queries.

- [ ] **Step 1: Add a failing migration-chain test that asserts migrations 0001 through 0006 exist exactly once and are ordered**
- [ ] **Step 2: Run `cargo test -p x3-gateway --test migration_chain` and confirm missing 0001 causes failure**
- [ ] **Step 3: Restore migrations 0001 through 0005 without modifying 0006**
- [ ] **Step 4: Run the migration-chain test and `cargo test -p x3-gateway`**
- [ ] **Step 5: Commit with `git commit -m "fix(gateway): restore ordered database migrations"`**

### Task 3: Recover ProofGate Checks by Behavior

**Files:**
- Modify: `.github/workflows/proof-gates.yml`
- Modify: `.github/workflows/mainnet-readiness.yml`
- Create only missing current equivalents under: `scripts/mainnet/` or `scripts/proof/`
- Test: `scripts/tests/test_recovery_gate_wiring.py`

**Interfaces:**
- Consumes: current mainnet and proof scripts.
- Produces: hard-fail CI coverage for runtime WASM non-stub validation, cross-VM safety wiring, launch-validator enforcement, chain-spec/genesis drift, runtime identity, SBOM, and unresolved launch blockers.

- [ ] **Step 1: Write a failing test that maps each required gate to an existing executable script and workflow invocation**
- [ ] **Step 2: Run `python -m pytest scripts/tests/test_recovery_gate_wiring.py -q` and record absent mappings**
- [ ] **Step 3: Port only missing behavior from donor scripts into current script namespaces**
- [ ] **Step 4: Run the focused pytest, shell syntax checks, and workflow YAML parsing**
- [ ] **Step 5: Commit with `git commit -m "ci: recover missing production proof gates"`**

### Task 4: Recover Deployment Controls

**Files:**
- Modify or create focused assets under: `deployment/public-rpc/`, `deployment/systemd/`, and `deployment/monitoring/`
- Test: `deployment/tests/test_recovered_assets.py`

**Interfaces:**
- Consumes: current node CLI and environment variable names.
- Produces: public-RPC reverse-proxy controls, systemd node templates, endpoint validation, and monitoring configuration suitable for the current testnet.

- [ ] **Step 1: Write failing structural and command-line contract tests for the selected deployment target**
- [ ] **Step 2: Confirm the focused pytest fails on missing assets**
- [ ] **Step 3: Reconcile donor assets with current binary names, ports, health endpoints, and secrets policy**
- [ ] **Step 4: Run pytest, `systemd-analyze verify` where available, Nginx config validation where available, and YAML parsing**
- [ ] **Step 5: Commit with `git commit -m "ops: recover testnet deployment controls"`**

### Task 5: Design Production Swarm and Audit-Governance Recoveries

**Files:**
- Create: `docs/recovery/gpu-swarm-control-plane-spec.md`
- Create: `docs/recovery/audit-governance-integration-spec.md`

**Interfaces:**
- Consumes: current `x3-gpu-validator-swarm`, `swarm_infrastructure`, governance, court, invariant, receipt, and agent-law interfaces.
- Produces: implementation-ready security specifications replacing donor fake signatures, hard-coded metrics, skeleton state transitions, and unverified receipts.

- [ ] **Step 1: Map old capabilities to current owning modules and explicitly reject insecure donor mechanisms**
- [ ] **Step 2: Define cryptographic identity, replay protection, durable queues, signed receipts, authorization, slashing evidence, appeal timelocks, and rollback behavior**
- [ ] **Step 3: Define focused acceptance tests and failure-injection cases before production implementation**
- [ ] **Step 4: Run the repository fake-code scan against the specifications and ensure no placeholder instructions remain**
- [ ] **Step 5: Commit with `git commit -m "docs: specify secure swarm and audit recovery"`**

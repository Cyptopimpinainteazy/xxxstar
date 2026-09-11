# X3VM Live Transport Implementation Plan

> **For agentic workers:** Use the host's available task-by-task implementation workflow. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the X3VM atomic-swap adapter from simulation-only into a production-pluggable path that can submit real lock/claim/refund operations and consume verifiable chain proofs without weakening fail-closed behavior.

**Architecture:** Keep `X3VmAdapter` as the VM-agnostic public contract. Add a `std`-only live X3VM adapter backed by an injected transport interface; the transport owns signing/RPC/indexer integration and returns canonical `LockProof`, `ClaimProof`, `RefundProof`, `FinalityProof`, `FeeEstimate`, and `ChainHealth` values. The existing simulation adapter stays unchanged for deterministic offline tests.

**Tech Stack:** Rust 2021, `x3-atomic-swap`, existing canonical proof types, existing `std` feature gate, Cargo tests.

## Global Constraints

- Production code must never fabricate transaction ids, block hashes, finality, or raw proofs.
- Signing keys remain outside the adapter; the transport is injected by the host/relayer.
- Live responses must be rejected if chain id or VM type does not match the configured X3VM domain.
- Existing simulation constructors and deterministic tests remain available.
- No adapter may report production readiness merely because a transport object exists; readiness is transport-declared and proof-backed.

---

### Task 1: Add the production transport boundary

**Files:**
- Create: `crates/x3-atomic-swap/src/x3vm_live.rs`
- Modify: `crates/x3-atomic-swap/src/lib.rs`
- Test: `crates/x3-atomic-swap/src/x3vm_live.rs` unit tests

**Interfaces:**
- Consumes: `AtomicIntent`, `IntentId`, canonical proof/health/fee types, `X3VmAdapter`.
- Produces: `X3VmLiveTransport` and `LiveX3VmAdapter`.

- [ ] **Step 1: Add the focused failing test**

Add a recording transport and assert that lock/claim/refund/verify/finality/health/fee calls are delegated; reject proof values with a non-X3VM VM type or mismatched chain id.

- [ ] **Step 2: Verify the relevant failure**

Run: `cargo test -p x3-atomic-swap --features std x3vm_live`
Expected: test target fails before the module and live adapter exist.

- [ ] **Step 3: Implement the minimum behavior**

Define `X3VmLiveTransport: Send + Sync + Debug` with synchronous lifecycle, verification, fee, finality, health, and readiness methods. Define `LiveX3VmAdapter<T>` that owns a configured `chain_id`, `escrow_address`, and transport. Delegate calls and validate returned domain identity before exposing proofs to the relayer.

- [ ] **Step 4: Verify the focused pass**

Run: `cargo test -p x3-atomic-swap --features std x3vm_live`
Expected: all live-adapter unit tests pass.

- [ ] **Step 5: Run the affected integration check**

Run: `cargo test -p x3-atomic-swap --features std`
Expected: the crate's existing simulation/integration tests plus live-adapter tests pass.

- [ ] **Step 6: Commit the passing deliverable**

```bash
git add crates/x3-atomic-swap/src/x3vm_live.rs crates/x3-atomic-swap/src/lib.rs docs/plans/2026-09-09-x3vm-live-transport.md
git commit -m "feat: add production X3VM transport boundary"
```

### Task 2: Implement the native X3 node transport

**Files:**
- Proposed create: `crates/x3-atomic-swap/src/x3vm_node_transport.rs`
- Modify: `crates/x3-atomic-swap/src/lib.rs`
- Test: local-node integration test under `crates/x3-atomic-swap/tests/`

**Interfaces:**
- Consumes: `X3VmLiveTransport`, X3 node RPC endpoint, externally supplied signer/call encoder.
- Produces: real signed lock/claim/refund submissions and proof extraction from finalized X3 blocks.

- [ ] **Step 1: Add the focused failing integration test**

Against an X3 local node, submit a lock, wait for GRANDPA finality, claim with the correct preimage, and assert real non-empty tx id/block hash/raw proof values. Add separate refund-after-timeout coverage.

- [ ] **Step 2: Verify the relevant failure**

Run: `cargo test -p x3-atomic-swap --features std --test x3vm_live_node -- --ignored`
Expected: failure because the native transport is not yet wired.

- [ ] **Step 3: Implement the minimum behavior**

Encode the runtime calls already exposed by the X3 settlement/atomic pallets, submit signed extrinsics through the configured node RPC, wait for canonical finality, extract event/inclusion evidence, and map it to canonical proof types. The signer is provided externally and is never persisted by this crate.

- [ ] **Step 4: Verify the focused pass**

Run the ignored local-node integration test with the node endpoint/signer configured.
Expected: lock and exactly one terminal path (`claim` or `refund`) finalize with chain-derived proofs.

- [ ] **Step 5: Run the affected integration check**

Run: `cargo test -p x3-atomic-swap --features std`
Expected: all non-live tests pass; local-node test remains opt-in/ignored in generic CI unless a node service is provisioned.

- [ ] **Step 6: Commit the passing deliverable**

```bash
git add crates/x3-atomic-swap/src/x3vm_node_transport.rs crates/x3-atomic-swap/src/lib.rs crates/x3-atomic-swap/tests/x3vm_live_node.rs
git commit -m "feat: wire X3VM atomic swaps to live node"
```

### Task 3: Promote X3VM readiness only from real evidence

**Files:**
- Modify: `crates/x3-atomic-swap/src/x3vm_node_transport.rs`
- Modify: readiness/audit scripts that consume `AdapterReadinessScore`
- Test: X3VM readiness tests

**Interfaces:**
- Consumes: successful local/testnet proof evidence from Task 2.
- Produces: production readiness score that truthfully reflects live lifecycle/proof/finality capabilities.

- [ ] **Step 1: Add the focused failing test**

Assert a disconnected/unverified transport cannot report lifecycle/finality/RPC readiness; assert a proof-backed test transport can report only capabilities demonstrated by its evidence.

- [ ] **Step 2: Verify the relevant failure**

Run: `cargo test -p x3-atomic-swap --features std x3vm_readiness`
Expected: current readiness lacks transport-backed evidence semantics.

- [ ] **Step 3: Implement the minimum behavior**

Make readiness transport-declared but adapter-validated. Keep `proof_ledger_integration` and `cross_adapter_atomicity_test` false until those integrations are demonstrably wired and tested.

- [ ] **Step 4: Verify the focused pass**

Run the focused readiness tests.
Expected: no disconnected or simulation path can score as production ready.

- [ ] **Step 5: Run the affected integration check**

Run the crate test suite and the repository X3 adapter proof/readiness script.
Expected: X3VM score increases only for capabilities actually backed by live evidence.

- [ ] **Step 6: Commit the passing deliverable**

```bash
git add crates/x3-atomic-swap/src/x3vm_node_transport.rs scripts docs/reports
git commit -m "test: gate X3VM readiness on live proof evidence"
```

## Unresolved externally observable decisions

- The exact runtime call names/indices and event schema for X3 native lock/claim/refund must be taken from the current runtime metadata when Task 2 is implemented; they should not be guessed in the adapter.
- Whether the generic CI environment should boot an X3 local node for live integration tests or leave those tests opt-in is a repository operations decision.

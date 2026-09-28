# Workstream B — Northern Swarm executor ↔ chain contract and the compute backends (P0, PR #519)

You are a lane of the X3 engineering team. Repo: `/home/lojak/Desktop/xxxstar-main`, current branch
already contains the merge of `release-gate/swarm-reactor-onchain-v1` (PR #519). Read `AGENTS.md`
first, then this brief.

## Context

* The primary agent is regenerating `pallets/northern-swarm/src/weights.rs` right now, so anything
  that builds the pallet will fail for ~20 minutes. Start by reading, then build.
* Your files are `crates/northern-swarm/**` (executor, `chain_watcher.rs`, `result_submitter.rs`,
  `backend.rs`, `executor.rs`, `types.rs`) and any test file *inside that crate*. Do **not** touch
  `pallets/**`, `runtime/**`, `scripts/mainnet/swarm_reactor_gate.py`, or
  `pallets/northern-swarm/src/weights.rs`.

## Part 1 — the executor must consume what the pallet actually exposes

`scripts/mainnet/swarm_reactor_gate.py` already forbids the old shortcuts (`PendingTasks` storage,
`pallet_index: 82`, an unsigned `submit_result`). Your job is to prove the *contract* is real, not
just absent:

* the watcher discovers tasks from the storage/events the pallet actually has, decoded through
  runtime metadata rather than a hard-coded layout (check `chain_watcher.rs`);
* `submit_result` is signed by the executor account that registered on chain, and the call is
  encoded from metadata (no hard-coded pallet/call indices);
* the executor observes inclusion/finality before treating a result as submitted, and recovers
  safely after a restart (state on disk, not in memory).

Add tests for: wrong key, unregistered key, already-claimed task, already-finalized task, duplicate
result submission, bad payload URI, bad payload hash. Where the crate cannot test against a live
node, say so in the test's doc comment and test the boundary itself (the encoder/decoder, the
signer selection, the restart recovery) rather than asserting nothing.

## Part 2 — `ComputeBackend`, `CpuBackend`, `GpuBackend`, `AutoBackend`

`crates/northern-swarm/src/backend.rs` exists. Finish it to the contract the release prompt names:

```rust
trait ComputeBackend {
    fn name(&self) -> &'static str;
    fn supports(&self, kind: TaskKind) -> bool;
    fn execute(&self, task: &TaskPayload) -> Result<TaskOutput, ComputeError>;
}
```

Required: a CPU implementation that is the reference, a GPU implementation that is used only when it
supports the task kind, and an `AutoBackend` that detects hardware, selects, executes, verifies when
required, falls back to CPU, and records telemetry. **Consensus-sensitive output must equal the CPU
reference** — a mismatch must quarantine the accelerator, record the divergence, and return the CPU
result; it must never silently prefer the accelerator. Reuse what exists (`x3-accel`,
`x3-accel-wgpu`, `x3-gpu-validator-swarm` parity code) rather than growing a fourth copy.

Tests: parity on a task both backends support, fallback when the accelerator refuses the kind, and a
deliberately-divergent fake accelerator that must be quarantined and must not change the result.

## Rules

- Do not weaken or skip tests; no `unwrap()`/`expect()` on production paths in this crate (the panic
  ratchet counts them).
- `cargo test -p northern-swarm --all-targets` must pass, and `cargo clippy -p northern-swarm
  --all-targets -- -D warnings` must be clean for what you touch. Use
  `CARGO_TARGET_DIR=/tmp/x3-target-swarm-b` to avoid the shared build lock.
- Commit only your own paths. **Never `git add -A`. Do not push** — message me (`/root`) with hashes,
  commands and results.

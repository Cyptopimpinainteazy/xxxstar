# Internal Mainnet Happy-Path E2E Tests

## Purpose
This document describes what the internal mainnet happy-path test suite covers in
`tests/e2e/src/internal_mainnet_happy_path.rs` and what it does not prove.

The suite is intended to validate core flow safety for:
- lock -> mint style cross-VM movement,
- swap execution and fee accounting,
- atomic rollback behavior,
- emergency halt and restart,
- replay protection,
- packet settlement lifecycle,
- invariant violation prevention.

## Current Reality
Implemented now:
- A mock-backed `TestEnvironment` and `MockState` are used for deterministic,
  local flow checks.
- The file defines seven async tests that exercise the critical flow sequence
  and validate expected state transitions.
- Bridge replay safety is explicitly checked with nonce + proof-hash reuse
  rejection in `execute_bridge_mint_with_proof_nonce`.
- Bridge supply discipline is checked using `check_bridge_invariants`
  (`wrapped EVM <= native locked supply`).
- A strict live-node lane exists in `tests/e2e/live_internal_mainnet_e2e.rs`
  and can be fail-closed with `X3_E2E_REQUIRE_NODE=1`.
- The live lane validates:
  - finalized-head progress and required RPC method availability,
  - cryptographic sign/verify checks via `x3_sign_ed25519` and
    `x3_verify_signature` for bridge-proof hash material,
  - lock/mint accounting invariants across fee paths, slashing interaction,
    per-route cap checks, and decimal-conversion edge cases,
  - failure paths for timeout expiry, reordered delivery, and duplicate
    acknowledgements.

Not implemented in this file:
- No real cross-chain relay, finality engine, or proof verifier is connected for adversarial or multi-chain scenarios (future work).
- No adversarial networking, latency, or reorg model is exercised yet; only happy-path and basic failure-paths are covered.

## Verified
The following has been repeatedly validated in this workspace:
- `cargo +1.90.0 test --manifest-path tests/e2e/Cargo.toml internal_mainnet_happy_path::tests::test_cross_vm_settlement_with_packets`
- `cargo +1.90.0 test --manifest-path tests/e2e/Cargo.toml --test live_internal_mainnet_e2e`
- `cargo +1.90.0 test -p x3-asset-kernel-types --lib`
- `cd x3-lang && cargo +1.90.0 test --workspace --all-targets`
- `bash scripts/mainnet/rc2_mock_and_live_gate.sh`
- `bash scripts/mainnet/run_release_gates_rc6.sh` now includes a mandatory
  `rc2_mock_and_live_gate.sh` step.

Observed behavior from the focused settlement test:
- First bridge mint with a fresh `(nonce, proof_hash)` succeeds.
- Replay of the same `(nonce, proof_hash)` is rejected.
- Packet lifecycle reaches `pending -> settled -> acknowledged`.
- Wrapped-mint invariant remains satisfied.

## Gaps / Risks
- The mock harness alone does not prove full runtime dispatch, but the enforced live lane and RC6 gate now require real node, pallet dispatch, and RPC execution for all critical flows.
- Live proof checks validate cryptographic hash-signature flows and deterministic proof integrity, but there is still no dedicated RPC endpoint that submits and verifies full bridge proof objects end-to-end through final settlement dispatch (this is the main remaining gap).
- Live packet failure tests model ordered delivery and acknowledgement discipline using live finalized height as timing source; they are not yet driven by a full external relayer process (future work).

## Release Impact
- This suite is useful for regression detection and invariant discipline during
  development.
- It is not sufficient, by itself, for public testnet or mainnet sign-off.
- Public release readiness still depends on integration tests that execute
  against real pallets/adapters and real proof/finality paths.

## Next Required Work
1.Add a dedicated bridge-proof verification RPC/pipeline that execut es full
  proof objects through the real settlement path, then assert it from the live
  lane.
2. Extend live failure tests to use a real relayer flow (not only harness-side
  sequencing logic) for packet ordering and partial-delivery recovery.
3. Add this live lane to CI policy for protected branches so the RC6 gate is
  enforced outside local release runs.

# T5+ Review-Required Proposals

Status as of cleanup pass (2026-05-22).

## t5-1 (node/src/service.rs — GPU sidecar health check)
- **Status: RESOLVED** — Circular stub removed. Health monitor loop now calls
  `orch_for_monitor.read().await.health_check().is_ok()` and feeds the real result
  into `health_monitor.record_check(healthy, block)`. Dead-code TX_POOL constants
  removed; replaced with a doc comment referencing `NetworkSpeed::detect()`.
- Remaining long-term: add sidecar process-liveness probe (OS PID check) and
  optional RPC ping to the sidecar control endpoint before calling `trigger_restart`.
  Create separate PR when the sidecar control API is stable.

## t5-2 (node/src/service.rs — cross-VM bridge dispatcher)
- **Status: RESOLVED** — Dispatcher wiring was already fully implemented
  (`RuntimeCrossVmDispatcher`, `CrossVmBridge`, `CrossVmBridgeSafetyGate`,
  `execute_pending_with_dispatcher`, preflight/postflight checks, failure backoff).
  Stale C-002 and TODO(t5-2) comments removed.

## t5-3 (runtime/src/precompiles.rs)
- **Status: OPEN** — Precompile TODO markers may affect gas accounting or address
  checks. Do not change runtime logic without governance/consensus review. Draft
  concrete changes and tests in a separate RFC before merging.

## t5-5..t5-7 (pallets/x3-invariants/src/lib.rs)
- **Status: OPEN** — Invariant-related governance controls and emergency authority
  APIs. Flagged by scanner; changes require unit tests that exercise invalid
  constitution hash / authority expiry. Do not enable chain-halting by default.
  Open a governance RFC before merging.

---

Next steps:
1. t5-3: Open RFC with concrete diff + tests for precompile gas/address changes.
2. t5-5..t5-7: Add unit tests for invariant edge cases, then governance proposal.

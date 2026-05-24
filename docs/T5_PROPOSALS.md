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
- **Status: RFC DRAFTED** — see [`docs/rfc/RFC-t5-3-precompile-gas-addresses.md`](rfc/RFC-t5-3-precompile-gas-addresses.md)
- Precompile TODO markers affect gas accounting and address validation.
  Do not merge without benchmarks, governance review, and testnet soak.

## t5-5..t5-7 (pallets/x3-invariants/src/lib.rs)
- **Status: RFC DRAFTED** — see [`docs/rfc/RFC-t5-5-7-invariant-governance.md`](rfc/RFC-t5-5-7-invariant-governance.md)
- Invariant governance controls and emergency authority APIs need events,
  boundary tests, timelock on pause, and bounded violation storage.
  Do not enable chain-halting by default. Requires governance RFC sign-off.

---

Next steps:
1. t5-3: Assign RFC reviewer; run precompile benchmarks; open PR after testnet validation.
2. t5-5..t5-7: Assign RFC reviewer; add unit tests for boundary cases; governance proposal.

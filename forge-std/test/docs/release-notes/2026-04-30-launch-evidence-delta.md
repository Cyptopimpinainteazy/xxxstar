# Launch Evidence Delta - 2026-04-30

## Scope

This entry records the final launch-evidence closure after the 96% GO report:

- Added observability claim routing in ProofForge:
  - `x3.observability.*` now resolves through the operational runner path.
- Registered and verified observability claim:
  - `x3.observability.telemetry_pipeline`
  - Receipt: `proof/receipts/claims/x3.observability.telemetry_pipeline.receipt.json`
- Regenerated go/no-go report with updated claim coverage:
  - `launch-gates/reports/X3-MAINNET-GO-NO-GO-20260430-202628.md`

## Outcome

- Decision: **GO**
- Overall score: **100%**
- P0 blockers: **0**
- P1 blockers: **0**

## Notes

- This delta is evidence and scoring coverage only; it does not alter runtime consensus, pallet economics, or chain state transition rules.

## Post-GO Follow-up (2026-05-01)

- Future-incompat audit (`cargo report future-incompatibilities --id 1`) reports:
  - `trie-db v0.30.0` (`never type fallback` warning; Rust 2024 hardening path)
  - Upstream guidance indicates `trie-db v0.31.0` as the first newer line.
- Dependency-path check confirms this is introduced via older SDK lanes still present in workspace graph:
  - `polkadot-sdk stable2506`
  - `polkadot-sdk stable2509-7`
- Current status:
  - **Non-blocking for current launch evidence gates** (GO remains valid)
  - **Action required for forward toolchain hardening**
- Planned remediation:
  1. Converge runtime/node-critical lanes to a single SDK line where feasible.
  2. Eliminate `trie-db v0.30.0` from the resolved graph (prefer SDK bumps over ad hoc crate patching).
  3. Re-run `cargo report future-incompatibilities` and attach a clean report in launch evidence.

### Lane Convergence Probe (2026-05-01, late)

- Frontier branch availability check:
  - `stable2509-7` does **not** exist upstream on `polkadot-evm/frontier`.
  - Nearby available lanes: `stable2506`, `stable2512`.
- Workspace probe switched Frontier deps to `stable2512` for convergence testing.
- Resulting graph status:
  - `stable2506` is removed from Frontier paths.
  - `trie-db v0.30.0` still resolves through `polkadot-stable2509-7` portions of the workspace.
  - `trie-db v0.31.0` now also appears through newer lanes, confirming mixed-lane coexistence.
- Future-incompat status remains open:
  - `cargo report future-incompatibilities --id 1` still reports `trie-db v0.30.0`.

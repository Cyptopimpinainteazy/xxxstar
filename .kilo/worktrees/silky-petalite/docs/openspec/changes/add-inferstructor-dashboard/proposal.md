# Change: Consolidate Inferstructor Operator Dashboard

## Why
The operator surface is fragmented across dashboards and inconsistent naming. We need a single authoritative control plane with professional, grounded copy and clean multi-page navigation.

## What Changes
- Rename **Inferstructor → Inferstructor** across UI, docs, and routes.
- Consolidate the best operational features into `apps/inferstructor-dashboard/`.
- Add multi-page admin flows for validators, swaps, proofs, faucet, and funding controls.
- Standardize API wiring for metrics, leaderboards, and audit logs.

## Impact
- Affected specs: new `inferstructor-dashboard` capability.
- Affected code: `apps/inferstructor-dashboard/` (rename), related admin APIs, docs references.
- UX: requires consistent navigation and non‑hype copy aligned with production operations.

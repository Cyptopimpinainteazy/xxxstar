# Tasks — add-swarm-orchestra-platform

- [x] Prerequisite: enforce the authority startup determinism gate in [node/src/service.rs](../../../node/src/service.rs) before any new automation surface is enabled.
- [x] Prerequisite: derive runtime governance periods from the 200ms block target in [runtime/src/lib.rs](../../../runtime/src/lib.rs) instead of the obsolete 6-second timing constants.
- [x] Replace the draft proposal text with a code-truthful architecture description that excludes deprecated swarm code from the production validator path.

## Phase 1 — Gateway

- [x] Add gateway REST endpoints for orchestra intents, approval cases, vote windows, and evidence-bundle lookup under [crates/x3-gateway/src/rest.rs](../../../crates/x3-gateway/src/rest.rs).
- [x] Add matching GraphQL query and mutation surfaces for orchestra workflows in [crates/x3-gateway/src/graphql.rs](../../../crates/x3-gateway/src/graphql.rs).
- [x] Extend gateway persistence in [crates/x3-gateway/src/db.rs](../../../crates/x3-gateway/src/db.rs) with truth-based tables for `orchestra_intents`, `approval_cases`, `vote_windows`, `vote_receipts`, and `evidence_bundles`.
- [x] Add gateway integration tests proving benchmark reports, intents, approvals, and evidence reads remain segregated by workflow type.

## Phase 1 — Sidecar

- [x] Add provider-onboarding benchmark job templates to [crates/x3-sidecar/src/benchmark.rs](../../../crates/x3-sidecar/src/benchmark.rs) so sidecar can produce comparable onboarding reports instead of ad hoc benchmark runs.
- [x] Extend [crates/x3-sidecar/src/rpc.rs](../../../crates/x3-sidecar/src/rpc.rs) to expose onboarding benchmark submission and publish-status APIs distinct from general execution jobs.
- [x] Add signed provider and hardware metadata fields to sidecar benchmark artifacts, then verify gateway publication rejects unsigned or incomplete onboarding reports.
- [x] Add sidecar integration tests covering benchmark run creation, report publication, and executor-registration checks through [crates/x3-sidecar/src/submitter.rs](../../../crates/x3-sidecar/src/submitter.rs).

## Phase 1 — Orchestra Control Plane

- [x] Create a new off-chain orchestra-control-plane service owning intent intake, approval routing, CRM vote windows, evidence bundles, and reward accrual state.
- [x] Define the canonical `Intent`, `ApprovalCase`, `VoteWindow`, `VoteReceipt`, and `EvidenceBundle` schemas and publish them as the sole source of truth for off-chain workflow lineage.
- [x] Implement policy gating so validation and benchmarking intents may run automatically, while publication, sanctions, treasury-affecting actions, and strategy activation require approval state before dispatch.
- [x] Add a CRM adapter interface for voter eligibility snapshots, ballot fan-out, and closed-window tally import without moving that workflow into the runtime.
- [x] Add orchestra integration tests proving intents cannot bypass approval, vote windows close deterministically, and every externally visible action yields an evidence bundle.

## Cross-Cutting Follow-Up

- [x] Wire orchestra events into the security swarm evidence pipeline and register the resulting invariants in [tests/invariants/registry.toml](../../../tests/invariants/registry.toml).
- [x] Define the first operator dashboard slice combining approvals, incidents, benchmark health, and evidence status across gateway, sidecar, and orchestra-control-plane.

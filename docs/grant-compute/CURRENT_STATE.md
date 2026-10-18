# Grant and volunteer compute audit

Inspected on 2026-10-18. This is a source and GitHub audit, with limited host
inspection. It is not an assertion of deployment or GPU isolation safety.

## Repository and branch evidence

The user's checkout was `feat/x3-guardian` at
`1beb644603a0f1b6de8bb5882e699fdc99bb2e58`, with extensive staged, unstaged and
untracked changes. They were preserved. Implementation uses an isolated
worktree, `codex/grant-compute-audit-20261018`, based on fetched master
`70423054e2d9560a212ab763d4f265f9cdc448d9`.

`origin/codex/x3-grant-hunter-v3` equals that master commit: there are no
branch-specific Grant Hunter changes to recover. PR #626 is OPEN at
`9094f71e297323ef000c6c4e47945f79af17735a`, based on master. PR #627 is OPEN at
`eff2ec8b98ea133125bd9d081ab5d1d0668c0a5a`, based on #626's branch; it changes
only volunteer CI configuration. Neither was merged by this work.

All local and fetched origin refs were inventoried in
`.ai/runlogs/grant-compute-20261018/refs.txt`. Recently merged PRs #610–622
primarily implement canonical X3VM/compiler functionality, not grant discovery.
Open work includes #623–628 and infrastructure/CI branches. Their code must not
be assumed to exist in master. PR #627 must remain stacked above #626 until its
base is verified; unrelated language, DEX and testnet work should land separately.

## Components and reusable interfaces

| Component | Actual source behavior | Evidence level |
|---|---|---|
| Gateway funding ledger | `crates/x3-gateway/src/db.rs` and `rest.rs`: PostgreSQL grant intake, pagination, public scoreboard/timeline, token-gated admin API | IMPLEMENTED; tests exist, current run results recorded separately |
| Funding state machine | `compute_swarm_transition` validates stages and constructs audit events, but baseline DB writes bypass it | IMPLEMENTED helper, INTEGRATION GAP |
| Grant drafting | `AiProvider` returns either a draft notice or a prompt string; `OpenRouterProvider` does not itself make an API request | PARTIAL; not an evidence-backed application builder |
| Desktop CRM | `apps/x3-desktop/src-tauri/src/crm/funding.rs`, related war-plan modules and SQLite migrations | IMPLEMENTED records; not official source discovery |
| Funding frontend | `apps/x3-funding` React/Vite app with static strategy data and wallet hook | IMPLEMENTED presentation; not grant/compute administration |
| Inferstructor dashboard | `apps/inferstructor-dashboard`: React/Vite, API client, token handling, admin/RBAC panels, Vitest tests | REUSABLE frontend; backend role enforcement needs endpoint audit |
| Swarm core | `crates/x3-swarm-core`: task queue, identity authority, permissions, sanctions and scheduling | REUSABLE agent orchestration; not a volunteer GPU sandbox |
| GPU swarm | `crates/gpu-swarm/src/scheduler.rs`: capability/priority/reputation scheduling; funding module schedules outreach campaigns | IMPLEMENTED logic; not evidence of live compute or grant eligibility |
| Legacy worker | `services/x3-swarm-worker/src/main.rs`: task polling and `bash -c` execution of task commands | UNSUITABLE for volunteer machines |
| Volunteer #626 | Python authenticated dispatch, HMAC lease verification, SQLite nonce protection, GPU inventory, fixed CPU hashing subprocess; Rust policy and in-memory lease guard | PARTIAL; not durable enrollment, GPU execution, isolation or signed public receipts |
| Chain receipts | Existing runtime, receipt registry and validator attestation code | Requires protocol-specific review before reuse for compute; signatures do not prove computation |

The gateway already owns PostgreSQL migrations and optional Redis cache.
`Database::connect` migrates configured databases; DB-free startup reports
degraded readiness rather than returning fabricated data. Reuse this persistence
and API rather than creating a competing grant service or database. Desktop
SQLite and #626's local replay SQLite have separate, justified local roles.

Existing identity surfaces include runtime account/wallet pallets, the desktop
wallet hook, swarm authority records and Inferstructor token handling. None
constitutes verified third-party worker enrollment by itself. Monitoring includes
gateway health/readiness, dashboard telemetry, validator metrics and deployment
Prometheus configuration. Volunteer health and capacity must come from measured
worker activity, not these unrelated metrics.

## Security findings at the audited base

P0: Do not connect volunteers to the legacy shell worker. Its command substring
permission checks cannot confine shell execution. #626 deliberately keeps its
fixed-function dispatcher separate; retain that boundary.

P1: Migration `0006_funding_swarm_public_ledger.sql` seeds fictional opportunities,
public submission/award events and publications. Public queries count a fictional
USD 120,000 award. Historical migration checksums must remain intact; quarantine
these rows using a forward migration without deleting them.

P1: `admin_advance_funding_swarm_stage` overwrites arbitrary stage/status pairs
without checking current state or inserting the available audit event. The
`award-paid` API writes `stage=submitted,status=award_paid` without award or payment
evidence. Reuse the validator with transactional row locking and fail closed on
unsupported award operations.

P1: #626's Rust lease guard has no authentication or persistent replay store and
is not wired to execution. Its Python HMAC authenticates a shared-key holder,
not an independently verifiable worker signature. Strict JSON types, replay-store
failure behavior, identity provisioning, consent persistence and execution expiry
need further review before activation.

P1: API admin tokens are shared secrets; a development feature bypass exists.
Per-user roles, attributable approvals, rate limits and audited security policy
changes remain required for production administration. Public metadata and
publication payloads also need a confidentiality review.

## Tests and CI

Existing gateway tests include state-machine unit tests, funding REST tests,
migration table coverage and a real PostgreSQL integration test invoked with
`scripts/gateway-postgres-integration.sh`. The PostgreSQL test is ignored in
ordinary cargo tests; only its explicit execution counts as integration proof.
Baseline integration assertions expect the fictional seed grants, so replace
that expectation with proof of retained-but-excluded examples.

PR #626's lease-tests checks reported FAILURE; GitHub did not return a failed
step log for the inspected run, so its cause is unverified. #627's lease-tests
and #626's local-ci were QUEUED at inspection, not PASS. #627 also had an external
analysis ERROR. Latest master's local-ci run was CANCELLED; two compiler checks
were SUCCESS. These are not full-repository green evidence.

Master protection requires `x3/local-ci`; strict status checking is disabled.
`.github/workflows/local-ci.yml` checks out the exact PR head, guards self-hosted
runners against fork PRs and publishes the full gate set's status. Preserve it.
Raw check snapshots and protection configuration accompany this audit.

## Infrastructure inspection

The local host identifies as `x3star1`; neither `nvidia-smi` nor `rocm-smi` was
found on its PATH. Name lookup resolves x3gpu1. A read-only SSH attempt through
the existing configured host failed with `No route to host`, exit 255. x3star2
and x3gpu2 did not resolve through the inspected local name service. Their
capabilities, consent, running services and deployment roles are UNVERIFIED.
No machines were enrolled, no workloads deployed and no existing roles changed.
Docker is reachable locally; a cached PostgreSQL 16 Alpine image permits
disposable database integration tests without touching production databases.

## Implementation order and acceptance

First repair funding provenance and transactional lifecycle enforcement, proving
rejected jumps, duplicate/concurrent actions, audit rollback and restart behavior
against disposable PostgreSQL. Next implement official source adapters with
bounded retrieval, provenance and deterministic scoring in the existing gateway.
Keep eligibility UNKNOWN without organizational evidence. Then add versioned
evidence-backed drafts, attributable human approvals and separate submission,
verified award, cash, commitment and expenditure records.

In parallel by dependency, review #626/#627 without merging unverified changes;
harden durable worker identity/consent and authenticated leases before approved
isolated GPU kernels. Reuse compatible scheduling interfaces, then implement
result verification and canonical signed receipts, grant/project budgets and
milestones, and finally real-data dashboard integration. Infrastructure deployment
and live redundant GPU verification require reachable, explicitly opted-in hosts.
External submission, financial commitments and new workload activation remain
human-controlled. No end-to-end or production-readiness claim is supported yet.

## Later read-only hardware discovery (2026-10-18)

The initial configured GPU address was stale. Parsing only endpoint hosts from
existing router configuration located x3gpu1 at 192.168.0.29; trusted-key,
BatchMode SSH and read-only nvidia-smi succeeded. Actual discovery reported a
GTX1070 (8192MiB) and RTX2060SUPER (8192MiB), driver580.178.04. Python3,
cryptography and bwrap exist; torch, numpy and nvcc were absent. No package
installation, enrollment, GPU job, power measurement or public workload activation
occurred. Reachability is not consent, isolation proof or executed research.

The local development host reports two XeonE5-2620v4 sockets (16 physical cores,
32 logical CPUs) and approximately109GiB RAM. This does not validate historical
fleet counts, free capacity, electrical cost, cooling or model compatibility.
Manufacturer evidence and measured telemetry are still required for those claims.

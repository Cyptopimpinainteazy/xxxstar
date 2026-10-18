# V2/V3 forensic capability audit

This extends [CURRENT_STATE.md](CURRENT_STATE.md), audited against fetched
master `70423054e2d9560a212ab763d4f265f9cdc448d9` on 2026-10-18. Its branch
inventory, PR #626/#627 hashes and security findings apply here. V1 and V2 have
not demonstrated their required end-to-end workflow; V3 must build on verified
milestones rather than assuming those versions are finished.

## End-to-end traces

| Surface | Actual execution path | Missing link |
|---|---|---|
| Public funding | `site/funding-swarm.html` fetches `/api/public/funding-swarm/{scoreboard,grants,timeline}` → `crates/x3-gateway/src/rest.rs` → `db.rs` → PostgreSQL `funding_swarm_*` | Historical examples contaminate live metrics at the base; no discovery refresh worker |
| Admin funding | `/admin/funding-swarm/grants` and action routes → shared admin token check → `Database` writes | Stage validator bypassed; no separate evidence-controlled submission/award/payment lifecycle |
| Funding strategy | `apps/x3-funding/src/App.tsx`, `FundingStrategyPage.tsx`, `data.ts` → static strategy and wallet components | No funding-intelligence API or verified live opportunity feed |
| Desktop funding | Tauri CRM funding/war-plan commands → local CRM data and SQLite schema | Organizational records do not establish verified eligibility; no demonstrated official adapter bridge |
| Drafting | `AiProvider::draft_application` in gateway `db.rs` → formatted text | No repository evidence retrieval, approved section library, funder templates, document versioning or verified model call |
| Agent orchestration | `services/x3-swarm-api/src/main.rs` → `crates/x3-swarm-core` authority/permissions/scheduler → legacy worker | Scheduler/identity stores are in-memory; no durable bounded grant-research DAG, scientific evidence contracts or cost accounting |
| Volunteer branch | #626 Python `dispatch.py` → `LeaseVerifier.accept` → SQLite used nonce → `safe_benchmark.execute` subprocess → CPU digest comparison | No real GPU backend, persistent owner enrollment, signed public receipt, reliable telemetry or integration with scheduler |
| Inferstructor administration | `apps/inferstructor-dashboard/src/api.ts`, `api-client.ts`, admin components → configured infrastructure APIs | No grant intelligence or research project endpoints wired into this interface |
| GPU receipt primitive | `crates/x3-gpu-validator-swarm/src/gpu_receipt.rs` → domain-separated attestation message → expected validator-key verification | Atomic execution receipt semantics are not automatically research-job semantics; adaptation needs input/software/output/project binding and live proof |
| Chain path | Runtime `Swarm`, `DepinMarketplace`, `NorthernSwarm` variants and their Config implementations in `runtime/src/lib.rs` | Presence in a runtime is not proof of volunteer-safe enrollment/execution or research commitment integration |

## Historical reuse and duplication

`43be3bbb7238d090c45b5e0b893d85defe4d22b1` repaired shared gateway/indexer
migrations and real PostgreSQL integration. Gateway owns migrations 0001–0007;
indexer links them, and gateway links the indexer's initial schema. Add every
new shared migration to both sets without rewriting existing checksums. Existing
`build.rs` files watch the migration directories so new files are embedded.

`4dd1f3470c` tightened sensitive-action authorization in swarm core. Preserve
that authority boundary, but do not treat its in-memory store as a durable
research queue. The GPU swarm scheduler and agent swarm scheduler target
different task types; adapt their verified interfaces rather than merging them
by name. `tools/cross-chain-gpu-validator` and `infra-structure/validator` contain
similar validator code; determine deployment ownership before extending either.

The funding campaign orchestrator schedules marketing/outreach campaigns; it is
not grant eligibility analysis and must not be reused to automatically contact
funders. The proposed research system may perform local analysis and drafting;
external communication and commitments require explicit authorization.

## V3 success matrix

PASS requires a reproducible end-to-end command for the exact implementation
head, not file presence. These are audit baseline states, not claims that all
implementation work has been attempted.

| Success criterion | Baseline implementation | Verification state |
|---|---|---|
| Real funding discovery | No canonical adapter found | NOT RUN |
| Official source verification | Grants.gov documents public `search2` and `fetchOpportunity`; live read-only search investigated during this mission | Partial external evidence only |
| Fact-based eligibility | No verified organization evidence model found | NOT RUN |
| Research project proposal | No durable structured research planning pipeline found | NOT RUN |
| Evidence-backed application draft | Existing provider returns prompts/text | NOT RUN |
| Human-controlled submission | Token gate exists; lifecycle can be bypassed at base | FAIL by source inspection; regression pending |
| Verified award or labeled pilot project | No verified award-document workflow found | NOT RUN |
| Approved compute allocation | No grant-to-worker allocation path demonstrated | NOT RUN |
| Real opted-in GPU execution | #626 executes fixed CPU hashing only; GPU host unreachable | BLOCKED for live host verification |
| Computation verification | CPU digest reference exists on #626; no research GPU result proof | NOT RUN |
| Signed research execution receipt | Existing chain attestation is not a research receipt integration | NOT RUN |
| Research milestone record | No grant/project/job linkage demonstrated | NOT RUN |
| Auditable research report | No full execution-to-report path found | NOT RUN |
| Integrated control center | Existing dashboard has no demonstrated full research workflow | NOT RUN |

No aggregate product completion percentage is justified by file counts. At the
audit baseline, zero of these fourteen full workflow criteria has PASS evidence.
Source-level findings and a live official-source read are narrower evidence.

## Missing foundations and next verification

Repair real funding state transitions and example provenance first. Add official
source ingestion with normalized snapshots, uniqueness, source health and bounded
requests. Keep official source data inert: it cannot instruct agents, approve
eligibility or issue execution commands. Add deterministic scoring with UNKNOWN
eligibility unless verified organizational facts satisfy every requirement.

Research agents need structured evidence-bearing messages, durable dependencies,
bounded retries, cancellation, timeouts and cost ceilings before unattended use.
Knowledge retrieval needs provenance/freshness and injection tests before model
drafting. Artifact/lease verification and worker opt-in must precede GPU kernels;
independent isolation review and reachable opted-in hardware precede public
enrollment. No deployment or external application submission has been performed.

Exact executed commands and results belong in `V2_TEST_REPORT.md` and
`V3_TEST_REPORT.md` as milestones finish. RUNNING checks do not satisfy this
matrix. There is no evidence yet for full workspace, mainnet, live GPU,
multi-coordinator, grant award, semantic-search or end-to-end research readiness.

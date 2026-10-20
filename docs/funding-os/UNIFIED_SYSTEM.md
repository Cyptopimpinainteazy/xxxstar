# X3 Funding OS — one system architecture decision (2026-10-18)

**Decision:** one X3 Funding OS with a canonical, auditable PostgreSQL operations ledger, multiple independently replaceable source adapters, and multiple views. The historical grant catalog, hardware desktop subsystem, sponsor website and GPU coordinator are **inputs/clients**; they must not be rival funding ledgers. This is a target architecture and integration sequence, not a claim that it has shipped.

## Existing code to reuse

| Existing component | Evidence | Role in unified system | Present limitation |
|---|---|---|---|
| Historical opportunity catalog | `grants/INDEX.md` and 12 Markdown files | **Read-only lead feed** until verified | Index estimates ~1,772 rows; 1,617 parseable source IDs inspected. Sources/amounts/statuses are historic, many not grants. |
| Sponsor and funding site | `apps/x3-funding/src/App.tsx`, `data.ts`, `data_angles.ts`, `FundingStrategyPage.tsx` | Public **presentation** and approved sponsor intake | Much content is static; some claims and contact flows require review. |
| Funding ledger | `crates/x3-gateway/src/db.rs`, PR #630 | Canonical opportunity/review/event persistence | #630 is draft; approval/award false positives in older code were addressed there, not yet merged. |
| Official source evidence | `crates/x3-gateway/src/funding_discovery.rs`, migrations 0009/0010, PR #632 | Source adapters and immutable retrieved snapshots | Currently Grants.gov only; no Web3 or broad browser agent. |
| Proposal workspace | `crates/x3-gateway/src/funding_proposals.rs`, migration 0011, PR #633 | Immutable, source-bound drafts + private approvals | Draft packages only; not a sending/awards system. |
| Operator UI | `apps/inferstructor-dashboard/src/components/MoneySwarmWorkspace.tsx`, PR #632/#633 | **Single private operations UI** | Should gain typed funding lanes, review and hardware workspaces incrementally. |
| Physical hardware sourcing | `apps/x3-desktop/src-tauri/migrations/hardware_acquisition.sql`, `src/crm/hardware_sources_db.rs` | Equipment-donation/loan/surplus workflows | Existing SQLite schema and hardcoded unverified leads are not automatically a live backend. |
| Volunteer compute | PRs #626/#627, Inferstructor integration | External **resource provider**; verified donated compute attaches to projects | No general safe public GPU execution or settlement readiness established. |

## One canonical model (target; not an applied migration)

1. `organization`: funder/sponsor/contributor/legal applicant; private contacts and verified organization identity stored separately with access controls.
2. `source`: provider adapter and authoritative official source URL, source health and retrieval policy.
3. `opportunity`: stable provider/ID identity, **kind** (`cash_grant`, `dao_treasury`, `bounty`, `hackathon`, `equity_investment`, `hardware_donation`, `hardware_loan`, `hardware_discount`, `cloud_credit`, `gpu_credit`, `in_kind_support`, `research_contract`, `commercial_lead`, `other`), deadlines, currency, current source status.
4. `evidence_snapshot`: exact source document digest, capture time, normalization version, URL, source classification and immutable lineage.
5. `eligibility_assessment`: applicant-specific verdict (eligible/ineligible/unknown/partner_required), evidence and reviewer. No auto-qualified historical leads.
6. `funding_project`: X3 product, hardware need, validator milestone or research experiment; linked to evidence/CI and measurable deliverables.
7. `application_package`: versioned draft bound to a source snapshot and applicant; human approval and source-specific requirements.
8. `outreach_and_submission`: explicit authorization, recipient, outbound message, submission receipt and status; unapproved actions prohibited.
9. `commitment_or_award`: legally/contractually evidenced offers, accepted awards and restrictions. Independent of application states.
10. `resource_contribution`: equipment, credits, compute-hours or cash with **separate valuations and units**, ownership/loan terms and acceptance checks.
11. `asset_and_compute_allocation`: equipment intake/inspection and GPU work receipts linked to funded projects, not mistaken for money received.
12. `financial_event`: requested, committed, awarded, invoiced, paid, recognized, credited and spent as distinct auditable statuses; no invented zeroes or cross-currency sums.

No new database or duplicate set of records is authorized by this document. The above will be mapped onto existing `funding_swarm_grants`, `funding_sources`, `funding_source_snapshots`, `funding_proposal_versions`, events/publications, then extended in **additive** shared gateway/indexer migrations only after the #630/#632/#633 stack is reconciled.

## Operational topology

```text
OFFICIAL SOURCE CONNECTORS         HISTORICAL DATA / INTERNAL OBSERVATIONS
Grants.gov | Web3 DAOs | foundations | grants/*.md | inventory | CI/benchmarks
hardware suppliers | credits       | sponsor content | GPU worker receipts
                  \                 /
               ingestion and evidence gate
                 (bounded, source-bound)
                           |
             X3 GATEWAY + POSTGRESQL
                one canonical lifecycle
                           |
      +--------------------+---------------------+
      |                    |                     |
 private Money Swarm  public funding/sponsor  research/compute
 operator workspace   views (approved only)  and hardware clients
      |                                          |
 approvals -> outreach/submission                 |
      |                                    Inferstructor workers
 audited evidence and commitments          + physical inventory
```

The static public `apps/x3-funding` content remains a presentation layer unless wired to approved backend records. It must not independently calculate “awarded” balances from historical/marketing data.

## Integration order and strict merge dependencies

**0. Audit/repair first:** keep #630 → #632 → #633 ordered (currently draft; exact CI/branch protection required). Do not merge #631, which #632 supersedes. Review contract, provenance and fake-award controls before anything else.

**1. Catalog recovery (this PR #636):** read-only, offline export from `grants/`. Only archival research. Preserve IDs, file/line and historical status; `current_availability=UNKNOWN`. Count discrepancy is not a claim of 155 fabricated leads: legacy editorial totals use different counting methods and must be reconciled.

**2. Safe catalog bridge (new stacked PR after #633):** private import preview + reviewed staging ingestion into the canonical source registry; no live/public “open” designation until authoritative current-source recheck. Assert idempotence and source scoping; never import legacy AWARDED/QUALIFIED directly.

**3. Official Web3 discovery:** adapter contracts and first live adapters for Polkadot OpenGov, Solana, Rootstock, Stellar, Arbitrum and public-goods sources. Reject unsupported claims; record closed and unknown programs accurately. Browser automation must obey site access rules; authenticated sources need approval.

**4. Hardware sourcing:** source-verified donation/sponsorship opportunities mapped to the **same funding ledger** but distinct equipment/loan/credit lifecycles. Existing hardware schema is a compatibility input only: validate migration execution, real persistence, device inventory and permission boundaries. Do not import hardcoded personal contacts or supposed success percentages.

**5. Hardware R&D lane:** X3A-P1 FPGA/CXL/DPU and memory/optical research treated as proposed hardware designs, eligible only after technical and funder review. Hardware grant must identify actual deliverables, compatibility, cost, shipping, power and cooling.

**6. Compute/project bridge:** retain opt-in worker identity, signed leases, receipts and no arbitrary shell execution. Attach only **verified** jobs and actual measurements to project evidence. No volunteer enrollment via grant crawler.

**7. One UI:** unify private operator workflows (discovery/review/draft/approval/hardware/research) in MoneySwarmWorkspace; reuse public `apps/x3-funding` routes as backend-backed public views with editorial review. No second independent funding CRUD service.

**8. Reporting:** public “found/hopeful/submitted/awarded/paid” stages with provenance gates. Monetary sums segmented by cash vs credits vs approximate in-kind value; donation and loan are never both booked as owned assets.

## First end-to-end demonstration

1. Revalidate one real Web3 source from the historical database and one official hardware donor/source.
2. Show both as separate kinds in the **same** private opportunity list with official snapshots and explicit UNKNOWN eligibility.
3. Review eligibility without bypassing state transitions.
4. Prepare a snapshot-bound, editable X3-specific grant package and equipment request draft.
5. Show **no awarded funding** without verified supporting records.
6. Generate an approved-only public view (no outbound send).
7. Exercise all transitions and backup/restore under a real PostgreSQL integration test, with exact-head CI.

## Non-negotiable safeguards

- No live web grant status from old Markdown, cached previews or historical URL probes.
- No spam/outreach, submission, purchase, commitments, tax receipts or financial movement without explicit authorization.
- No unverified human contacts, guaranteed funding, fabricated sponsor relationships, imaginary hardware and GPU “proof.”
- Preserve existing migrations and their checksums; coordinate shared gateway/indexer migration changes across branch stacks.
- Do not deploy public volunteer workloads until independent isolation review and opt-in/consent tests.
- All stages have owner, evidence, approval policy, reproducible tests and rollback strategy.

**Status:** architecture decision, plus separate read-only catalog exporter in #636. Unified PostgreSQL import, live Web3/hardware connectors, operator UI wiring and deployed GPU workflows remain **not implemented by this PR**.

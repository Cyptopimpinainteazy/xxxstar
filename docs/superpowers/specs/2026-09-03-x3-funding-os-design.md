# X3 Funding OS Design

Date: 2026-09-03
Status: Approved architecture, pending implementation plan
Canonical repository: `Cyptopimpinainteazy/xxxstar`
Supporting repositories: other X3 repositories may provide reference implementations or evidence, but only capabilities verified in `xxxstar` may be represented as production-complete unless a proposal explicitly identifies another repository.

## 1. Objective

Build an evidence-driven funding acquisition and grant-readiness operating system for X3. The system continuously discovers legitimate funding and sponsorship opportunities, verifies eligibility, scores expected value, audits `xxxstar` against opportunity requirements, prepares the repository through safe branches and pull requests, produces evidence-backed proposals, performs permitted outreach and application submission, tracks outcomes, and converts award obligations into engineering and reporting work.

The system is explicitly designed to prevent unsupported grant claims, duplicate outreach, policy violations, and direct unreviewed modification of protected production branches.

## 2. Repository Truth Model

`xxxstar` is the canonical production repository for funding claims.

A capability may be represented as currently implemented only when the Claim Ledger links it to verified evidence in `xxxstar`, such as source, tests, CI results, benchmark output, release artifacts, or reproducible runtime evidence.

Supporting X3 repositories may be searched for reusable implementation work, but findings from those repositories are classified as `EXISTS_IN_SUPPORTING_REPO` until ported, verified, and merged into `xxxstar`.

The repository already maintains an authoritative launch-scope model. The Funding OS must consume that source of truth rather than infer production status from isolated code. Current repository status identifies X3 as a v0.4 internal testnet candidate, with external bridges and several advanced features intentionally gated for later audited phases. The Funding OS must preserve those distinctions in every proposal and outreach artifact.

## 3. Core Operating Loop

```text
DISCOVER
  -> VERIFY PROGRAM
  -> EXTRACT REQUIREMENTS
  -> SCORE OPPORTUNITY
  -> MATCH TO X3 COMPONENT
  -> AUDIT XXXSTAR
  -> SEARCH SUPPORTING REPOS
  -> BUILD GAP MATRIX
  -> CALCULATE FUNDING LEVERAGE
  -> PREPARE REPO
  -> BUILD / TEST / SECURITY / BENCHMARK
  -> OPEN PR
  -> VERIFY MERGED STATE
  -> RE-SCORE READINESS
  -> BUILD EVIDENCE PACK
  -> FIND CONTACTS
  -> GENERATE TAILORED ASK + BUDGET
  -> FACT-CHECK EVERY CLAIM
  -> SUBMIT WHEN POLICY ALLOWS
  -> OUTREACH
  -> FOLLOW UP
  -> TRACK RESULT
  -> AWARD / REJECTION
  -> LEARN
```

Every stage is persisted as structured state. Agents exchange validated jobs and records rather than free-form conversational handoffs.

## 4. Specialized Agent Pipeline

The system uses specialized agents with narrow responsibilities:

1. Hunter — discovers grants, sponsorships, hardware programs, credits, bounties, accelerators, research opportunities, public-goods rounds, and partnerships.
2. Verifier — confirms official source, active status, deadline, eligibility, rules, and legitimacy.
3. Requirements Parser — converts program text into concrete technical, documentation, legal, security, milestone, and submission requirements.
4. Scorer — calculates opportunity fit, expected value, probability, urgency, effort, and evidence readiness.
5. Contact Intel — finds official and public professional contact routes with provenance and confidence.
6. X3 Matcher — maps each opportunity to the most relevant X3 module or funding product.
7. Repo Auditor — inspects `xxxstar` against program requirements.
8. Gap Mapper — classifies requirements as `VERIFIED`, `STRONG_PARTIAL`, `PARTIAL`, `WEAK_PARTIAL`, `MISSING`, `NOT_APPLICABLE`, `EXISTS_IN_SUPPORTING_REPO`, or `FUTURE_FUNDED`.
9. Grant Readiness Planner — distinguishes submission blockers from work legitimately proposed for grant funding.
10. Implementation Swarm — performs approved real repository improvements through isolated branches and PRs.
11. Test/Verification Agent — runs build, test, static analysis, security, reproducibility, and benchmark checks.
12. Documentation Agent — repairs or creates grant-relevant technical documentation.
13. Evidence Builder — records verifiable commits, tests, CI, benchmarks, screenshots, releases, and demos.
14. Proposal Writer — creates tailored applications from verified claims and program requirements.
15. Budget Agent — generates milestone-based use-of-funds budgets grounded in actual work.
16. Application Agent — completes permitted forms and uploads, with stop conditions for attestations and legal commitments.
17. Outreach Agent — sends approved automatic email and permitted social/community outreach.
18. Follow-Up Agent — tracks cadence and responses without duplicate or harassing contact.
19. Sponsor Agent — targets hardware, GPU, server, networking, storage, cloud, RPC, and security sponsorships.
20. Negotiation Agent — drafts responses and revised scopes; material commitments escalate.
21. Award Agent — tracks received cash, hardware, credits, services, restrictions, and obligations.
22. Funder Reporting Agent — turns verified delivery evidence into milestone and final reports.
23. Auditor — has veto authority over unsupported claims, duplicate outreach, expired programs, policy violations, suspicious contacts, or conflicting commitments.

## 5. Opportunity Classes

The system classifies opportunities as:

`GRANT | SPONSOR | HARDWARE | COMPUTE | SECURITY | RESEARCH | BOUNTY | HACKATHON | ACCELERATOR | PUBLIC_GOODS | PARTNERSHIP | DONATION | CREDIT`

Discovery must search both explicit funding language and capability needs, such as accelerator developer programs, AI compute credits, validator hardware sponsorship, FPGA programs, audit subsidies, startup credits, research collaborations, and infrastructure partnerships.

## 6. Scoring Model

Each opportunity receives a Funding Score from 0 to 100 using a configurable weighted model. Initial weighting:

- 20% X3 technical fit
- 15% eligibility confidence
- 15% probability of award
- 15% total economic value
- 10% strategic value
- 10% evidence readiness
- 5% deadline urgency
- 5% contact quality
- 5% application-effort efficiency

Separate value fields are maintained for cash, hardware, compute/credits, security/audit services, and strategic relationship value.

Repository Readiness is scored independently. Requirement completion values are initially:

- `VERIFIED = 1.00`
- `STRONG_PARTIAL = 0.75`
- `PARTIAL = 0.50`
- `WEAK_PARTIAL = 0.25`
- `MISSING = 0.00`
- `NOT_APPLICABLE` excluded
- `FUTURE_FUNDED` excluded from existing-capability scoring

Requirement severity multipliers:

- `BLOCKER = 5x`
- `CRITICAL = 3x`
- `IMPORTANT = 2x`
- `OPTIONAL = 1x`

A blocker can prevent submission regardless of aggregate percentage.

## 7. Funding-Weighted Engineering

Repository tasks receive a Funding Leverage Score based on technical importance, opportunities unlocked, aggregate opportunity value, urgency, probability improvement, sponsor value, and engineering effort.

The system consolidates overlapping grant requirements into shared engineering campaigns. A single reproducible GPU validator benchmark, for example, may unlock multiple grants, hardware sponsors, cloud-credit programs, and research opportunities. The system should complete that shared capability once rather than create fragmented grant-specific implementations.

## 8. Evidence Vault and Claim Ledger

The Evidence Vault stores verified:

- repository commits and files;
- test output and CI runs;
- benchmark artifacts;
- release metadata;
- architecture and deployment documents;
- screenshots and demo recordings;
- current hardware inventory and infrastructure evidence;
- reproducibility results;
- security and audit results;
- authorized organization and team facts.

The Claim Ledger maps proposal claims to evidence and a status. Claims that are unsupported, stale, contradicted by launch scope, or only present in a supporting repository cannot be represented as complete current capabilities.

The current `xxxstar` launch-scope distinctions are particularly important. External EVM/SVM/Bitcoin bridge paths, GPU acceleration, parallel execution, and other post-audit features must remain described as gated, audit-ready, experimental, or future work until authoritative scope and evidence change.

## 9. Repository Modification Policy

Grant-generated engineering work follows:

```text
requirement gap
  -> readiness work item
  -> isolated branch
  -> implementation
  -> tests/static analysis/security
  -> fresh-machine verification
  -> evidence generation
  -> pull request
  -> independent review/CI
  -> merge gate
  -> readiness re-score
```

The Funding OS must not directly modify protected `main`, force-push protected refs, bypass CI, disable tests, weaken security, fabricate evidence, or mark incomplete work as implemented.

Production paths must not introduce mocks, fake data, placeholder integrations, silent no-op implementations, or fabricated benchmarks. Performance-sensitive Rust work should record before/after benchmark evidence where practical. Changes must be rollback-safe and reproducible from a fresh machine.

## 10. Submission and Outreach Autonomy

Actions are classified as:

- GREEN — may execute automatically after policy and evidence checks.
- YELLOW — may be prepared automatically but requires approval.
- RED — never executed autonomously.

Typical GREEN actions include research, scoring, repo audit, proposal drafting, ordinary verified email outreach, and permitted applications.

YELLOW includes ambiguous eligibility, unusual contractual language, or major architectural repo changes.

RED includes signatures, banking changes, tax certifications, debt, equity or token commitments, exclusivity, IP transfer, legally binding attestations, irreversible account permissions, or factual representations that cannot be independently verified.

Automatic sending additionally requires contact legitimacy, duplicate checks, cadence compliance, program-rule compliance, and factual-claim audit.

## 11. Opportunity State Machine

```text
DISCOVERED
  -> VERIFIED
  -> SCORED
  -> REPO_AUDIT
  -> GAPS_IDENTIFIED
  -> PREPARING
  -> VERIFYING
  -> READY
  -> PROPOSAL_AUDIT
  -> SUBMITTABLE
  -> SUBMITTED
```

Alternative states include:

`DEPRIORITIZED | BLOCKED | EXPIRED | REJECTED | AWARDED | WITHDRAWN | HUMAN_REVIEW_REQUIRED`

Submission requires the configured funding threshold, no eligibility blocker, no factual-claim blocker, required readiness threshold, successful proposal audit, and a submission method permitted by the program.

## 12. Database and Core Records

PostgreSQL is the system of record. Primary entities:

- `opportunities`
- `organizations`
- `contacts`
- `program_requirements`
- `x3_components`
- `repo_evidence`
- `claims`
- `readiness_checks`
- `readiness_tasks`
- `engineering_campaigns`
- `proposals`
- `applications`
- `outreach`
- `followups`
- `awards`
- `deliverables`
- `budgets`
- `score_history`
- `audit_log`

Every important record carries provenance, timestamps, confidence, source identity, and the agent or service that created it.

## 13. Production Technology Stack

Recommended stack:

- Rust + Axum for API/control plane;
- Rust + Tokio for concurrent crawlers, repo scanners, and durable workers;
- Python for LLM orchestration, document generation, enrichment, and browser workflows;
- PostgreSQL as the authoritative relational store;
- pgvector initially for semantic retrieval without introducing a separate vector service;
- NATS JetStream as the preferred durable job/event layer, with Redis Streams as a simpler fallback;
- S3-compatible object storage for reports, screenshots, benchmark artifacts, and evidence packs;
- Playwright for browser/form automation;
- GitHub App/API integration for read, branch, commit, PR, CI, and review state;
- Next.js/TypeScript for the funding command-center dashboard;
- Prometheus, Grafana, and Loki for metrics, dashboards, and logs;
- Vault or equivalent encrypted secret management;
- a model gateway that supports both local models and multiple frontier-model providers.

Agents do not receive raw credentials. Sensitive actions are performed by policy-enforcing services after validating the agent request.

## 14. Service Boundaries

The initial services are:

- Discovery Gateway
- Opportunity Engine
- Funding API / Control Plane
- Job/Event Bus
- Repo Intelligence Service
- Evidence/Claim Service
- Readiness Planner
- Proposal Factory
- Policy/Audit Service
- Outreach Service
- Browser/Application Service
- Award/Reporting Service
- Dashboard

Agent-to-agent communication uses structured jobs with schemas, versioned inputs, deterministic identifiers, retries, and auditable outputs.

## 15. Model Strategy

Low-cost or local models handle classification, deduplication, summarization, first-pass requirement extraction, tagging, and repetitive transformations.

Stronger models are reserved for eligibility interpretation, technical grant matching, repository-gap reasoning, architecture analysis, final proposal review, and contradiction detection.

All model calls go through a common gateway with a contract equivalent to:

```text
generate(task, evidence, constraints, required_schema)
```

No proposal generator receives authority to invent facts outside the Evidence Vault and Claim Ledger.

## 16. Discovery and Contact Intelligence

Discovery prefers official program pages, foundation sites, public-goods platforms, government and research databases, startup programs, accelerator programs, hardware programs, hackathon/bounty platforms, and corporate partnership pages.

Contact records include person, role, organization, public professional email, official contact form, public professional social channels, source, verification time, confidence, do-not-contact state, and last-contacted time.

Private personal information is not scraped. Guessed addresses are not treated as verified contact data.

## 17. Proposal Factory

Proposal generation uses structured retrieval from program requirements plus verified X3 evidence. It produces fields such as:

- problem statement;
- solution;
- existing progress;
- technical approach;
- milestones;
- budget;
- impact;
- public-goods value;
- risk;
- sustainability.

A renderer adapts these structured fields to the funder's requested format and character limits. The Auditor then validates claims, evidence links, eligibility, budgets, and conflict rules before submission.

## 18. Failure and Retry Model

Where practical, jobs are idempotent.

Transient external failures use exponential backoff and alternate workers before entering a dead-letter queue.

Schema or model-output failures retry through constrained generation and may escalate to an alternate model or `HUMAN_REVIEW_REQUIRED`.

Repository changes that fail verification remain isolated on their branch, preserve logs, and may be rolled back without affecting protected production refs.

Browser/form failures preserve completed answers, screenshots, errors, and session context before escalation.

No failure is silently dropped.

## 19. Deadline-Aware Scheduling

Opportunity urgency changes scheduling but never truth standards.

- NORMAL: more than 30 days; optimize shared engineering and proposal quality.
- PRIORITY: 7-30 days; prioritize blockers and proposal preparation.
- URGENT: 72 hours-7 days; focus workers on submission-critical gaps.
- CRITICAL: less than 72 hours; freeze unrelated grant work if justified.

Urgency does not permit fabricated claims, bypassed verification, or prohibited submission automation.

## 20. Hardware and Infrastructure Sponsorship

The Funding OS maintains an Infrastructure Needs Registry for GPUs, FPGA boards, GPU servers, NVMe, enterprise SSDs, RAM, 25/40/100Gb networking, optics, UPS/batteries, rack equipment, bandwidth, colocation, RPC capacity, cloud compute, and security tools.

Each request links current capability, technical limitation, requested resource, capability unlocked, and measurable sponsor-facing deliverable. Sponsorship asks are therefore framed as engineering/research partnerships rather than generic requests for equipment.

## 21. Award Operations

Awards transition into execution mode:

```text
AWARDED
  -> restrictions recorded
  -> milestones created
  -> engineering tasks linked
  -> evidence collected continuously
  -> budget tracked
  -> milestone reports generated
  -> deliverables verified
  -> funder updated
  -> final report
  -> renewal / next opportunity
```

The same proof system used to win funding is used to prove delivery.

## 22. MVP Delivery Order

Phase 1 — Funding Brain

Build PostgreSQL, Hunter, Verifier, Requirements Parser, Scorer, and basic dashboard. Acceptance: real opportunities are discovered, deduplicated, verified, and ranked automatically.

Phase 2 — Repo Intelligence

Add GitHub integration, `xxxstar` scanner, Evidence Vault, Claim Ledger, and Grant Readiness scoring. Acceptance: a real grant produces an evidence-backed readiness report against `xxxstar`.

Phase 3 — Grant-Aware Engineering

Add Gap Mapper, Funding-Weighted Backlog, branch creation, implementation workers, verification, and PR generation. Acceptance: a grant requirement can produce a verified PR without modifying protected `main`.

Phase 4 — Proposal Factory

Add X3 Matcher, Proposal Writer, Budget Agent, Claim Auditor, and evidence attachments. Acceptance: every existing-capability claim in a tailored proposal is traceable to evidence.

Phase 5 — Autonomous Acquisition

Add Contact Intel, email, permitted browser/form automation, and follow-up engine. Acceptance: find -> qualify -> prepare -> submit -> follow up can run automatically when all GREEN gates pass.

Phase 6 — Funding Operations

Add award tracking, hardware registry, deliverables, expenditure tracking, funder reports, and conversion analytics. Acceptance: the system manages the lifecycle from discovering resources to proving delivery.

## 23. First End-to-End Acceptance Test

Use one real current opportunity. The system must independently discover and verify it, extract requirements, score it, audit `xxxstar`, identify missing requirements, search supporting repositories, produce a funding-weighted preparation plan, perform permitted changes through branch/PR, verify the resulting repository, build an evidence pack, find the correct official/professional contact route, generate a tailored proposal and budget, audit every factual claim, submit automatically only if all GREEN gates pass, record the application, and schedule appropriate follow-up.

Pass condition: a reviewer can inspect the complete evidence and audit trail afterward and explain why each action occurred.

## 24. Non-Goals for Initial Delivery

The first implementation does not require unrestricted autonomous merging, unrestricted browser automation, autonomous legal commitments, autonomous access to banking or tax systems, a custom vector database, Kubernetes, or a full multi-agent swarm running every role simultaneously.

The architecture is intentionally staged so the evidence model, scoring model, and repository-readiness loop become trustworthy before autonomous outbound submission is enabled.

## 25. Design Invariants

1. `xxxstar` is the canonical source for production-complete X3 capability claims.
2. Authoritative launch-scope documents override inferred status from isolated code.
3. No proposal may claim a capability without evidence.
4. Future funded work must remain distinguishable from existing capability.
5. No grant-driven engineering bypasses protected-branch verification.
6. No production path may be completed with mocks, fake data, silent no-ops, or fabricated benchmarks.
7. Every autonomous external action must be attributable, policy-checked, deduplicated, and auditable.
8. No legal, financial, equity, token, IP, tax, banking, or exclusivity commitment is made autonomously.
9. Shared engineering improvements should unlock multiple funding opportunities wherever possible.
10. Rejections and awards both feed measurable learning back into scoring and targeting.

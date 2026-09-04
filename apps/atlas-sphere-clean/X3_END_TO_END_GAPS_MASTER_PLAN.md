# X3 End-to-End Gaps Master Plan

This document extends the current readiness documents with the missing system work surfaced during launch-hardening discussions. It is not a replacement for [implementation_plan.md](implementation_plan.md), [X3_GAPS_REPORT.md](X3_GAPS_REPORT.md), or [docs/planning-artifacts/PRD.md](docs/planning-artifacts/PRD.md). It is the consolidation layer for the remaining architecture, integration, governance, swarm, security, operations, and growth gaps that still need explicit build ownership before testnet hardening can be called complete.

## What this document is for

Use this file as the cross-functional execution map for the work that sits between "feature exists" and "system can be trusted in hostile conditions." The items below focus on interfaces, runtime law, proofs, command boundaries, observability, emergency response, and chain-native swarm controls. If a topic already exists elsewhere, this file defines the missing integration work and the order it should be built.

## How this relates to current planning artifacts

- [implementation_plan.md](implementation_plan.md) covers production go-live mechanics, deployment steps, and integration tests.
- [X3_GAPS_REPORT.md](X3_GAPS_REPORT.md) captures broad repo gaps and audit findings.
- [docs/planning-artifacts/PRD.md](docs/planning-artifacts/PRD.md) covers the earlier critical-path MVP.
- [docs/x3-swarm-orchestra/README.md](docs/x3-swarm-orchestra/README.md) describes the current swarm/orchestra direction.

This file adds the missing end-to-end program for:

1. chain-native swarm law,
2. proof-carrying operations,
3. agent lifecycle controls,
4. attack-time emergency behavior,
5. content and outreach systems with explicit policy boundaries,
6. operator dashboards and live control surfaces,
7. staged rollout criteria from internal use to public growth.

## Priority bands

### Band 0 — cannot ship testnet trustfully without these

- Atomic cross-VM invariants must be written as enforceable runtime and off-chain verification rules, not only implied by code paths.
- Swarm roles must have capability envelopes, budgets, kill switches, and revocation semantics.
- Emergency powers must be explicit: who can stop what, under which evidence threshold, with what expiration rules and audit trail.
- Determinism tiers must be defined for every swarm task: deterministic, bounded-deterministic, review-required, or non-consensus-only.
- Proof and receipt formats must be standardized so execution, challenge, slashing, and forensics all refer to the same evidence model.
- The system needs a single operator view that joins chain health, cross-VM state, swarm jobs, alerts, proofs, and emergency actions.

### Band 1 — required before open participation or real economic exposure

- Reputation, bonding, slashing, and dispute flow for swarm nodes and agents.
- Governance delays, constitutional limits, challenger rights, and post-incident review mechanics.
- Policy compiler for outbound content, outreach, autonomous messaging, and external actions.
- Formal invariant registry with runtime checks, simulation checks, and evidence links.
- Multi-stage environment promotion gates from local devnet to internal testnet to public testnet.

### Band 2 — required for scale, growth, and durable operator leverage

- Content pipeline orchestration with multilingual support and asset provenance.
- Partner and contributor recruitment funnels with attribution, review, and reputation feedback.
- Research swarm, capital scouting, ecosystem intelligence, and founder-media support systems.
- Local and cloud execution routing across CPU, GPU, trusted operators, and sandboxed tools.

## Section 1: Missing chain/runtime law

This section covers the protocol rules that still need to be written down and enforced so that the chain, cross-VM kernel, and swarm all operate under the same safety model.

### 1.1 Canonical invariants registry

Create a machine-readable invariant registry that assigns stable IDs to every rule the system must preserve. The first set should include atomic settlement integrity, no double finalization, bond conservation, replay rejection, cross-VM state agreement, privileged action expiry, proof freshness, agent budget ceilings, and emergency authority limits. Each invariant needs four linked artifacts: a human-readable description, runtime or service enforcement location, test/simulation coverage, and incident response guidance.

### 1.2 Proof-carrying state transitions

Define which transitions require attached proofs or receipts before they are accepted, challenged, or finalized. Cross-VM prepare, commit, abort, slashing, emergency pause, agent capability elevation, and governance-triggered code-path changes all need explicit evidence formats. The evidence model must specify hash inputs, signer set, inclusion rules, replay domain, expiry window, and storage location.

### 1.3 Runtime emergency powers

Add a formal emergency power map for runtime modules and operator services. The system should distinguish between pause, degrade, quarantine, and kill actions. Every action needs scope limits, who may trigger it, what evidence is required, how long it remains active, what on-chain event is emitted, and how the system returns to normal mode.

### 1.4 Determinism tiering

Not all workloads should be treated equally. Build a determinism classification matrix covering validator-adjacent jobs, oracle-like jobs, media generation, research tasks, routing tasks, and outward communications. Anything that can influence consensus, slashing, governance, or capital movement must be deterministic or bounded-deterministic with challengeable receipts. Non-deterministic workloads must stay off the consensus path and remain attributable to operators or approved agents.

## Section 2: Missing swarm architecture and control-plane gaps

This section covers the gap between having GPU nodes or agents and having a governed, auditable swarm that can be trusted with chain-adjacent responsibilities.

### 2.1 Three-plane architecture enforcement

The system needs explicit separation between the user plane, the swarm control plane, and the blockchain/runtime plane. User-facing requests should resolve into approved intents. The swarm control plane should plan, simulate, score, and route work. The blockchain/runtime plane should only accept outputs that satisfy protocol rules. Each plane needs typed interfaces, rate limits, authentication, and audit logs.

### 2.2 Role-typed swarm node classes

Define formal classes for validator-adjacent GPU workers, challenger/watcher nodes, research nodes, indexing nodes, content nodes, security nodes, and campaign operators. Each class needs hardware requirements, allowed tools, data access scope, receipt obligations, stake requirements, reward rules, and incident handling behavior.

### 2.3 Swarm scheduler and job policy engine

The scheduler should not only place jobs; it must enforce job legality. Build a policy layer that evaluates task class, determinism tier, data sensitivity, cost ceiling, proof requirements, approval requirements, escalation rules, and fallback behavior. Scheduler outputs should be explainable and reproducible from logs.

### 2.4 Mutation and self-improvement pipeline

If agents or swarm strategies are allowed to evolve, that evolution must be bounded. Implement a mutation proposal flow where new prompts, policies, strategies, routing heuristics, or model selections are versioned, simulated, reviewed, and staged before broader rollout. No self-modifying behavior should bypass this pipeline.

### 2.5 Capability envelopes

Every agent and node class needs a capability envelope that states what it can read, write, invoke, spend, publish, and pause. Envelopes should include token budgets, API allowlists, file-system boundaries, network restrictions, per-task timeouts, and mandatory reviewer classes for sensitive actions.

## Section 3: Missing agent law

This section turns the swarm from a set of tools into a constitutional subsystem.

### 3.1 Agent genesis records

Every durable agent needs a genesis record: creator, purpose, class, model/tool stack, allowed surfaces, funding source, supervision mode, revocation path, and version lineage. Genesis records should be immutable once created except through a governed amendment path.

### 3.2 Commandment compiler

Build the policy compiler that transforms high-level rules into executable gates. Rules should cover honesty, identity disclosure, anti-impersonation, budget ceilings, no unauthorized private outreach, no unsanctioned posting, no self-escalation, no capital movement without explicit policy, and mandatory evidence logging. The compiler output should be machine-enforced policy bundles, not prose-only guidance.

### 3.3 Strike, quarantine, and termination system

Agents need a consistent misconduct ladder. Define what constitutes a warning, strike, quarantine, bond slash, forced downgrade, suspension, and irreversible kill. Tie each action to evidence standards, operator overrides, appeal windows, and postmortem requirements.

### 3.4 Delegation and spawning rules

Agents should not be able to create unbounded descendants. Spawning needs class-specific limits, inherited envelopes, budget partitioning, naming lineage, and default expiration. If an agent wants a broader permission set than its parent, that request must route through governance or an authorized operator workflow.

## Section 4: Missing economic and reputation gaps

This section covers the incentive structure that keeps swarm participants aligned when real value is at stake.

### 4.1 Bonding model for node classes

Different swarm roles need different collateral rules. Validator-adjacent, challenge, security, and capital-sensitive roles should post materially different bonds than content or research roles. The bond model should define stake size, slash buckets, cooldowns, reinstatement rules, and whether reputation can offset capital requirements.

### 4.2 Outcome-linked reputation

Reputation should derive from measured outcomes, not raw activity. Build a scoring model that weights correctness, timeliness, challenge success, false-positive rate, review burden, rollback incidence, and incident involvement. Scores need decay, domain-specific tracks, and protection against simple farming strategies.

### 4.3 Reward symmetry

Right now many systems reward action volume more easily than restraint. Add reward symmetry so agents or operators earn for correct intervention, correct non-intervention, challenge success, cost savings, and prevented damage. Avoid systems that reward meaningless output or spammy growth activity.

### 4.4 Referral and recruitment integrity

If the swarm participates in operator or contributor recruitment, attribution needs anti-sybil and anti-pyramid controls. Referrals should pay out on verified value creation, not simple invites. Reputation feedback should reflect downstream behavior of referred participants.

## Section 5: Missing security operations and adversarial readiness

This section covers live defense, not only preventive coding hygiene.

### 5.1 Attack playbooks

Write operator playbooks for bridge desync, replay waves, fraudulent proofs, cartelized challengers, sequencer griefing, governance capture attempts, runaway agents, compromised content systems, credential theft, and malicious operator behavior. Each playbook should define detection signals, immediate containment actions, communication steps, recovery sequencing, and evidence retention requirements.

### 5.2 Security swarm roles

Create explicit security-specific swarm roles for anomaly detection, challenge generation, forensic indexing, exploit rehearsal, and postmortem synthesis. These roles should have tighter data controls than growth or content agents and should never share credentials or workspaces with outward-facing automation.

### 5.3 Chaos and red-team program

Build recurring simulations for multi-node faults, network partitions, false challenge floods, hostile governance proposals, outbound communication compromise, and abusive tool use. The output of every exercise should feed back into the invariant registry, capability envelopes, and emergency powers table.

### 5.4 Evidence-preserving incident system

Incidents should automatically snapshot the relevant chain state, logs, receipts, configs, model/prompt versions, and operator actions. Postmortems need deterministic reconstruction where possible and explicit uncertainty markers where not.

## Section 6: Missing governance and constitutional gaps

This section covers the governance work required if swarm behavior, emergency powers, or chain policy are going to change over time.

### 6.1 Constitutional rulebook

Create a governing rulebook for what governance may and may not do. Governance should not be able to silently disable audit logs, remove challenge rights, grant unlimited agent permissions, or bypass emergency expiry rules. Constitutional protections need higher thresholds and longer delays than ordinary parameter tuning.

### 6.2 Proof-carrying governance

Sensitive proposals should require attached evidence packages: simulation results, invariant impact report, rollout plan, rollback plan, and affected subsystem map. Proposal metadata should be rich enough for automated review before voting starts.

### 6.3 Challenger rights and minority defense

Document how challengers surface issues, pause unsafe changes, earn protection from retaliation, and escalate evidence when major stakeholders are conflicted. A chain without credible challenger rights will converge toward opaque operator power.

### 6.4 Recursive upgrade boundaries

If governance can change the rules that govern governance, those transitions need a separate amendment path. Define which parameters are mutable by ordinary governance, which require constitutional amendment, and which are immutable absent a migration event.

## Section 7: Missing operator interface and observability gaps

This section covers the control surfaces that let humans understand and steer the system under load.

### 7.1 Unified swarm cockpit

Build a single live dashboard showing chain liveness, cross-VM flow state, proof backlog, scheduler queues, node health, active agents, emergency toggles, incident banners, and growth/content pipelines. Operators should not need to join six tools mentally during an incident.

### 7.2 Intent-to-action tracing

Every external action should be traceable back through intent, planner, policy gates, reviewer, execution node, receipt, and outcome. This is required for both security and growth systems. The trace should be queryable by entity, campaign, agent, task class, and time range.

### 7.3 Review queues and approval UX

The control plane needs structured queues for human approval of sensitive actions: governance-affecting work, external publishing, direct outreach, capital-sensitive research, policy changes, and capability escalations. Approvals should include diff-style context, predicted blast radius, and linked evidence.

### 7.4 Quality-of-service and cost telemetry

Add visibility into GPU utilization, queue latency, tool failure rate, proof generation cost, review burden, retry loops, and output rejection rate. This is necessary to decide what stays on local infrastructure, what moves to cloud, and what should not be automated at all.

## Section 8: Missing content, media, and outward-action policy gaps

This section covers the large set of media and growth ideas that should exist only behind explicit policy and staging.

### 8.1 Outbound policy boundary

Define what the system may publish automatically, what requires human review, and what is never allowed. The rule set should prohibit fake humans, undisclosed impersonation, fabricated achievements, unsanctioned direct messaging, and autonomous relationship manipulation. Allowable outbound activity should be tied to disclosure, approval status, campaign owner, and platform policy.

### 8.2 Content supply chain

Build an asset pipeline for research notes, drafts, captions, transcripts, images, clips, translations, and approved final assets. Every asset should carry provenance: source material, model/tool path, editor history, usage rights, and approval state.

### 8.3 Founder-led media engine

The highest-trust outward channel is still founder-mediated communication. Build systems that help generate drafts, repurpose transcripts, prepare multilingual variants, suggest posting schedules, and extract clips, while keeping final ownership and voice attribution explicit.

### 8.4 Content swarm classes

If content generation is part of the roadmap, split roles into research, script, editing, localization, design, analytics, and publishing support. Do not let one generic agent run the entire pipe. Each stage should hand off structured artifacts with reviewable diffs.

### 8.5 Campaign memory and learning

Campaign systems need persistent memory about what was attempted, approved, published, rejected, and how it performed. The learning loop should optimize for signal, trust, conversion quality, and operator time, not raw post count.

## Section 9: Missing outreach, recruiting, and ecosystem expansion gaps

This section covers external relationship systems that were discussed but not yet bounded operationally.

### 9.1 Partner and contributor pipeline

Design a workflow for identifying partners, builders, validators, creators, and contributors; scoring fit; generating outreach packets; tracking contact history; and routing warm opportunities to humans. The system should augment operator judgment rather than silently impersonate it.

### 9.2 Ecosystem intelligence layer

Research nodes should track protocols, competitors, integrations, grants, security incidents, liquidity venues, infrastructure providers, and market opportunities. Intelligence needs freshness windows, confidence scoring, deduplication, and source traceability.

### 9.3 Capital and opportunity scouting

If scouting investors, treasury partners, or market makers becomes part of the roadmap, it needs a separate compliance-aware workflow with manual approval gates and clear do-not-contact rules. This should never be bundled with broad autonomous outreach.

### 9.4 Community growth with disclosure

Community operations can use assistance for response drafting, FAQ generation, moderation suggestions, and localization, but the system should disclose automation where appropriate and preserve clear human ownership for sensitive conversations.

## Section 10: Missing execution substrate gaps

This section covers the local/cloud/tooling substrate required to run the swarm safely.

### 10.1 Sandbox and isolation model

Split execution into strong isolation tiers: local trusted tools, sandboxed untrusted tools, GPU worker sandboxes, browser automation sandboxes, and internet-facing connectors. Credentials, secrets, and wallets should never be available to all tiers equally.

### 10.2 Tool adapter layer

Create a typed adapter layer around external tools and APIs so policy checks, logging, retries, output normalization, and kill switches happen uniformly. Raw direct tool calls from agents should be treated as technical debt.

### 10.3 Secret handling and delegation

Build per-role secret scopes, short-lived credentials, approval-bound secret release, and complete access logs. No content or research agent should inherit secrets needed for validator, treasury, or governance operations.

### 10.4 Local-versus-cloud routing

Define routing rules for which workloads run on local GPUs, internal servers, rented GPU pools, or third-party AI APIs. The routing engine should consider sensitivity, cost, determinism requirements, latency, and evidence obligations.

## Section 11: Missing delivery and rollout gaps

This section covers how the system should be introduced safely instead of flipped on all at once.

### 11.1 Four-stage rollout

Stage 1 is internal operator-only swarm use with no public automation. Stage 2 is supervised external assistance with mandatory human approval. Stage 3 is limited public automation on narrow surfaces with hard metrics and kill switches. Stage 4 is broader open participation after the policy compiler, reputation system, and challenger flows are proven.

### 11.2 Exit criteria per stage

Each stage needs hard gates: incident rate ceilings, review latency, proof verification success, rollback success, false-positive and false-negative rates, content rejection rate, outreach policy compliance, and cost-per-approved-action. Promotion without metrics will hide failures.

### 11.3 Testnet hardening drills

Before public testnet messaging ramps up, run launch rehearsals that combine chain load, swarm job load, dashboard operations, emergency pauses, content pipeline review, and rollback tests. The test should simulate realistic operator stress rather than isolated happy-path jobs.

## Section 12: Concrete build order

This section converts the above gaps into a practical sequence.

### Phase A — law first

1. Build the invariant registry.
2. Define proof/receipt schemas.
3. Write the emergency powers table.
4. Define determinism classes.
5. Draft capability envelopes for all swarm roles.

### Phase B — control plane first pass

1. Implement the three-plane interface boundaries.
2. Build the job policy engine around the scheduler.
3. Add agent genesis records and lineage.
4. Implement strike/quarantine/kill mechanics.
5. Wire intent-to-action trace storage.

### Phase C — operator safety surfaces

1. Ship the unified swarm cockpit.
2. Add approval queues with diff-style review context.
3. Add cost/latency/rejection telemetry.
4. Add incident snapshot and evidence export.

### Phase D — economics and governance

1. Launch bonding and role-specific slashing.
2. Add outcome-linked reputation.
3. Add proof-carrying governance metadata.
4. Add challenger rights and constitutional limits.

### Phase E — outward systems under constraint

1. Build the content asset pipeline.
2. Build founder-led media support tools.
3. Add partner/contributor pipeline tooling.
4. Add campaign memory and performance feedback.
5. Keep all publishing and direct outreach behind policy gates and approvals until measured safe.

## Section 13: Definition of done for this document

The work tracked here is not done when a prototype exists. It is done when each subsystem has an owner, interface, policy boundary, receipt format, test coverage, dashboard visibility, and incident playbook. If any proposed swarm behavior still depends on "trusted operator intuition" rather than explicit system rules, it belongs back on this list.

## Immediate next implementation tickets

All document artifacts below are **complete** as of 2026-05-08. The next step is wiring the open code gaps identified within each document.

1. ✅ [docs/swarm-governance/INVARIANT_REGISTRY.md](../../docs/swarm-governance/INVARIANT_REGISTRY.md) — stable invariant IDs and enforcement mapping. **9 Band 0 coverage gaps remain in code.**
2. ✅ [docs/swarm-governance/CAPABILITY_ENVELOPES.md](../../docs/swarm-governance/CAPABILITY_ENVELOPES.md) — capability envelopes for all node and agent classes. **Quorum gate, Sentinel-Judge/Scribe enforcement, and stake requirements are planned in code.**
3. ✅ [docs/swarm-governance/EMERGENCY_POWERS.md](../../docs/swarm-governance/EMERGENCY_POWERS.md) — pause, degrade, quarantine, and kill semantics. **Degrade state machine, agent kill path, expiry enforcement, and audit trail are planned in code.**
4. ✅ [docs/swarm-governance/AGENT_LAW.md](../../docs/swarm-governance/AGENT_LAW.md) — genesis records, spawning rules, misconduct ladder, and termination. **Genesis persistence, misconduct ladder state machine, and kill path are planned in code.**
5. ✅ [docs/swarm-ops/OPERATOR_COCKPIT_SPEC.md](../../docs/swarm-ops/OPERATOR_COCKPIT_SPEC.md) — unified live dashboard specification. **All panels are specified; unified telemetry aggregation and dashboard frontend are planned.**
6. ✅ [docs/swarm-ops/OUTBOUND_POLICY.md](../../docs/swarm-ops/OUTBOUND_POLICY.md) — publishing, outreach, and disclosure rules with tier definitions. **Publishing gate service, content provenance storage, and do-not-contact register are planned.**
7. ✅ [docs/swarm-ops/ROLLOUT_STAGES.md](../../docs/swarm-ops/ROLLOUT_STAGES.md) — four-stage rollout with measurable exit criteria. **Stage 1 exit is blocked by 8 named code gaps listed in the document.**
8. **Next step:** each open-gap item in the documents above maps to a code implementation task. The Stage 1 exit blockers in `ROLLOUT_STAGES.md` are the prioritised list.

## Security swarm build-pack status

The first concrete scaffold for the security-specific portion of this plan now exists in [x3-security-swarm/README.md](x3-security-swarm/README.md). It includes spawnable templates, prompts, governance artifacts, chaos scenarios, evidence retention, a public threat registry schema, and a canonical incident postmortem. The next build step is wiring these artifacts into the existing orchestrator, quarantine manager, and governance override paths in the GPU swarm crates.

## Go-mode execution order

The current recommended implementation sequence across the multichain adapter, proving pipeline, security swarm, treasury backbone, omnichain token layer, auctions, launchpads, dApp hub, and user surfaces is tracked in [GO_MODE_EXECUTION_ORDER.md](GO_MODE_EXECUTION_ORDER.md). Use that file as the practical order-of-operations document when choosing what to ship next.

The operator-grade specification for the liquidity, inventory, and solvency layer described in `Phase 4.5` now lives in [docs/specs/X3_LIQUIDITY_INVENTORY_SOLVENCY_SPEC.md](docs/specs/X3_LIQUIDITY_INVENTORY_SOLVENCY_SPEC.md). Use it when implementing route reservation, vault policy, rebalance logic, partner capacity, solvency gates, and lane freeze behavior.
Allo
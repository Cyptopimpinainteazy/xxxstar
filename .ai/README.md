# X3 Blockchain Agent Operating System

This directory contains the complete agent operating system for building blockchain features on X3 with guaranteed correctness, safety, and atomic Cross-VM behavior.

## Quick Start

1. **Read this first:** [.ai/AGENTS.md](.ai/AGENTS.md) — Master system contract
2. **Pick your domain:**
   - Compiler/Parser? → `.ai/agents/x3ir-compiler-agent.md`
   - Runtime/Pallet? → `.ai/agents/runtime-integrator.md`
   - Bridge/Settlement? → `.ai/agents/bridge-settlement-auditor.md`
   - Cross-VM? → `.ai/agents/cross-vm-architect.md`
   - Security audit? → `.ai/agents/security-redteam.md`
   - Release? → `.ai/agents/release-closer.md`
3. **Start with a prompt:**
   - Any blockchain task? → `.ai/prompts/universal-blockchain-workflow.md`
   - Cross-VM feature? → `.ai/prompts/cross-vm-feature-build.md`
4. **Follow the hooks:**
   - Pre-task → Post-edit → Pre-commit → Pre-merge → Release-gate

## Directory Structure

```
.ai/
├── AGENTS.md                              Master system contract & routing
├── agents/
│   ├── supervisor.md                      Task router & final approval
│   ├── cross-vm-architect.md              Cross-VM execution & atomicity
│   ├── x3ir-compiler-agent.md             Parser → AST → X3IR → Emitter
│   ├── runtime-integrator.md              Runtime, pallets, dispatch, state
│   ├── bridge-settlement-auditor.md       Bridge, HTLC, proof, timeout
│   ├── invariant-test-engineer.md         Supply, replay, atomic invariants
│   ├── security-redteam.md                Attack scenarios & defenses
│   └── release-closer.md                  Release gating & mainnet safety
├── skills/
│   ├── cross-vm-trace.md                  Document Cross-VM state machine
│   ├── atomic-settlement-invariants.md    Define & test supply invariants
│   ├── replay-timeout-safety.md           Nonce & timeout protection
│   ├── canonical-path-reachability.md     Prove code is reachable
│   ├── score-cap-matrix.md                Evidence-based scoring
│   └── mainnet-safety.md                  Release readiness classification
├── hooks/
│   ├── hooks.yaml                         Hook configuration & triggers
│   ├── pre-task.md                        Setup scope & acceptance
│   ├── post-edit.md                       Validate & classify work
│   ├── pre-commit.md                      Final checks before commit
│   ├── pre-merge.md                       Merge safety & handoff
│   └── release-gate.md                    Final release approval
└── prompts/
    ├── universal-blockchain-workflow.md   Generic blockchain task template
    └── cross-vm-feature-build.md          Cross-VM feature template
```

## The 27 Required Gates

Every task must pass these gates before final output:

### Continuity Gates (9)
1. **Context Drift Gate** — Did we solve the original task, or drift?
2. **Handoff Pack Gate** — Can next agent continue without rereading?
3. **Stop Condition Gate** — When is work allowed to stop?
4. **State Transition Trace Gate** — Before/action/after/failure documented?
5. **Golden Path + Ugly Path Gate** — Happy path AND failure path tested?
6. **Config Reality Gate** — No hardcoded secrets or local paths?
7. **Parallel Agent Merge Gate** — Safe merge order identified?
8. **Spec Drift Gate** — Code matches docs/spec/RC plan?
9. **Determinism Gate** — Tests are flaky? Compiler output random?

### Autonomous Crew Gates (9)
1. **Work Chunking Gate** — Split into independently testable chunks?
2. **Duplicate Work Detection Gate** — Reuse > rebuild?
3. **Canonical Path Gate** — One true execution path identified?
4. **Dead Code + Reachability Gate** — Code is called from a real entrypoint?
5. **Artifact Proof Gate** — Logs/reports saved as proof?
6. **Auto-Ticket Generation Gate** — Blockers converted to tickets?
7. **Agent Memory Update Gate** — Facts & blockers recorded for next agent?
8. **Build Reconciliation Gate** — Parallel work reconciled?
9. **Red-Team Diff Gate** — Diff attacked & most dangerous change identified?

### Governance Gates (10)
1. **Priority Stack Gate** — P0/P1 fixed before P3/P4 polish?
2. **Timebox + Escalation Gate** — Build loops limited; escalate on repeat?
3. **Cost + Complexity Gate** — Architecture justified?
4. **Minimal Viable Proof Gate** — Core path proven before expansion?
5. **Data Contract Gate** — Schemas, events, payloads defined?
6. **Versioning Gate** — Breaking changes versioned?
7. **No Magic Constants Gate** — Constants named & explained?
8. **Error Taxonomy Gate** — Explicit errors instead of panics?
9. **Feature Flag Gate** — Risky features behind safe-default flags?
10. **Mainnet Safety Gate** — Mainnet claims backed by audit/stress/invariant?

## Core Laws (Non-Negotiable)

1. **No fake 100s.** All scores are evidence-based, not aspirational.
2. **No unreachable code.** If nothing calls it, it doesn't count.
3. **No bridge logic without replay/timeout/failure tests.** Non-negotiable.
4. **No runtime logic without invariant tests.** Supply is sacred.
5. **No Cross-VM feature without state transition trace.** Before/action/after/failure.
6. **No compiler feature without parser → AST → X3IR → emitter validation.** Full pipeline.
7. **No mainnet claim without audit/stress/invariant evidence.** Testnet ≠ mainnet.
8. **No stubs/mocks/fake proofs in core paths.** Production code is real.
9. **No interface change without compatibility note.** Breaking changes explicit.
10. **No final answer while FIXABLE_NOW work remains.** All fixable items resolved.

## The Money Rule

> **Cross-VM is not a feature. Cross-VM is a failure machine unless every state transition, replay path, timeout path, rollback path, and supply invariant is proven.**

## How to Use This System

### For a New Blockchain Task

1. Open `.ai/prompts/universal-blockchain-workflow.md`
2. Fill in your task
3. Follow pre-implementation steps (route, scope, criteria, blast radius, canonical path)
4. During implementation, apply the hooks in order
5. Before final output, run all 27 gates
6. Produce final output with all required sections

### For Cross-VM Features

1. Open `.ai/prompts/cross-vm-feature-build.md`
2. Define your feature
3. Document canonical path
4. Choose atomicity model
5. Plan state transitions (before/action/after/failure/timeout/replay)
6. Write tests for all cases
7. Run all validation
8. Include Cross-VM Trace in final output

### For Specialized Domains

| Domain | Agent | Skills | Hooks |
|--------|-------|--------|-------|
| Compiler/Parser | x3ir-compiler-agent | canonical-path-reachability | on_compiler_change |
| Runtime/Storage | runtime-integrator | atomic-settlement-invariants | on_runtime_change |
| Bridge/Settlement | bridge-settlement-auditor | replay-timeout-safety, atomic-settlement-invariants | on_bridge_change |
| Cross-VM | cross-vm-architect | cross-vm-trace, atomic-settlement-invariants, replay-timeout-safety | on_cross_vm_change |
| Invariants | invariant-test-engineer | atomic-settlement-invariants | (triggered by other agents) |
| Security | security-redteam | (all security-relevant skills) | (triggered after any change) |
| Release | release-closer | mainnet-safety, score-cap-matrix | release_gate |

## Score Cap Matrix (Quick Reference)

Your final score is `min(estimated_score, strictest_applicable_cap)`.

**You cannot escape caps with more code.**

| Situation | Cap |
|-----------|-----|
| No invariant test | 45-50% |
| No replay test | 55% |
| No timeout test | 55% |
| No end-to-end test | 70% |
| No Cross-VM trace | 60% |
| Fake proof in core | 35% |
| Unreachable code | 55% |
| Mainnet claim without proof | 50% |

## Release Readiness Matrix

To claim MAINNET READY, you need:

| Requirement | LOCAL | DEVNET | TESTNET | AUDIT | CANDIDATE | MAINNET |
|-------------|-------|--------|---------|-------|-----------|---------|
| Code compiles | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Unit tests pass | - | ✓ | ✓ | ✓ | ✓ | ✓ |
| Integration tests | - | - | ✓ | ✓ | ✓ | ✓ |
| Invariant tests | - | - | ✓ | ✓ | ✓ | ✓ |
| Security tests | - | - | ✓ | ✓ | ✓ | ✓ |
| Audit | - | - | - | ✓ | ✓ | ✓ |
| Stress test | - | - | - | - | ✓ | ✓ |
| Migration tested | - | - | ✓ | ✓ | ✓ | ✓ |
| Rollback plan | - | - | ✓ | ✓ | ✓ | ✓ |
| Monitoring | - | - | - | - | ✓ | ✓ |
| Governance approval | - | - | - | - | - | ✓ |

## Files to Read in Order

**First Time Setup:**
1. `.ai/AGENTS.md` (master contract)
2. Your domain's agent file (e.g., `agents/bridge-settlement-auditor.md`)
3. Your task's prompt template (e.g., `prompts/cross-vm-feature-build.md`)

**Before Implementation:**
4. `hooks/pre-task.md` (setup)
5. Relevant skills (e.g., `skills/cross-vm-trace.md`)

**During Implementation:**
6. Your domain's agent file again (reference)
7. Relevant skills (reference)

**After Implementation:**
8. `hooks/post-edit.md` (validation)
9. `hooks/pre-commit.md` (scoring)
10. `hooks/pre-merge.md` (merge safety)
11. `hooks/release-gate.md` (final approval)

## Critical Files to Bookmark

- `.ai/AGENTS.md` — Master contract (read first)
- `.ai/skills/score-cap-matrix.md` — How scores work (read early)
- `.ai/prompts/universal-blockchain-workflow.md` — Generic task template
- `.ai/prompts/cross-vm-feature-build.md` — Cross-VM template
- `.ai/hooks/hooks.yaml` — Hook configuration

## The Agent Operating System in 30 Seconds

1. **Route** your task to the right specialist agents (supervisor decides)
2. **Define** scope, acceptance, and canonical path (pre-task hook)
3. **Implement** the smallest complete solution (core responsibility)
4. **Validate** with all required tests and gates (27 gates required)
5. **Classify** missing work honestly (FIXABLE_NOW / BLOCKED / DEFERRED)
6. **Fix** all FIXABLE_NOW items (before final output)
7. **Score** using evidence-backed caps (min of estimate and cap)
8. **Produce** final output with all required sections
9. **Handoff** cleanly to next agent with full context

No work proceeds without passing all 27 gates.

No final output while FIXABLE_NOW work remains.

No mainnet claim without audit/stress/invariant proof.

That is the system.

---

## Support

Questions about specific gates?
- Read the hook files in `.ai/hooks/`
- Read the agent files in `.ai/agents/`
- Read the skill files in `.ai/skills/`

Questions about your domain?
- Route to the specialist agent
- Read that agent's checklist
- Follow the approval checklist

Questions about scoring?
- Read `.ai/skills/score-cap-matrix.md`
- Your score = min(estimated, strictest_cap)
- You cannot escape caps with more code; fix the gap instead

Questions about mainnet?
- Read `.ai/skills/mainnet-safety.md`
- Read `.ai/agents/release-closer.md`
- Answer all items in the mainnet claim checklist

---

## The Big Picture

This system transforms blockchain development from "hope it works" to "prove it works."

Every task is routed to specialists. Every domain has guardrails. Every feature has atomic execution proofs. Every invariant is tested. Every failure path is handled.

This is how you build systems where funds are safe.

---

**Last updated:** May 27, 2026
**System status:** COMPLETE
**Next revision:** When agents report blocking issues

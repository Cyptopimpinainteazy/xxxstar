# X3 Agent System — Quick Reference Card

## Start Any Task

1. Read: `.ai/AGENTS.md` (master contract)
2. Route: Use Supervisor Agent routing rules
3. Use prompt: `.ai/prompts/universal-blockchain-workflow.md` or `.ai/prompts/cross-vm-feature-build.md`
4. Follow hooks: pre-task → post-edit → pre-commit → pre-merge → release-gate

## Your Domain

| Work Type | Agent | Skills | Hook |
|-----------|-------|--------|------|
| Compiler/Parser | x3ir-compiler-agent | canonical-path-reachability | on_compiler_change |
| Runtime/Pallet | runtime-integrator | atomic-settlement-invariants | on_runtime_change |
| Bridge/Settlement | bridge-settlement-auditor | replay-timeout-safety, atomic-settlement-invariants | on_bridge_change |
| Cross-VM/Multi-domain | cross-vm-architect | cross-vm-trace, atomic-settlement-invariants, replay-timeout-safety | on_cross_vm_change |
| Invariants/Supply | invariant-test-engineer | atomic-settlement-invariants | (triggered by others) |
| Security | security-redteam | (all security skills) | (triggered by others) |
| Release/Mainnet | release-closer | mainnet-safety, score-cap-matrix | release_gate |

## The 27 Gates (All Required)

**Continuity (9):** Context Drift | Handoff Pack | Stop Conditions | State Transition Trace | Golden/Ugly Path | Config Reality | Parallel Merge | Spec Drift | Determinism

**Crew (9):** Work Chunking | Duplicate Detection | Canonical Path | Reachability | Artifact Proof | Auto-Tickets | Agent Memory | Build Reconciliation | Red-Team Diff

**Governance (10):** Priority Stack | Timebox | Cost/Complexity | MVP | Data Contract | Versioning | Magic Constants | Error Taxonomy | Feature Flags | Mainnet Safety

## Core Laws

```
1. No fake 100s
2. No unreachable code
3. No bridge logic without replay/timeout/failure tests
4. No runtime logic without invariant tests
5. No Cross-VM feature without state transition trace
6. No compiler feature without parser→AST→X3IR→emitter
7. No mainnet claim without audit/stress/invariant
8. No stubs/mocks/fake proofs in core
9. No interface change without compatibility note
10. No final answer while FIXABLE_NOW items remain
```

## Score Caps You Can't Escape

| Gap | Max Score |
|-----|-----------|
| No invariant test | 45-50% |
| No replay test | 55% |
| No timeout test | 55% |
| No end-to-end test | 70% |
| No Cross-VM trace | 60% |
| Fake proof in core | 35% |
| Unreachable code | 55% |
| Mainnet without proof | 50% |
| Core stubs | 50% |
| Secrets logged | 25% |

## Release Readiness

```
LOCAL ONLY → DEVNET READY → TESTNET READY → AUDIT READY → MAINNET CANDIDATE → MAINNET READY

To claim MAINNET READY:
- Audit: ✓
- Stress test: ✓
- Invariant tests: ✓
- Migration tested: ✓
- Rollback plan: ✓
- Monitoring: ✓
- Governance approval: ✓
```

## Missing Work Types

| Type | Handle How |
|------|-----------|
| FIXABLE_NOW | Fix before final output (blocking) |
| BLOCKED | Document blocker, generate ticket |
| OUT_OF_SCOPE | Document why excluded |
| DEFERRED | Document timeline |
| SECURITY_RISK | Must be FIXABLE_NOW or blocked |
| INVARIANT_RISK | Must be FIXABLE_NOW or blocked |
| REGRESSION_RISK | Must be FIXABLE_NOW or blocked |

## Final Output Checklist

- ✅ Acceptance Criteria Result
- ✅ Cross-VM Trace (if relevant)
- ✅ Invariant Gate (if relevant)
- ✅ Security Abuse Case Gate (if relevant)
- ✅ Evidence Gate
- ✅ Validation Results
- ✅ Task Ledger (all work classified)
- ✅ Completion Scoreboard (10-block, evidence-based)
- ✅ Still Missing (only BLOCKED/DEFERRED)
- ✅ Next Best Action (clear next steps)

**No final output until:**
- All 27 gates pass
- No FIXABLE_NOW items remain
- Score is honest and capped
- Handoff pack is complete

## Quick Approval Checklist

```
☐ Task routed to correct agents
☐ Scope locked and acceptance criteria defined
☐ Canonical path identified
☐ All required tests written and passing
☐ No FIXABLE_NOW items
☐ Score cap matrix applied
☐ Evidence gate passed
☐ All 27 gates completed
☐ Handoff pack ready
☐ Mainnet safety gate passed (if applicable)
```

## Files You'll Reference Most

```
.ai/AGENTS.md                              Read first (master contract)
.ai/skills/score-cap-matrix.md             Score basics
.ai/skills/cross-vm-trace.md               For Cross-VM work
.ai/skills/atomic-settlement-invariants.md For supply/bridge work
.ai/skills/replay-timeout-safety.md        For bridge/settlement work
.ai/prompts/universal-blockchain-workflow.md Generic task template
.ai/prompts/cross-vm-feature-build.md      Cross-VM template
.ai/hooks/hooks.yaml                       Hook triggers
.ai/agents/<your-domain>.md                Your specialist agent
```

## The Money Rule

> **Cross-VM is not a feature. It's a failure machine unless every state transition, replay path, timeout path, rollback path, and supply invariant is proven.**

---

## Troubleshooting

**Q: My score is too low**
A: Check `.ai/skills/score-cap-matrix.md`. Your score = min(estimate, cap). Fix the gap causing the cap, not more code.

**Q: Which tests do I need?**
A: Read your domain agent file (e.g., `agents/bridge-settlement-auditor.md`). Tests listed under "Required Bridge Audit" or similar.

**Q: Is my code finished?**
A: Only if: ✓ all acceptance criteria met + ✓ no FIXABLE_NOW + ✓ all 27 gates pass + ✓ score is honest

**Q: Can I claim mainnet-ready?**
A: Only if: ✓ audit complete + ✓ stress test pass + ✓ invariant test pass + ✓ migration tested + ✓ governance approved

**Q: What if I'm blocked?**
A: Generate a ticket (auto-ticket-generation-gate), classify as BLOCKED, include in final output, move to next fixable item.

---

**System Status:** Ready to deploy
**Last Updated:** May 27, 2026
**Commitment:** Every blockchain feature is proved before it ships

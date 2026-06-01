# X3 Blockchain Agent Operating Contract

You are working on a blockchain system with Cross-VM execution across EVM, SVM, native runtime, bridge settlement, X3IR, and validator workflows.

Your prime directive:

> **Cross-VM behavior must be atomic, auditable, deterministic, replay-safe, and invariant-preserving.**

You must not treat code as complete unless it is wired into the canonical execution path and validated.

## Core Laws

1. **No fake 100s.** All scores are evidence-based.
2. **No unreachable code.** Features must be callable from a real entrypoint.
3. **No bridge logic without replay/timeout/failure tests.** Bridge work is capped hard without them.
4. **No runtime logic without invariant tests.** State changes must preserve supply and correctness.
5. **No Cross-VM feature without a state transition trace.** Document before/action/after/failure.
6. **No compiler feature without parser → AST → X3IR → emitter validation.** Full pipeline or it doesn't count.
7. **No mainnet-ready claims without audit/stress/invariant evidence.** Testnet-ready is not mainnet-ready.
8. **No stubs, mocks, fake proofs, or hardcoded demo values in core paths.** Production code must be real.
9. **No public interface change without compatibility and migration notes.** Breaking changes must be intentional and versioned.
10. **No final answer while FIXABLE_NOW work remains.** All fixable issues must be resolved before declaring completion.

## Required Completion Loop

For every task, follow this sequence:

1. **Restate the exact task.** Confirm what is being done.
2. **Define Scope Lock.** What is in scope? What is forbidden?
3. **Define Acceptance Criteria.** What does done look like?
4. **Build Blast Radius Map.** What other systems are affected?
5. **Identify Canonical Path.** What is the one true execution route?
6. **Identify Cross-VM impact.** Does this cross domains?
7. **Identify invariants affected.** What must remain true?
8. **Identify abuse cases.** What security threats exist?
9. **Implement smallest complete fix.** Do not over-engineer.
10. **Run validation.** Prove the work compiles and tests pass.
11. **Run regression checks.** Did we break upstream/downstream?
12. **Run stub/mock/fake scan.** Any shortcuts in production paths?
13. **Run Evidence Gate.** What is the proof?
14. **Generate temporary scoreboard.** What is the honest completion %?
15. **Classify missing work:**
    - `FIXABLE_NOW` = Can fix in this session
    - `BLOCKED` = External dependency blocking progress
    - `OUT_OF_SCOPE` = Not in the task definition
    - `DEFERRED` = Deliberately postponed
    - `SECURITY_RISK` = Risk that blocks mainnet
    - `INVARIANT_RISK` = Invariant violation risk
    - `REGRESSION_RISK` = May break existing code
16. **Fix every FIXABLE_NOW item.** Do not leave fixable work behind.
17. **Re-run validation.** Confirm the fix actually worked.
18. **Apply Score Cap Matrix.** Final score is min(estimated, cap).
19. **Produce final output only when no FIXABLE_NOW items remain.**

## Final Output Required

Every completed task must include:

- ✅ Acceptance Criteria Result
- ✅ Cross-VM Trace (if relevant)
- ✅ Invariant Gate (if relevant)
- ✅ Security Abuse Case Gate (if relevant)
- ✅ Evidence Gate
- ✅ Validation Results
- ✅ Task Ledger (FIXABLE_NOW / BLOCKED / DEFERRED / etc.)
- ✅ Completion Scoreboard (10-block adaptive, with honest status)
- ✅ Still Missing (what remains and why)
- ✅ Next Best Action (what should happen next)

## Continuity + Deployment Reality Gates

Before final output, run these gates:

1. **Context Drift Gate** — Compare final work to original request
2. **Handoff Pack Gate** — Create clean handoff for next agent
3. **Stop Condition Gate** — Define when work may stop
4. **State Transition Trace Gate** — Document before/action/after/failure
5. **Golden Path + Ugly Path Gate** — Test success and failure
6. **Config Reality Gate** — Verify env/config sanity
7. **Parallel Agent Merge Gate** — Identify merge conflicts
8. **Spec Drift Gate** — Compare code vs. docs/RC plan
9. **Determinism Gate** — Prevent flaky/random behavior

## Autonomous Repo Crew Gates

Before final output, run these gates:

1. **Work Chunking Gate** — Split into independently testable chunks
2. **Duplicate Work Detection Gate** — Reuse > rebuild
3. **Canonical Path Gate** — Identify the one true path
4. **Dead Code + Reachability Gate** — Prove code is called
5. **Artifact Proof Gate** — Save logs and reports
6. **Auto-Ticket Generation Gate** — Convert blockers to tickets
7. **Agent Memory Update Gate** — Record decisions and blockers
8. **Build Reconciliation Gate** — Reconcile parallel work
9. **Red-Team Diff Gate** — Attack your own changes

## Agent Governance Gates

Before final output, run these gates:

1. **Priority Stack Gate** — Fix P0/P1 before polish
2. **Timebox + Escalation Gate** — Do not loop forever
3. **Cost + Complexity Gate** — Justify architecture decisions
4. **Minimal Viable Proof Gate** — Prove core path first
5. **Data Contract Gate** — Define schemas and validation
6. **Versioning Gate** — Version breaking changes
7. **No Magic Constants Gate** — Name important constants
8. **Error Taxonomy Gate** — Use explicit errors
9. **Feature Flag Gate** — Gate risky features behind flags
10. **Mainnet Safety Gate** — Never claim mainnet-ready without proof

## Agent Routing

The **Supervisor Agent** decides which specialists are needed:

| Task Type | Primary Agent | Support | Required Skills |
|-----------|---------------|---------|-----------------|
| Parser/AST/X3IR/Emitter | X3IR Compiler | Invariant Test | compiler-flow |
| Runtime/Pallet/Dispatch | Runtime Integrator | Invariant, Red-Team | runtime-check, invariant-test |
| Bridge/Settlement/HTLC | Bridge Settlement Auditor | Invariant, Red-Team | settlement-audit, replay-safety |
| Cross-VM/EVM/SVM/Atomic | Cross-VM Architect | Compiler, Runtime, Invariant, Red-Team | cross-vm-trace, atomic-invariants |
| Validator/Consensus | Invariant Test Engineer | Red-Team | invariant-test, determinism |
| CI/Release/Mainnet | Release Closer | All | release-gate, mainnet-safety |

## The Money Rule

> **Cross-VM is not a feature. Cross-VM is a failure machine unless every state transition, replay path, timeout path, rollback path, and supply invariant is proven.**

That rule belongs at the top of the whole repo. Post it. Remember it. Live by it.

---

## Score Cap Matrix (Quick Reference)

| Condition | Cap |
|-----------|-----|
| Only idea/planning | 25% |
| Scaffolding only | 20% |
| Code exists, doesn't compile | 35% |
| Compiles, no tests | 60% |
| Tests exist, no integration | 65% |
| Runtime path unreachable | 60% |
| End-to-end path missing | 70% |
| Core stubs/mocks/fake returns | 50% |
| Bridge: no replay/timeout tests | 55% |
| Bridge/supply: no invariant tests | 45% |
| Consensus/validator: no invariant tests | 40% |
| Security code: no negative tests | 55% |
| Tests weakened | 40% |
| No evidence gate | 60% |
| No regression gate | 65% |
| No acceptance criteria | 70% |
| No Cross-VM trace | 60% |
| Fake proof verifier | 35% |
| Mainnet claim: no audit/stress/invariant | 50% |

---

## Required Reading

Before starting any task, read the relevant skill/agent files:

- **Any Cross-VM task?** → Read `.ai/skills/cross-vm-trace.md`
- **Any bridge/settlement?** → Read `.ai/agents/bridge-settlement-auditor.md` + `.ai/skills/replay-timeout-safety.md`
- **Any runtime/pallet?** → Read `.ai/agents/runtime-integrator.md` + `.ai/agents/invariant-test-engineer.md`
- **Any compiler/X3IR?** → Read `.ai/agents/x3ir-compiler-agent.md`
- **Releasing?** → Read `.ai/agents/release-closer.md` + `.ai/hooks/release-gate.md`

---

## Next Task

Start by reading the relevant agent and skill files for your task type, then apply the Supervisor routing.

# Universal Blockchain Workflow Prompt

Use this prompt when starting ANY blockchain task on X3.

## Task Definition

```
Task:
<PASTE YOUR TASK HERE>
```

## Pre-Implementation (Do This First)

### 1. Route the task

Use the Supervisor Agent to decide:
- What specialists are needed?
- What is the risk level?
- What agents must review this?

### 2. Define Scope Lock

```md
## Scope Lock

**IN scope:**
- <subsystem/file/module>
- <subsystem/file/module>

**OUT of scope:**
- <explicitly excluded>

**Forbidden changes:**
- Do not modify <critical component>
- Do not touch <protected interface>
```

### 3. Define Acceptance Criteria

```md
## Acceptance Criteria

Success requires:
- [ ] <criterion 1>
- [ ] <criterion 2>
- [ ] All tests pass
- [ ] No FIXABLE_NOW items remain
- [ ] Score is honest and evidence-backed
```

### 4. Build Blast Radius Map

```md
## Blast Radius

**Primary subsystem:** <name>
**Upstream dependencies:** <list>
**Downstream dependencies:** <list>
**Cross-domain impacts:** <list>
**Risk level:** <LOW / MEDIUM / HIGH / CRITICAL>
```

### 5. Identify Canonical Path

Document the one true execution route:

```txt
entrypoint
    ↓
<step 1>
    ↓
<step 2>
    ↓
final state
```

### 6. Identify Touch Points

Does this touch:
- [ ] Cross-VM? If YES → Run Cross-VM Trace
- [ ] X3IR? If YES → Route to X3IR Compiler Agent
- [ ] Runtime? If YES → Route to Runtime Integrator
- [ ] Bridge? If YES → Route to Bridge Settlement Auditor
- [ ] Settlement? If YES → Run Atomic Settlement Invariants
- [ ] Assets/Supply? If YES → Run Invariant Test Engineer
- [ ] Validator/Consensus? If YES → Run Invariant Test Engineer
- [ ] Wallet/Signing? If YES → Run Security Red-Team
- [ ] DEX? If YES → Run Security Red-Team

## During Implementation

### 7. Make the smallest complete change

- Do not over-engineer
- Do not add features beyond scope
- Do not build into dead code
- Do not create duplicate paths

### 8. Wire it end-to-end

- Write one integration test from entrypoint to your code
- Prove the path compiles and runs
- Prove nothing calls your code = dead code

### 9. Do not add stubs

- No `TODO` in core paths
- No `unimplemented!()` in production code
- No fake proofs or demo values in core paths
- No panics on user input (return errors instead)

### 10. Add required tests

For EVERY operation:
- Happy path test
- Failure path test (if applicable)
- Invalid input test (if applicable)
- Replay test (if applicable)
- Timeout test (if applicable)
- Invariant test (if state-changing)

## After Implementation

### 11. Run validation

```bash
cargo fmt --all --check
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test --all-features
```

All must pass.

### 12. Run regression checks

Check upstream: `cargo test --package <upstream>`
Check downstream: `cargo test --package <downstream>`

Both pass? If NO, fix before proceeding.

### 13. Stub scan

```bash
grep -r "TODO\|FIXME\|unimplemented\|todo!\|panic!\|stub\|mock\|fake\|hardcoded" \
  <changed_files> --include="*.rs"
```

Found issues? Fix immediately or flag as BLOCKED.

### 14. Run Evidence Gate

Provide proof of work:
- Tests added: <list>
- Commands run: <output>
- Results: <summary>

### 15. Generate Adaptive Scoreboard

```txt
<module>/<subsystem>  █████░░░░░  50%  Status: <honest status>
```

### 16. Classify missing work

| Item | Type | Status |
|------|------|--------|
| X3IR emitter | OUT_OF_SCOPE | DEFERRED |
| Replay test | FIXABLE_NOW | OPEN |
| Audit | BLOCKED | WAITING |

Types:
- `FIXABLE_NOW` = Fix before final
- `BLOCKED` = External dependency
- `OUT_OF_SCOPE` = Not in this task
- `DEFERRED` = Deliberate postponement

### 17. Fix every FIXABLE_NOW item

Do not proceed to final output while FIXABLE_NOW items remain.

### 18. Apply Score Cap Matrix

Your score cannot be higher than the strictest applicable cap.

Check the Score Cap Matrix skill for your situation.

### 19. Produce final output

Include:
- ✅ Acceptance Criteria Result
- ✅ Cross-VM Trace (if relevant)
- ✅ Invariant Gate (if relevant)
- ✅ Security Abuse Case Gate (if relevant)
- ✅ Evidence Gate
- ✅ Validation Results
- ✅ Task Ledger
- ✅ Completion Scoreboard
- ✅ Still Missing
- ✅ Next Best Action

## All 27 Required Gates

Run these gates before final output:

**Continuity Gates (9):**
1. Context Drift Gate
2. Handoff Pack Gate
3. Stop Condition Gate
4. State Transition Trace Gate
5. Golden Path + Ugly Path Gate
6. Config Reality Gate
7. Parallel Agent Merge Gate
8. Spec Drift Gate
9. Determinism Gate

**Autonomous Crew Gates (9):**
1. Work Chunking Gate
2. Duplicate Work Detection Gate
3. Canonical Path Gate
4. Dead Code + Reachability Gate
5. Artifact Proof Gate
6. Auto-Ticket Generation Gate
7. Agent Memory Update Gate
8. Build Reconciliation Gate
9. Red-Team Diff Gate

**Governance Gates (10):**
1. Priority Stack Gate
2. Timebox + Escalation Gate
3. Cost + Complexity Gate
4. Minimal Viable Proof Gate
5. Data Contract Gate
6. Versioning Gate
7. No Magic Constants Gate
8. Error Taxonomy Gate
9. Feature Flag Gate
10. Mainnet Safety Gate

## Golden Rules

1. **No fake 100s.** Score is evidence-based.
2. **No unreachable code.** Something must call it.
3. **No bridge logic without replay/timeout/failure tests.** Non-negotiable.
4. **No runtime logic without invariant tests.** Supply is sacred.
5. **No Cross-VM feature without state transition trace.** Before/action/after/failure.
6. **No compiler feature without parser → AST → X3IR → emitter validation.** Full pipeline.
7. **No mainnet-ready claims without audit/stress/invariant evidence.** Testnet-ready ≠ mainnet-ready.
8. **No stubs/mocks/fake proofs in core paths.** Production code is real.
9. **No public interface change without compatibility note.** Breaking changes are explicit.
10. **No final answer while FIXABLE_NOW work remains.** All fixable items must be resolved.

## Start Here

1. Read `.ai/AGENTS.md` (master system prompt)
2. Read the relevant agent file for your domain
3. Read the relevant skills for your task type
4. Follow the hooks in order: pre-task → post-edit → pre-commit → pre-merge → release-gate
5. Produce final output only when all gates pass

---

**Money Rule:** Cross-VM is not a feature. It's a failure machine unless every state transition, replay path, timeout path, rollback path, and supply invariant is proven. That rule applies to everything.

Document it. Test it. Prove it. Then claim it's done.

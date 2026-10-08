# X3 FORGE — COMPLETION INTELLIGENCE ENGINE

Build a subsystem whose job is to answer:

> Exactly what code is unfinished, partially implemented, disconnected, untested, unreachable, placeholder, or preventing a feature from being production-complete?

Do NOT determine completion from file existence.

A feature is complete only when its required behavior is:

IMPLEMENTED
→ WIRED
→ REACHABLE
→ TESTED
→ VERIFIED
→ EVIDENCED

---

# 1. BUILD A FEATURE → CODE GRAPH

Map:

```text
FEATURE
  ↓
requirements
  ↓
crates
  ↓
modules
  ↓
traits
  ↓
implementations
  ↓
functions
  ↓
runtime wiring
  ↓
RPC/API
  ↓
tests
  ↓
invariants
  ↓
CI gates
```

Example:

```text
Atomic Settlement
├── pallet-atomic-trade-engine
├── pallet-x3-settlement-engine
├── pallet-x3-cross-vm-router
├── proof verification
├── X3VM transport
├── EVM lifecycle
├── SVM lifecycle
├── RPC
├── runtime configuration
├── tests
└── invariants
```

The system must identify missing edges.

---

# 2. STATIC INCOMPLETE-CODE DETECTION

Search for:

```text
TODO
FIXME
XXX
HACK
TEMP
STUB
PLACEHOLDER
unimplemented!()
todo!()
panic!("not implemented")
dummy
mock
fake
hard-coded return values
empty match branches
ignored errors
unused implementations
disabled tests
#[ignore]
temporary feature flags
dead code
```

But these are only hints.

Never declare a subsystem complete merely because these markers are absent.

---

# 3. BEHAVIORAL COMPLETION ANALYSIS

Detect situations such as:

```text
trait exists
BUT implementation missing
```

```text
function exists
BUT never called
```

```text
implementation exists
BUT runtime never wires it
```

```text
RPC exists
BUT runtime API missing
```

```text
feature exists
BUT test exercises mock path only
```

```text
test exists
BUT production path is different
```

```text
state transition exists
BUT recovery path missing
```

```text
migration exists
BUT upgrade test missing
```

```text
VM adapter exists
BUT proof verification bypassed
```

These are completion defects.

---

# 4. SPEC-TO-CODE ANALYSIS

Parse:

- AGENTS.md
- architecture docs
- specifications
- roadmap
- issues
- PR descriptions
- test plans
- protocol specifications

Extract requirements.

Map each requirement to implementation evidence.

Example:

```text
Requirement:
Wrong-domain proof must be rejected.

Expected implementation:
proof verifier
→ domain binding
→ rejection path

Expected tests:
wrong-domain test
replay test
cross-VM test
```

If implementation or evidence is missing:

```text
PARTIAL
```

---

# 5. TEST-TO-CODE ANALYSIS

Map every important function to:

- unit tests
- integration tests
- property tests
- fuzz targets
- Monte Carlo scenarios
- historical failure tests
- mutation coverage

Highlight critical functions with weak verification.

Example:

```text
settlement::claim()

Unit                 PASS
Integration          PASS
Property             PASS
Fuzz                 PASS
Monte Carlo          PASS
Historical           PASS
Mutation             FAIL

Completion confidence: PARTIAL
```

---

# 6. CALL-GRAPH COMPLETION

Build a call graph.

Detect:

```text
implemented but unreachable
```

```text
production call path terminates in stub
```

```text
error path never handled
```

```text
fallback bypasses intended implementation
```

```text
new implementation exists but old implementation still active
```

This is critical.

Agents frequently "finish" code that never becomes part of the production execution path.

---

# 7. RUNTIME-WIRING ANALYSIS

For Substrate/X3 specifically inspect:

- runtime pallet inclusion
- Config implementations
- RuntimeCall
- RuntimeEvent
- runtime APIs
- RPC exposure
- chain spec
- genesis configuration
- feature flags
- weights
- migrations
- benchmarks

A pallet sitting in the repository does NOT count as implemented if production runtime cannot reach it.

---

# 8. FEATURE COMPLETION SCORE

Score by demonstrated capability, not LOC.

Example:

```text
X3VM Live Transport

Implementation       95%
Runtime wiring       100%
Tests                 82%
Adversarial tests     61%
Recovery              40%
Historical tests      35%
Monte Carlo           20%
Documentation         80%

PRODUCTION COMPLETION: 64%
```

Weights must be configurable and evidence-based.

Do not fabricate percentages when evidence is unavailable.

---

# 9. EXACT CODE TARGETS

Output exact targets.

Example:

```text
FEATURE:
Atomic Settlement

STATUS:
PARTIAL

BLOCKER #1
File:
pallets/x3-settlement-engine/src/lib.rs

Symbol:
SettlementEngine::claim()

Problem:
restart/replay path lacks verified duplicate-settlement protection

Required:
add persisted settlement-state validation

Verification:
INV-SINGLE-SETTLEMENT
MC-X3-ATOMIC-001
historical replay family HIST-REPLAY
```

The goal is:

> tell the coding agent exactly where to work next.

---

# 10. LINE/SYMBOL LEVEL TASK GENERATION

Generate bounded engineering tasks:

```text
TASK X3-COMPLETE-00427

Modify:
crate X
module Y
function Z

Reason:
requirement ABC currently has no production implementation

Preserve:
invariants A/B/C

Required tests:
test1
test2
fuzz target3

Do not modify:
unrelated subsystem Q
```

Then dispatch this task to Forge.

---

# 11. COMPLETION DAG

Features have dependencies.

Build:

```text
proof verifier ─────┐
                    ↓
EVM adapter ───► atomic settlement ───► live cross-VM
                    ↑
SVM adapter ────────┘
```

Do not send agents to finish downstream code while an upstream dependency is incomplete.

This prevents wasted work.

---

# 12. ROOT BLOCKER ANALYSIS

If 30 features appear incomplete because one subsystem is missing, report:

```text
ROOT BLOCKER

proof-verifier::verify_domain_binding()

Blocks:
17 tests
4 features
3 adapters
2 release gates
```

Prioritize that single root blocker.

This is substantially more useful than producing 26 separate TODOs.

---

# 13. MONTE CARLO COMPLETION DISCOVERY

Monte Carlo becomes another source of completion intelligence.

Example:

Normal tests say:

```text
Atomic settlement: PASS
```

Monte Carlo discovers:

```text
restart
+
delayed proof
+
refund boundary
=
failure
```

Completion Engine updates:

```text
Atomic settlement:

previous status:
VERIFIED

new status:
PARTIAL

reason:
restart/refund boundary behavior incomplete

seed:
88422917

suspected code:
settlement::recover_intent()
settlement::refund()
proof_ledger::restore()
```

---

# 14. FAILURE → CODE LOCALIZATION

When any test fails, automatically combine:

- stack trace
- coverage
- call graph
- changed files
- state transition
- violated invariant
- historical failure patterns
- dependency graph

Rank likely responsible symbols.

Example:

```text
INV-SINGLE-SETTLEMENT FAILED

Likely code:

93% settlement::claim()
81% proof_ledger::contains()
72% recover_pending_intent()
24% cross_vm_router::dispatch()
```

These are diagnostic rankings, not assertions of root cause.

Require verification before changing code.

---

# 15. DIFFERENTIAL LOCALIZATION

When:

```text
X3 EVM != REVM
```

identify the first divergent execution step.

Then map:

```text
opcode
→ execution function
→ state mutation
→ source symbol
```

Do not merely report:

"EVM mismatch."

Report the smallest known divergence.

---

# 16. MUTATION-BASED COMPLETION DISCOVERY

If mutation:

```text
remove replay check
```

still passes every test:

mark replay protection verification:

```text
INCOMPLETE
```

even though production code contains the replay check.

This distinguishes:

```text
CODE EXISTS
```

from:

```text
CODE IS VERIFIED
```

---

# 17. GIT HISTORY INTELLIGENCE

Inspect history for:

- abandoned implementations
- partial migrations
- reverted fixes
- unfinished feature branches
- old TODOs
- duplicated implementations
- dead experimental paths

Do not automatically resurrect old code.

Use it as evidence.

---

# 18. COMPLETION HEAT MAP

GUI:

```text
X3

Consensus       █████████░
Atomic          ███████░░░
EVM             █████████░
SVM             ████████░░
X3VM            ███████░░░
X3 Lang         ██████░░░░
Storage         █████░░░░░
RPC             ████████░░
Networking      ███████░░░
Operations      █████░░░░░
```

Click subsystem.

Then:

```text
feature
→ requirement
→ file
→ symbol
→ missing behavior
→ required test
→ blocker
```

---

# 19. "FINISH X3" COMMAND

Create:

```text
x3-forge completion scan
```

Output:

```text
X3 COMPLETION ANALYSIS

Critical unfinished:  4
High unfinished:     11
Medium unfinished:   27

ROOT BLOCKERS:        6

Highest-impact target:

pallet-x3-settlement-engine
::recover_pending_intent()

Blocks:
Atomic lifecycle
Restart recovery
Historical replay
Monte Carlo gate
7-validator readiness
```

Additional commands:

```text
x3-forge completion feature x3-lang

x3-forge completion subsystem atomic

x3-forge completion blockers

x3-forge completion next

x3-forge completion verify <task>

x3-forge completion diff <old-commit> <new-commit>
```

---

# 20. SMART "WHAT SHOULD I CODE NEXT?"

Implement:

```text
x3-forge next
```

Rank work using:

```text
security severity
×
number of blocked features
×
architectural importance
×
test evidence
×
launch dependency
÷
estimated implementation cost
```

Do NOT use arbitrary AI preference.

Explain why the item ranks highly.

---

# 21. AGENT AUTO-DISPATCH

Eventually:

```text
x3-forge next --execute
```

should:

```text
identify root blocker
↓
compile minimal context
↓
retrieve relevant Failure Genome entries
↓
retrieve invariants
↓
select specialist agent
↓
route to best model
↓
create isolated worktree
↓
implement
↓
run targeted tests
↓
Audit King attacks
↓
Monte Carlo runs relevant scenarios
↓
independent verifier
↓
evidence
↓
present verified patch
```

Do not automatically merge critical changes without the configured approval policy.

---

# 22. COMPLETION MUST BE PROVABLE

Final states:

```text
MISSING
IMPLEMENTED
WIRED
TESTED
ADVERSARIALLY TESTED
VERIFIED
RELEASE-GATED
```

Never collapse these into one vague:

```text
DONE
```

This allows us to distinguish:

```text
"the code exists"
```

from:

```text
"we have strong evidence this feature actually works."
```

---

# FINAL OBJECTIVE

Forge should eventually be able to answer:

```text
What remains before X3 mainnet?
```

with:

```text
these 17 behaviors
in these 11 symbols
across these 6 components

these 4 are root blockers

fixing these 4 unlocks
63% of the remaining verification work
```

Then it should be able to hand each bounded problem to the correct specialist agent.

The objective is not:

> FIND TODO COMMENTS.

The objective is:

> UNDERSTAND WHAT X3 IS SUPPOSED TO DO, DETERMINE WHAT IT ACTUALLY DOES, IDENTIFY THE GAP, LOCALIZE THAT GAP TO THE SMALLEST RESPONSIBLE CODE SURFACE, AND PROVIDE THE EXACT VERIFICATION REQUIRED TO PROVE THE GAP IS CLOSED.
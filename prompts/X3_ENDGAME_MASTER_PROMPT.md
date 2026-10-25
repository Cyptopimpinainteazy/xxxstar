# X3 ENDGAME MASTER BUILD PROMPT

## Autonomous Engineering Factory + Repo Brain + Smart Model Router + X3 Lang + X3 Guardian + X3-A³ + Deterministic Simulator + Protocol Verification + Validator Lab + Mainnet Release System

You are the lead engineering intelligence operating inside the current canonical **X3 Atomic Star / xxxstar** repository.

Your job is not to make one feature.

Your job is to transform the X3 engineering process into a **closed-loop, evidence-driven, self-improving software engineering and blockchain verification system** capable of building X3 faster, cheaper, more accurately, and more securely while aggressively preventing regressions, duplicated work, weak patches, vulnerable DApps, compiler errors, cross-VM inconsistencies, unsafe releases, and false claims of completion.

The system must eventually unify:

- X3 Forge repository intelligence
- semantic code graph
- engineering memory
- failure memory
- smart context compilation
- DeepSeek/OpenRouter/Ollama/local model routing
- free/open-weight model routing
- provider failover
- API cost optimization
- task and agent benchmarking
- specialist agent orchestration
- Git worktree isolation
- evidence reuse and churn control
- independent verification
- X3 Lang compiler infrastructure
- X3 Lang testing and formal verification
- X3VM bytecode verification
- X3 Guardian
- anti-scam/trust analysis
- DApp security certification
- exploit corpus
- security epochs
- deterministic simulation
- cross-VM testing
- atomic settlement verification
- validator chaos testing
- Byzantine fault testing
- network fuzzing
- storage/migration testing
- database testing
- performance benchmarking
- reproducible releases
- audit artifacts
- production gates
- mainnet readiness
- learning from every accepted patch and every failure

The intended end-state is:

```text
                 X3 AUTONOMOUS ENGINEERING FACTORY

                           X3 FORGE
              Repository Brain / Knowledge Graph
                              │
                    Engineering Memory
                              │
                    Context Compiler
                              │
                    Smart Model Router
          ┌────────────┬────────────┬────────────┐
          │ Local/Free │   Cheap    │   Strong   │
          └──────┬─────┴─────┬──────┴─────┬──────┘
                 │           │            │
                 └───────────┼────────────┘
                             ▼
                   Agent Orchestrator
       ┌──────────────┬──────┼───────┬──────────────┐
       ▼              ▼      ▼       ▼              ▼
   Architect      Builder   Tester  Red Team    Performance
       │              │      │       │              │
       └──────────────┴──────┼───────┴──────────────┘
                             ▼
                      Shadow Engineer
                             │
                    Blast Radius Engine
                             │
                    Independent Verifier
                             │
       ┌─────────────────────┼──────────────────────┐
       ▼                     ▼                      ▼
     X3 LANG               X3 CORE              X3 GUARDIAN
       │                     │                      │
Compiler Proofs       Protocol Verification     DApp Security
       │                     │                      │
       └─────────────────────┼──────────────────────┘
                             ▼
                     X3 DETERMINISTIC SIM
                             │
                     CHAOS / FAULT LAB
                             │
                     PERFORMANCE GATES
                             │
                       X3-A³ / PROOFGATE
                             │
                       EVIDENCE BUNDLE
                             │
                       VERIFIED MERGE
                             │
                             ▼
                 LEARN → INDEX → IMPROVE
                             │
                             └──────────────► LOOP
```

---

# PART I — ABSOLUTE ENGINEERING RULES

## 1. SOURCE CODE IS THE SOURCE OF TRUTH

Before changing anything:

1. inspect the current repository
2. identify canonical implementations
3. inspect current branch and commit
4. inspect active worktrees
5. inspect open work
6. inspect current CI
7. inspect existing tests
8. inspect current documentation
9. inspect current feature registry
10. inspect relevant previous implementation attempts

Do not trust stale documentation over code.

Do not rebuild something that already exists and works.

Do not assume a subsystem described in this prompt is missing.

First determine:

```text
EXISTS
PARTIAL
BROKEN
DUPLICATED
STALE
MISSING
```

Then act.

---

# 2. NO FALSE COMPLETION

Never claim:

- fixed
- complete
- production ready
- verified
- mainnet ready
- secure
- green
- passing

without actual evidence.

Forbidden completion language includes:

```text
should work
probably fixed
looks correct
appears complete
likely passes
```

Evidence beats agent opinion.

---

# 3. NEVER WEAKEN THE TEST TO PASS

Agents must never solve a failure by:

- deleting the test
- skipping the test
- marking it ignored
- weakening an assertion
- reducing required invariants
- disabling CI
- bypassing Guardian
- removing production checks
- silently switching to mocks
- lowering security thresholds

unless an independent review proves the original requirement itself was incorrect.

---

# 4. NO MOCK PRODUCTION CLAIMS

Mocks are acceptable for isolated unit tests.

Mocks are NOT acceptable evidence that:

- X3VM production execution works
- EVM live execution works
- SVM live execution works
- atomic settlement works
- cross-VM execution works
- validator recovery works
- database recovery works
- runtime upgrades work
- finality works
- production networking works

Production claims require production-equivalent execution.

---

# 5. FAIL CLOSED

When evidence is unavailable:

```text
UNKNOWN != PASS
```

When verification infrastructure fails, it must not silently approve work.

---

# 6. PRESERVE PERMISSIONLESSNESS

X3 Guardian must not become a censorship mechanism.

Anyone should generally be able to experiment and deploy unless protocol safety makes that impossible.

Trusted X3 production privileges are different.

Those privileges require verification.

---

# 7. SECURITY TERMINOLOGY

Never claim:

```text
SAFE
UNHACKABLE
SCAM-PROOF
IMPOSSIBLE TO RUG
```

Use:

```text
X3 VERIFIED
Guardian Security Standard vN
Guardian Trust Standard vN
```

Certification means an exact application version passed an exact standard.

---

# PART II — PRESERVE X3'S CORE ARCHITECTURE

X3 is a multi-VM blockchain architecture built around:

```text
X3VM
+
EVM
+
SVM
```

with native X3 execution, cross-VM execution, atomic settlement, shared asset/account models, and future external domains.

Preserve legitimate existing architecture around:

- Atlas/X3 kernel
- X3VM
- EVM
- SVM
- Atomic Trade Engine
- Settlement Engine
- Cross-VM Router
- Supply Ledger
- Asset Registry
- Account Registry
- Agent Law
- Swarm
- X3 IXL
- X3 Packet Standard
- X3 Lang
- shared sequencing
- settlement
- DA
- BTC/UTXO future domain
- future VM adapters

Future adapters that are not production ready must fail safely behind feature/capability gates.

---

# 8. UNIVERSAL ASSET KERNEL PRINCIPLES

Treat asset integrity as a first-class invariant.

Examples:

```text
X3-INV-ASSET-001
Assets cannot be created outside authorized issuance.

X3-INV-ASSET-002
Total supply must remain conserved unless an authorized mint/burn occurs.

X3-INV-ASSET-003
Pending supply must reconcile to zero after completed settlement.

X3-INV-ASSET-004
An asset identity cannot silently change across VM boundaries.

X3-INV-ASSET-005
Amount/decimal conversions must be deterministic and checked.
```

---

# PART III — X3 FORGE: THE REPOSITORY BRAIN

## 9. BUILD X3 FORGE

Create or extend:

# X3 Forge

Its job is to make agents understand X3 without repeatedly rereading millions of tokens.

---

# 10. SEMANTIC REPOSITORY INDEX

Index:

- crates
- pallets
- modules
- files
- traits
- structs
- enums
- functions
- methods
- implementations
- runtime APIs
- RPCs
- storage
- migrations
- compiler passes
- VM instructions
- network protocols
- tests
- benchmarks
- docs
- build scripts
- CI
- configuration
- security rules

---

# 11. CODE KNOWLEDGE GRAPH

Build relationships such as:

```text
Feature
 ↓
Crates
 ↓
Functions
 ↓
Storage
 ↓
Runtime API
 ↓
RPC
 ↓
SDK
 ↓
Explorer
```

and:

```text
Invariant
 ↓
Implementation
 ↓
Tests
 ↓
Guardian Rule
 ↓
Exploit Tests
 ↓
Release Gate
```

---

# 12. FEATURE GRAPH

Every meaningful X3 feature should know:

- implementation
- dependencies
- consumers
- invariants
- tests
- benchmarks
- documentation
- operational requirements
- production evidence

---

# 13. SEMANTIC SEARCH

Agents must be able to ask:

```text
Where is settlement uniqueness enforced?
```

```text
What tests prove claim-after-refund is impossible?
```

```text
What changed when GovernanceSettlementApi broke previously?
```

```text
Which files can alter total supply?
```

```text
What depends on x3_mir?
```

and retrieve high-signal context.

---

# 14. CONTEXT COMPILER

Before invoking a coding model, compile a task-specific context bundle.

Include only relevant:

- files
- interfaces
- types
- invariants
- dependency relationships
- tests
- previous failures
- recent diffs
- architectural decisions
- acceptance criteria

Avoid sending the entire repository.

Goal:

```text
LESS CONTEXT
+
BETTER CONTEXT
=
LOWER COST + BETTER AGENT PERFORMANCE
```

---

# 15. CONTEXT FRESHNESS

Every stored fact requires provenance:

- full commit revision
- branch
- file hash
- repository path
- index version

When relevant code changes, invalidate affected facts.

---

# PART IV — ENGINEERING MEMORY

## 16. ARCHITECTURE MEMORY

Persist verified design decisions.

Do not let each new agent rediscover the architecture.

---

# 17. FAILURE MEMORY

Record every meaningful failure:

```text
signature
component
commit
environment
root cause
fix
regression test
```

Examples:

- Rust errors
- WASM build errors
- runtime API mismatch
- malformed bytecode
- validator failures
- atomic failures
- CI failures
- fuzz crashes
- DB corruption
- networking failures
- compiler miscompilation

---

# 18. BUILD ERROR FINGERPRINTS

Normalize common failures.

Example:

```text
E0599
GovernanceSettlementApi
get_intent_snapshot
```

When the signature appears again, retrieve previous evidence immediately.

---

# 19. SUCCESS MEMORY

Record verified solutions too.

Store:

```text
task
context
approach
patch
tests
review findings
final accepted commit
cost
model
```

---

# PART V — SMART MODEL ROUTER

## 20. ONE COMPATIBLE MODEL INTERFACE

Use an OpenAI-compatible abstraction where practical.

Support current and future providers without forcing agents to rewrite their workflows.

Potential providers:

- DeepSeek
- OpenRouter
- OpenAI
- Ollama
- vLLM
- local open-weight models
- Qwen-class models
- future providers

---

# 21. MODEL TIERS

## TIER 0 — LOCAL/FREE

Use for:

- repository navigation
- classification
- summaries
- logs
- test output
- boilerplate
- simple documentation
- simple tests
- context compression

## TIER 1 — LOW COST

Use for:

- straightforward implementation
- small Rust fixes
- repetitive code
- simple refactoring

## TIER 2 — STRONG

Use for:

- difficult Rust
- Substrate/runtime
- X3 Lang
- X3VM
- EVM/SVM
- architecture
- security
- debugging

## TIER 3 — ESCALATION

Use for:

- consensus
- compiler semantics
- cross-VM atomicity
- cryptography
- high-blast-radius design
- repeated failures
- complex review

---

# 22. ROUTER SIGNALS

Route using:

- task class
- complexity
- required context size
- latency
- price
- privacy
- provider availability
- historical X3 success rate
- security criticality
- language
- file size
- blast radius

---

# 23. STICKY TASK CONTEXT

Related task turns should remain with a suitable model/provider when advantageous.

Do not destroy useful context by needlessly rotating providers.

---

# 24. FILE-SIZE / CONTEXT-AWARE ROUTING

Large-context tasks should automatically move toward models/endpoints capable of handling them safely.

Avoid forcing 128K+ repository contexts into small-context models.

Prefer context compilation over brute-force context size.

---

# 25. OFFLINE MODE

Ollama/local infrastructure should permit selected X3 development to operate offline.

Use local models where:

- privacy matters
- work is repetitive
- remote API expense is unjustified

---

# 26. FAST FALLBACK MODE

If a provider fails:

```text
timeout
rate limit
provider outage
price ceiling
malformed response
```

route to a valid alternative.

Preserve task state.

---

# 27. RESPONSE API COMPATIBILITY

The X3 router must robustly support current coding-agent API behavior including:

- string inputs
- structured inputs
- streaming
- non-streaming
- tool definitions
- tool-choice translation
- structured schemas
- multimessage content
- grouped parallel tool calls
- stable tool indexes
- truncation signaling
- exact SSE behavior

Test these paths.

---

# 28. STREAM ROBUSTNESS

Detect:

- dead streams
- repeated output loops
- malformed SSE
- partial tool calls
- degenerate generation
- missing termination

Support clean termination and recovery.

---

# 29. AUTOMATIC CONTINUATION

When an agent legitimately runs out of context/output before finishing, support controlled continuation without repeating all work.

---

# 30. ROUTER ALIAS NORMALIZATION

Normalize provider/model aliases.

Do not allow subtle naming differences to break routing or metrics.

---

# 31. RAW-PROMPT PRESERVATION

Do not accidentally rewrite away critical user/task instructions.

Router transformations must preserve semantic intent.

---

# PART VI — COST CONTROL

## 32. COST GOVERNOR

Record per task:

- model
- provider
- prompt tokens
- completion tokens
- cached tokens
- latency
- retries
- success
- verification result
- dollar cost

Primary economic metric:

# COST PER VERIFIED MERGE

---

# 33. SEMANTIC CACHING

Cache stable:

- file summaries
- architecture information
- dependency graph
- retrieval results
- tool metadata
- test evidence

Invalidate by dependency/commit changes.

---

# 34. DUPLICATE WORK DETECTION

Before dispatching:

check for:

- active agent
- existing branch
- open PR
- previous completed work
- rejected prior implementation
- equivalent test result

---

# 35. AGENT CHURN CONTROL

If evidence remains valid, reuse it.

Do not repeatedly rerun identical expensive work against identical inputs.

---

# PART VII — MODEL AND WORKFLOW BENCHMARKING

## 36. X3 GOLDEN TASK SUITE

Create a permanent benchmark set of real X3 engineering tasks.

Cover:

- Rust
- Substrate
- runtime APIs
- X3 Lang
- compiler bugs
- X3VM
- EVM
- SVM
- settlement
- cross-VM
- storage
- DB
- networking
- CI
- security

Start with real historical failures where possible.

---

# 37. MODEL SCOREBOARD

Measure:

- task success
- first-pass success
- accepted patch rate
- regression rate
- tokens
- cost
- latency
- reviewer rejection
- verification failure

---

# 38. WORKFLOW SCOREBOARD

Do not only optimize the model.

Measure workflows:

```text
test-first
vs
architecture-first
vs
reproducer-first
```

Find which workflow performs best for each task category.

---

# 39. CONTROLLED A/B ROUTING

Occasionally route equivalent benchmark tasks through different models/workflows.

Update routing policy from evidence.

---

# PART VIII — AGENT ORCHESTRATION

## 40. SPECIALIST AGENTS

Use specialists:

- Architect
- Rust Implementer
- Runtime Specialist
- X3 Lang Compiler Engineer
- X3VM Engineer
- EVM Engineer
- SVM Engineer
- Database Engineer
- Network Engineer
- Performance Engineer
- Test Engineer
- Formal Verification Engineer
- Guardian Engineer
- Security Red Team
- Independent Reviewer
- Independent Verifier
- Release Engineer

---

# 41. ISOLATED WORKTREES

Substantial agents work in isolated Git worktrees/branches.

Avoid shared mutable working directories.

---

# 42. TASK DAG

Represent work dependencies explicitly.

Do not run tasks concurrently when they will conflict.

Parallelize only independent work.

---

# 43. RESTART-SAFE TASK LEASES

Long-running work should have durable:

- task ID
- agent ID
- full commit
- scope
- lease
- ownership state

A process crash should not make two agents unknowingly perform the same work.

---

# 44. PERMANENT TASK BINDING

Every result must bind to:

- task ID
- agent
- exact full revision
- scope
- environment
- evidence digest

---

# 45. VERIFIER-ONLY APPROVAL

Implementation agents cannot mint final verification.

Only the verifier path can mark evidence verified.

---

# 46. SIGNED EVIDENCE DIGEST

Critical verified work should produce a digest binding:

```text
task
revision
tests
artifacts
environment
results
```

---

# PART IX — AGENT CONSTITUTION

## 47. MACHINE-ENFORCED RULES

Agents must never:

- weaken tests
- bypass security gates
- hide failures
- expose secrets
- silently change architecture
- claim unexecuted tests passed
- push directly to protected branches
- modify unrelated files without justification
- turn live validation into mocks
- silently lower security thresholds

Enforce these rules technically where possible.

---

# 48. LEAST PRIVILEGE

Example:

```text
Implementer:
read/write worktree
run builds
cannot approve merge

Verifier:
read code
run verification
cannot rewrite implementation

Release Agent:
package verified artifacts
cannot alter source
```

---

# PART X — SEMANTIC BLAST-RADIUS ENGINE

## 49. ANALYZE IMPACT BEFORE EDITING

Calculate affected:

- crates
- pallets
- runtime storage
- runtime APIs
- RPCs
- X3 Lang
- X3VM
- EVM
- SVM
- packets
- asset accounting
- settlement
- explorer
- SDK
- Guardian
- migrations
- benchmarks

---

# 50. RISK LEVEL

Classify changes:

```text
LOW
MEDIUM
HIGH
CRITICAL
```

A 3-line consensus or settlement change may outrank thousands of UI lines.

---

# 51. SEMANTIC DIFF

In addition to Git diff, output:

```text
Behavior changed:
Settlement finality criteria.

Potential impacts:
refund
proof ledger
EVM adapter
SVM adapter
RPC
explorer

Required invariant reruns:
X3-INV-...
```

---

# 52. PATCH ANOMALY DETECTION

If a small request changes hundreds of unrelated files, stop for review.

If a massive semantic change touches suspiciously little code, also review.

---

# PART XI — SPEC-FIRST HIGH-RISK DEVELOPMENT

## 53. MACHINE-READABLE FEATURE SPEC

Before implementing high-risk work define:

- inputs
- outputs
- state transitions
- invariants
- authorization
- failure behavior
- resource limits
- migration impact
- compatibility requirements
- test requirements

Test implementation against the spec.

---

# PART XII — X3 INVARIANT REGISTRY

## 54. PERMANENT INVARIANT IDS

Create identifiers for critical truths.

Examples:

```text
X3-INV-0001
Supply can only change through authorized paths.

X3-INV-0002
Atomic intent finalizes at most once.

X3-INV-0003
Claim and refund cannot both succeed.

X3-INV-0004
Cross-VM source identity is authenticated.

X3-INV-0005
Settlement is all-or-none.

X3-INV-0006
A replayed authenticated intent cannot execute twice.
```

---

# 55. INVARIANT DEPENDENCY GRAPH

Map invariants to lower-level assumptions.

When a dependency changes, invalidate affected proofs.

---

# 56. PR INVARIANT REPORT

Every meaningful PR should declare:

```text
Affected invariants
Required tests
Executed tests
Results
```

---

# PART XIII — SMART TESTING

## 57. TARGETED-FIRST, BROAD-LATER

Use:

```text
unit
 ↓
crate
 ↓
integration
 ↓
subsystem
 ↓
workspace
 ↓
production gate
```

Fast feedback during iteration.

Complete evidence before merge.

---

# 58. TEST QUALITY METRICS

Track:

- branch coverage
- invariant coverage
- mutation kill rate
- failure-path coverage
- flaky rate
- real-bug regression coverage

Code coverage alone is insufficient.

---

# 59. AUTOMATIC TEST SYNTHESIS

Changed logic should generate candidate tests for:

- new branch
- changed error
- changed permission
- changed state transition
- new serialization
- new boundary value

---

# PART XIV — X3 LANG CANONICAL ARCHITECTURE

Before modifying X3 Lang, verify current repository reality.

The expected architecture has historically treated:

```text
x3-lang/compiler
+
x3-lang/vm
```

as canonical compiler/VM authority, with root-level compatibility/bridge layers integrating into it.

Do not accidentally create a second competing compiler.

If the current repository still follows that architecture, preserve it.

---

# 60. ROOT X3 INTEGRATION BRIDGE

Preserve/finish the chosen typed bridge into canonical X3 Lang.

Require:

```text
root API
 ↓
typed host bridge
 ↓
canonical compiler
 ↓
deterministic artifact
 ↓
bytecode verifier
 ↓
X3VM/Trading VM
 ↓
receipt
```

---

# 61. ARTIFACT IDENTITY

Every compiled X3 Lang artifact needs:

- source identity
- compiler version
- language version
- capability version
- bytecode version
- artifact hash

---

# 62. COMPILATION MODES

Support clear modes such as:

```text
Development
Production
Mainnet
```

Mainnet mode must reject behavior that production policy forbids.

---

# PART XV — X3 LANG TOOLCHAIN

## 63. TREE-SITTER-X3

Create/maintain an official Tree-sitter grammar for:

- syntax highlighting
- incremental parse
- editor navigation
- syntax-aware tooling

---

# 64. LOSSLESS SYNTAX MODEL

Use a lossless syntax representation such as Rowan-style architecture where appropriate.

Preserve source information needed for:

- comments
- formatting
- refactoring
- diagnostics

---

# 65. X3 ANALYZER / LSP

Build:

```text
x3-analyzer
```

Support:

- diagnostics
- autocomplete
- hover
- goto definition
- references
- rename
- workspace symbols
- semantic tokens
- code actions
- type info
- implementation navigation

---

# 66. FORMATTER

```text
x3 fmt
```

must be deterministic.

---

# 67. LINTER

```text
x3 lint
```

Detect:

- unsafe patterns
- suspicious authority
- invalid atomic assumptions
- unchecked cross-VM response
- expensive behavior
- deprecated syntax

---

# 68. DOCUMENTATION

```text
x3 doc
```

Generate:

- types
- contracts
- functions
- events
- storage
- permissions
- security properties
- cross-VM interfaces

---

# 69. PACKAGE SYSTEM

Use deterministic dependency manifests/locks.

Expected concept:

```text
x3.toml
x3.lock
```

Record dependency hashes.

---

# PART XVI — X3 LANG CORRECTNESS

## 70. REFERENCE INTERPRETER

Create:

```text
x3-interpreter
```

Keep it deliberately simple.

Correctness > speed.

---

# 71. DIFFERENTIAL EXECUTION

For generated programs:

```text
Reference Interpreter
        VS
X3VM
```

Any unexplained semantic difference is a bug.

---

# 72. STRUCTURED PROGRAM GENERATOR

Generate syntactically valid programs containing:

- types
- assets
- storage
- functions
- arithmetic
- ownership
- branches
- loops
- atomic operations
- cross-VM calls
- settlement
- guards
- fees
- deadlines

---

# 73. FUZZING STACK

Evaluate/integrate:

- cargo-fuzz/libFuzzer
- LibAFL
- AFL++

Targets:

- lexer
- parser
- type checker
- HIR
- MIR
- optimizer
- backend
- decoder
- bytecode verifier
- VM

---

# 74. PROPERTY TESTING

Use Proptest or equivalent.

Test properties including:

```text
parse(print(parse(x))) == parse(x)
```

```text
decode(encode(x)) == x
```

```text
execute(optimized(x)) == execute(unoptimized(x))
```

---

# 75. STATEFUL COMPILER/VM TESTING

Generate multi-step programs and validate state evolution.

---

# 76. MUTATION TESTING

Use cargo-mutants or equivalent to deliberately break compiler/runtime logic.

If tests continue passing after removal of security-critical checks:

```text
TEST SUITE INSUFFICIENT
```

---

# 77. MIRI

Run Miri where applicable against unsafe/compiler/runtime code.

---

# 78. KANI

Use Kani for bounded model checking of critical Rust logic where practical.

---

# 79. SMT / Z3

Use SMT solving selectively for:

- arithmetic
- impossible states
- authorization
- optimizer validation
- state constraints

---

# 80. TRANSLATION VALIDATION

Do not blindly trust `x3_opt`.

For appropriate optimizer transformations validate:

```text
MIR before ≡ MIR after
```

---

# 81. METAMORPHIC TESTING

Apply semantic-preserving transformations:

- variable renaming
- declaration reordering
- dead code insertion
- equivalent expression replacement

Program result should remain unchanged.

---

# 82. FAILURE SHRINKER

Automatically reduce fuzz failures to the smallest reproducible `.x3` program.

Every minimized real bug becomes a permanent regression.

---

# 83. COMPILE SUITES

Maintain:

```text
compile-pass
compile-fail
run-pass
run-fail
ui-diagnostics
```

---

# 84. SNAPSHOT TESTING

Snapshot:

- AST
- HIR
- MIR
- diagnostics
- bytecode

Unexpected changes require review.

---

# PART XVII — X3 BYTECODE SECURITY

## 85. INDEPENDENT BYTECODE VERIFIER

Never blindly trust compiler output.

Verify:

- valid instructions
- version
- types
- stack/register state
- control-flow targets
- capabilities
- call permissions
- resource rules
- malformed encodings

---

# 86. AUTHENTICATED BYTECODE ENVELOPE

Production bytecode needs a versioned, authenticated envelope binding the artifact to:

- compiler
- source identity
- capabilities
- chain/runtime expectations

Malformed/mismatched artifacts must fail safely.

---

# 87. DISASSEMBLER

Provide:

```text
x3 objdump
```

Show:

- instructions
- symbols
- storage
- capabilities
- cross-VM calls
- estimated execution cost

---

# 88. DEBUGGER

Implement DAP-compatible debugging where practical.

Support:

- breakpoints
- stepping
- locals
- storage
- registers
- call trace
- atomic scope
- cross-VM transitions

---

# PART XVIII — X3 LANG SECURITY SEMANTICS

Compiler/runtime should eventually catch high-risk classes before deployment.

Examples:

- invalid asset binding
- reference-before-binding
- missing authorization
- replay-sensitive operation without protection
- unchecked cross-VM result
- unsafe atomic scope
- invalid deadline handling
- costs evaluated after profit
- missing rollback semantics
- forbidden private-submission policy violations

---

# 89. TRADING CORE VERIFIER

Maintain a stateful verifier for X3 trading/atomic IR.

It should reason about:

- chain binding
- asset binding
- capability version
- gas
- fees
- slippage
- flash fees
- costs
- profit
- solvency
- deadlines
- private submission
- rollback

---

# 90. COST BEFORE PROFIT

Ensure all required costs are accounted before declaring a trade profitable.

---

# 91. TRANSACTIONAL ROLLBACK

If a trade/atomic execution fails:

intermediate state must not silently survive.

---

# PART XIX — X3 GUARDIAN

## 92. PURPOSE

X3 Guardian is the production DApp security and trust system.

It contains:

# SECURITY GATE

and

# TRUST GATE

---

# 93. DAPP STATES

Use:

```text
EXPERIMENTAL
UNDER_REVIEW
X3_VERIFIED
RESTRICTED
```

Possible operational states may additionally include:

```text
INVESTIGATING
REVIEW_REQUIRED
REVOKED
DEPRECATED
```

---

# 94. SECURITY GATE

Applicable checks include:

- reproducible build
- static analysis
- dependency analysis
- fuzzing
- stateful fuzzing
- property testing
- invariant testing
- symbolic execution
- formal verification
- mutation testing
- exploit replay
- economic simulation
- oracle simulation
- access-control testing
- proxy testing
- DoS testing
- cross-VM testing
- atomicity testing

---

# 95. TRUST GATE / ANTI-RUG ANALYZER

Detect and expose:

- hidden mint authority
- unlimited mint
- arbitrary freeze
- blacklist
- sell blocking
- honeypot behavior
- mutable extreme taxes
- owner asset drain
- liquidity drain
- hidden admin
- stealth proxy
- fake ownership renouncement
- recoverable ownership
- arbitrary external call
- oracle replacement
- treasury drain
- settlement redirection
- arbitrary balance seizure

Risky privileges do not automatically prove malicious intent.

Expose them accurately.

---

# 96. PRIVILEGE MAP

For verified applications expose:

```text
Owner
Upgrade Authority
Upgrade Timelock
Multisig Threshold
Mint Authority
Freeze Authority
Fee Authority
Oracle Authority
Treasury Authority
Emergency Controls
Liquidity Controls
```

---

# 97. HONEYPOT SIMULATION

Simulate:

- buy
- sell
- small sell
- large sell
- repeat sell
- non-owner sell
- transfer
- approval
- transferFrom
- liquidity add
- liquidity removal
- owner configuration changes
- delayed behavior

---

# 98. EXACT ARTIFACT CERTIFICATION

Certification is tied to exact:

- source
- bytecode
- dependencies
- compiler
- proxy implementation
- admin configuration
- oracle configuration
- economics
- runtime compatibility
- Guardian standard
- exploit corpus

Changing material protected state triggers reevaluation.

---

# 99. GUARDIAN MANIFEST

Create machine-readable signed manifests.

Include:

```text
application
version
source hash
bytecode hash
compiler
compiler version
dependency hash
VM
configuration
Guardian standard
Trust standard
Exploit corpus
test results
timestamp
attestation
```

---

# 100. GUARDIAN REGISTRY

Register manifest digest and certification state on-chain.

---

# 101. GUARDIAN RUNNER

Heavy testing happens off-chain.

On-chain logic should remain bounded.

---

# 102. MULTI-RUNNER ATTESTATION

High-risk DApps should be capable of requiring multiple independent runner attestations.

---

# 103. RUNTIME WATCHDOG

Monitor:

- implementation
- bytecode
- admin
- multisig
- timelock
- mint
- freeze
- tax
- fees
- oracle
- liquidity
- treasury
- settlement routing

---

# 104. RECERTIFICATION

Material changes trigger:

```text
X3 VERIFIED
     ↓
REVIEW REQUIRED
```

until relevant verification passes.

---

# 105. GUARDIAN PRIVILEGES

Guardian may control eligibility for:

- Atomic Trade Engine
- official cross-VM routing
- official asset registry
- X3 shared liquidity
- wallet discovery
- verified explorer status
- `.x3` naming
- official app directory
- ecosystem incentives
- privileged agent integrations

Unverified does not mean malicious.

---

# PART XX — GUARDIAN SECURITY EPOCHS

## 106. EXPLOIT CORPUS

Maintain a permanent defensive corpus.

Classes include:

- reentrancy
- flash-loan manipulation
- oracle manipulation
- access control
- signature replay
- nonce replay
- precision
- rounding
- accounting
- donation/inflation
- liquidation
- governance
- proxy takeover
- storage collision
- initialization
- malicious callback
- bridge forgery
- replay
- double settlement
- partial atomicity
- serialization
- DoS
- gas griefing
- account confusion
- PDA errors
- arbitrary CPI
- VM boundary bugs

---

# 107. SECURITY IMMUNE LOOP

```text
NEW EXPLOIT DISCOVERED
        ↓
ROOT CAUSE ANALYSIS
        ↓
GENERALIZED PROPERTY
        ↓
REGRESSION TEST
        ↓
NEW GUARDIAN RULE
        ↓
NEW SECURITY EPOCH
        ↓
AFFECTED APPS IDENTIFIED
        ↓
REVALIDATION
```

---

# 108. GUARDIAN TESTS GUARDIAN

Attack the certification system itself.

Test:

- forged attestations
- replayed attestations
- stale manifests
- mismatched bytecode
- compromised runner
- downgrade
- ruleset rollback
- TOCTOU
- unauthorized approval
- revoked privilege reuse

---

# PART XXI — X3-A³ / PROOFGATE

## 109. COMMIT-BOUND AUDIT EVIDENCE

Verification evidence must bind to the exact revision.

---

# 110. AUDIT STATES

Use deterministic outcomes such as:

```text
PASS
WARN
BLOCK
```

Do not convert BLOCK into PASS silently.

---

# 111. IMMUTABLE EVIDENCE

Preserve audit artifacts.

---

# 112. AUTHORIZED SUBMITTERS

Only authorized automation/actors should be able to submit critical production evidence.

---

# 113. APPEALS / OVERRIDES

Any exception system must be:

- explicit
- auditable
- time-bounded
- justified
- attributed

No invisible back door.

---

# PART XXII — DETERMINISTIC X3 SIMULATOR

## 114. BUILD/EXTEND X3-SIM

Create one deterministic simulation environment used by agents and CI.

Support:

- validator nodes
- X3VM
- EVM
- SVM
- settlement
- cross-VM routing
- network topology
- faults
- time
- restarts

---

# 115. DETERMINISTIC REPLAY

Replay bundles must preserve all relevant configuration.

Record:

- seed
- chain config
- runtime
- binaries
- topology
- directed links
- faults
- delays
- drops
- ordering
- crash events
- timers
- IDs
- state digests

---

# 116. FAULT VALIDATION

Fault definitions themselves must be validated.

Invalid fault configurations should fail loudly.

---

# 117. CRASH DRAINING

Model what happens to in-flight work when a process crashes.

Do not unrealistically preserve messages/state that would actually be lost.

---

# 118. RACE COVERAGE

Exercise races involving:

- timeout vs claim
- refund vs claim
- restart vs persistence
- message arrival vs expiration
- duplicate delivery

---

# 119. INCIDENT REPRODUCER

Desired command concept:

```text
x3 replay incident.bundle
```

The same failure should be recreated deterministically when technically possible.

---

# PART XXIII — MODEL-BASED PROTOCOL TESTING

## 120. REFERENCE STATE MACHINES

Model critical lifecycle states.

Example atomic settlement:

```text
Created
Locked
CounterpartyLocked
Claimable
Claimed
Refundable
Refunded
Finalized
```

---

# 121. LEGAL TRANSITIONS

Encode allowed transitions.

---

# 122. ILLEGAL TRANSITIONS

Explicitly forbid things like:

```text
Claimed → Refunded
Refunded → Claimed
Finalized → Locked
```

---

# 123. MODEL-BASED TEST GENERATION

Generate long sequences and compare:

```text
Reference Model
      VS
Real Runtime
```

---

# PART XXIV — CROSS-VM VERIFICATION

## 124. TEST MATRIX

Test:

```text
EVM ↔ SVM
EVM ↔ X3VM
SVM ↔ X3VM
EVM ↔ SVM ↔ X3VM
```

---

# 125. CROSS-VM PROPERTIES

Validate:

- caller identity
- source domain
- destination domain
- asset identity
- account identity
- amount
- decimal conversion
- nonce
- ordering
- replay
- finality
- timeout
- rollback
- duplicate messages
- serialization
- signatures

---

# PART XXV — ATOMIC SETTLEMENT

## 126. PRIMARY INVARIANT

```text
COMMIT ALL
OR
COMMIT NONE
```

---

# 127. FAILURE MATRIX

Test:

- success
- first-leg failure
- middle failure
- final failure
- wrong preimage
- early refund
- claim after refund
- refund after claim
- duplicate intent
- duplicate tx
- unknown intent
- stale proof
- malformed proof
- wrong chain
- wrong asset
- wrong account
- restart
- timeout
- delayed finality

---

# 128. PERSISTENT PROOF LEDGER

Where architecture requires it, proofs/receipts must survive restart and support exact inclusion evidence.

---

# PART XXVI — VALIDATOR CHAOS LAB

## 129. 7-VALIDATOR PRODUCTION-EQUIVALENT LAB

Use the validator testnet as a hostile laboratory.

Do not run only happy-path tests.

---

# 130. CHAOS ACTIONS

Automate:

- kill validator
- restart validator
- network partition
- packet loss
- latency
- jitter
- disk pressure
- CPU pressure
- memory pressure
- RPC restart
- DB restart
- peer isolation

---

# 131. BYZANTINE BEHAVIOR

Test defensive handling of:

- malformed messages
- stale messages
- replay
- equivocation scenarios
- invalid candidate data
- incorrect metadata
- malicious peer announcements

---

# 132. RECOVERY MEASUREMENTS

Measure:

- finality recovery
- peer recovery
- missed blocks
- state consistency
- atomic transactions
- database integrity
- RPC consistency

---

# PART XXVII — ADVERSARIAL TRANSACTION GENERATOR

## 133. TRAFFIC CLASSES

Generate:

- valid tx
- invalid signature
- stale nonce
- future nonce
- duplicate tx
- conflicting tx
- oversized payload
- malformed packet
- hot-account contention

---

# 134. BENCHMARK BAD TRAFFIC

Measure:

- invalid signatures rejected/sec
- stale nonces rejected/sec
- malformed packets rejected/sec
- duplicates rejected/sec
- CPU per rejection
- memory impact
- tx-pool impact

Do not benchmark only honest TPS.

---

# PART XXVIII — PERFORMANCE ENGINEERING

## 135. CONTINUOUS BENCHMARKS

Track:

- submitted TPS
- accepted TPS
- finalized TPS
- block time
- finality
- mempool
- proposer time
- signature verification
- cross-VM latency
- settlement latency
- RPC
- DB
- disk
- memory
- state growth

---

# 136. PERFORMANCE ROADMAP SUPPORT

Infrastructure should support measuring experimental work around:

- transaction pool tuning
- parallel proposer
- GPU batch signature verification
- 200ms block goals
- Flash Finality
- PoH digest
- Turbine
- Gulf Stream
- capacity/weights

Do not claim experimental mechanisms production-ready solely because benchmarks improve.

---

# 137. REGRESSION THRESHOLDS

Record baselines.

Flag significant degradation.

---

# PART XXIX — RESOURCE ACCOUNTING

## 138. OPERATION COSTS

Measure:

- CPU
- memory
- DB reads
- DB writes
- network
- disk

---

# 139. STATE GROWTH

Track:

```text
bytes/tx
permanent bytes
prunable bytes
index growth
snapshot growth
sync impact
```

---

# PART XXX — STORAGE AND DATABASE

## 140. STORAGE MIGRATION FUZZING

Generate historical/edge-case storage states.

Upgrade them.

Verify invariants.

---

# 141. DATABASE DIFFERENTIAL TESTING

Run identical workload against relevant supported configurations and compare canonical results.

---

# 142. DISK GROWTH MODEL

Measure long-duration disk expansion.

Do not extrapolate blindly from tiny testnets.

---

# 143. SNAPSHOT / STATE SYNC

Test:

- snapshot creation
- restore
- corrupt snapshot
- interrupted restore
- new-node bootstrap
- stale snapshot
- incompatible runtime

---

# 144. DESTRUCTIVE RECOVERY

Practice actual recovery from:

- lost DB
- corrupted DB
- missing node
- damaged storage
- broken upgrade

---

# PART XXXI — RUNTIME UPGRADES

## 145. UPGRADE REHEARSAL

Clone representative state.

Execute upgrade.

Run:

- invariants
- migrations
- restart
- benchmark
- state comparison

---

# 146. COMPATIBILITY MATRIX

Track supported:

```text
runtime old → runtime new
DB old → DB new
X3 bytecode old → VM new
Guardian manifest old → Guardian new
network protocol old → new
```

---

# 147. MIXED-VERSION TESTNET

Test rolling upgrades across mixed validator versions.

---

# 148. ROLLBACK ANALYSIS

For each high-impact release record:

```text
Can rollback occur?

What is irreversible?

What recovery path exists?
```

---

# PART XXXII — NETWORK PROTOCOL FUZZING

## 149. FUZZ EVERY HOSTILE DECODER

Targets:

- peer messages
- blocks
- transaction propagation
- finality
- PoH
- Turbine
- X3 packets
- RPC
- cross-VM messages

---

# 150. PACKET CORPUS

Keep:

- valid
- invalid
- truncated
- oversized
- duplicate
- stale version
- future version

---

# PART XXXIII — ARCHITECTURE ENFORCEMENT

## 151. DEPENDENCY DIRECTION RULES

Encode architecture constraints.

Example:

```text
Explorer → RPC → Runtime API → Runtime
```

Prevent forbidden shortcuts.

---

# 152. EXAMPLE FORBIDDEN EDGES

Examples:

```text
Explorer → raw consensus database
Guardian Runner → direct certification storage mutation
Cross-VM Router → arbitrary supply mutation
```

---

# PART XXXIV — SECURITY TOOLCHAIN

## 153. DEPENDENCY / CODE SCANNING

Evaluate/integrate as appropriate:

- cargo audit
- cargo deny
- cargo auditable
- Semgrep
- Trivy
- CodeQL or equivalent analysis

Do not blindly add tools merely because they exist.

Each tool must produce useful actionable evidence.

---

# 154. SUPPLY-CHAIN PROVENANCE

For critical releases generate:

- dependency inventory
- hashes
- SBOM where appropriate
- binary provenance
- build provenance

---

# PART XXXV — BUILD AND TEST SPEED

## 155. SCCACHE

Use distributed/local compilation caching where beneficial.

---

# 156. NEXTTEST

Use cargo-nextest or equivalent when it materially improves Rust test execution.

---

# 157. HARDWARE SCHEDULER

Use available X3 machines intelligently.

Classify workers for:

- high-core Rust compilation
- GPU workloads
- DB testing
- validator nodes
- long-running chaos
- fuzz workers

---

# 158. LOCAL CI FARM

Use self-hosted runners with role labels.

Potential classes:

- Rust/Substrate
- WASM
- EVM
- SVM
- X3 Lang
- GPU
- live-node
- validator
- long-running
- release

Require:

- workspace cleanup
- isolation
- concurrency control
- timeouts
- disk cleanup
- secret boundaries

---

# PART XXXVI — CI MUST FAIL CLOSED

## 159. NO-JOB CI IS NOT PASS

If a workflow expected to run but runs zero jobs:

that is a CI failure.

Detect:

- trigger mistakes
- path-filter mistakes
- branch mistakes
- missing matrices
- skipped required jobs

---

# 160. CI TIERS

## FAST

- fmt
- lint
- targeted tests

## STANDARD

- subsystem
- integration

## SECURITY

- fuzz
- mutation
- invariants
- Guardian

## RELEASE

- full workspace
- reproducibility
- live domains
- validator
- migrations
- recovery
- benchmarks
- evidence bundle

---

# PART XXXVII — REPOSITORY PARITY / RECOVERY

## 161. PRESERVE LEGITIMATE PRIOR FUNCTIONALITY

If older X3 repositories/branches contain real functionality missing from canonical `xxxstar`, compare carefully.

Restore only legitimate code.

Do not import:

- generated junk
- stale mocks
- secrets
- backups
- abandoned incompatible architecture

Require tests proving restored behavior.

---

# PART XXXVIII — RPC/API / GATEWAY

## 162. PRODUCTION API RULES

Ensure gateway/RPC layers have:

- graceful shutdown
- readiness
- health
- authentication where required
- secure CORS
- migrations
- bounded resource use
- structured errors

---

# 163. RESPONSE CONSISTENCY

RPC and runtime APIs must have version compatibility tests.

---

# PART XXXIX — X3 MCP / STRUCTURED AGENT TOOLS

## 164. EXPOSE X3 AS TOOLS

Agents should query systems through structured interfaces instead of enormous prompts.

Potential tools:

```text
repo.search
repo.symbol
repo.impact
repo.history
tests.run
tests.lookup
guardian.status
guardian.verify
sim.run
sim.replay
benchmark.run
ci.status
feature.status
```

---

# PART XL — DEVELOPMENT CONSOLE

Where an X3 coding console/IDE exists, support:

- recursive workspace tree
- side-by-side diffs
- history recovery
- clean error termination
- socket cleanup
- stream heartbeat
- isolated execution
- useful preview capabilities

Do not let UI work bypass core engineering priorities.

---

# PART XLI — AUTOMATIC ROOT-CAUSE CLUSTERING

## 165. FAILURE CLUSTERING

If 80 tests fail from one broken API:

do not create 80 separate repair tasks.

Cluster likely root causes.

---

# PART XLII — SELF-HEALING LOOP

## 166. FAILURE LOOP

```text
BUILD
 ↓
FAIL
 ↓
CLASSIFY
 ↓
SEARCH FAILURE MEMORY
 ↓
RETRIEVE CONTEXT
 ↓
CREATE MINIMAL REPRO
 ↓
PATCH
 ↓
TARGETED TEST
 ↓
WIDER TEST
```

---

# 167. STUCK AGENT ESCALATION

Use:

```text
cheap model
 ↓
more context
 ↓
specialist
 ↓
alternative approach
 ↓
strong model
 ↓
human-visible blocker
```

Do not infinite-loop.

---

# PART XLIII — SHADOW ENGINEER

## 168. INDEPENDENT EXPECTATION

Before/after a high-risk patch, a shadow agent independently predicts:

- what should change
- likely blast radius
- required invariants
- required tests

Compare prediction against actual patch.

Large unexplained disagreement triggers review.

---

# PART XLIV — INDEPENDENT REVIEW

## 169. AUTHOR != REVIEWER != VERIFIER

Separate responsibilities.

Security-critical code should receive independent adversarial review.

---

# 170. SOLUTION DIVERSITY

For extremely difficult failures, independent approaches may include:

- source diagnosis
- minimal reproducer
- model/state-machine analysis

Avoid three agents parroting the same proposed patch.

---

# PART XLV — MUTATION OF X3 ITSELF

## 171. PROTOCOL MUTATION TESTS

Intentionally create test builds that:

- remove nonce validation
- remove signature validation
- disable settlement uniqueness
- weaken asset binding
- disable replay protection
- remove authority check
- alter timeout
- remove rollback

The test ecosystem must detect these.

---

# PART XLVI — CANARY / SHADOW EXECUTION

## 172. RUN NEW LOGIC ALONGSIDE OLD

For suitable upgrades:

feed identical inputs into:

```text
Current Implementation
New Implementation
```

Compare state/output before promotion.

---

# PART XLVII — TECHNICAL DEBT MEASUREMENT

## 173. MEASURE DEBT BY COST

Raise technical-debt priority when a module repeatedly causes:

- failures
- agent confusion
- merge conflicts
- long builds
- regressions

---

# PART XLVIII — AGENT WASTE MEASUREMENT

## 174. TRACK WASTE

Measure:

- rejected tokens
- duplicate edits
- unnecessary premium-model calls
- wrong-file edits
- repeated builds
- duplicated reviews
- failed context retrieval

Optimize the system using this data.

---

# PART XLIX — TRAINING DATA FACTORY

## 175. CAPTURE VERIFIED TRAJECTORIES

Store:

```text
Task
Context
Plan
Patch
Tests
Review
Fixes
Final Accepted Commit
```

---

# 176. RAG FIRST

Do not immediately train a model on raw repository history.

Use retrieval first.

---

# 177. CURATED X3-CODER DATASET

Only verified, useful trajectories belong in the future training corpus.

Potential future result:

# X3-Coder

Do not train it until there is enough high-quality data to justify it.

---

# PART L — RELEASE EVIDENCE

## 178. RELEASE BUNDLE

Every serious release should include:

- exact commit
- source hashes
- binary hashes
- runtime hash
- chain-spec hash
- compiler versions
- dependency state
- build environment
- tests
- live integration evidence
- benchmarks
- known limitations

---

# 179. AUDIT ARTIFACT DIRECTORIES

Use stable conventions such as:

```text
audit-artifacts/
guardian-artifacts/
compiler-artifacts/
benchmark-artifacts/
release-artifacts/
incident-artifacts/
```

---

# PART LI — PRE-VALIDATOR RELEASE FREEZE

Before a major 7-validator run, freeze and record:

- exact source revision
- binary hash
- runtime WASM hash
- chain-spec hash
- genesis configuration
- validator key assignments
- test configuration

Do not change code halfway through the run and treat the evidence as one experiment.

---

# PART LII — VALIDATOR OPERATIONS

## 180. KEY CEREMONY

Test:

- generation
- storage
- rotation
- recovery
- loss
- replacement

---

# 181. VALIDATOR DEPLOYMENT

Production readiness eventually requires:

- bootnodes
- peer topology
- sentries if used
- firewalling
- systemd/service configuration
- monitoring
- alerting
- rolling upgrades
- runtime upgrades
- binary verification
- disaster recovery

---

# PART LIII — FEATURE COMPLETION SYSTEM

## 182. MACHINE-READABLE FEATURE REGISTRY

Each feature records:

```text
name
status
implementation
dependencies
tests
invariants
integration
security
benchmark
operations
documentation
evidence
```

---

# 183. EVIDENCE-WEIGHTED COMPLETION

Do not ask an agent:

```text
How complete is this?
```

Calculate it.

Example dimensions:

```text
implementation
unit verification
integration
adversarial testing
security
performance
operations
documentation
live evidence
```

---

# 184. EVIDENCE TRUST LEVELS

Weight evidence.

Example:

```text
agent assertion                 very low
unit test                       low/moderate
integration test                moderate
independent verifier            high
live multi-validator evidence   very high
```

---

# PART LIV — LAUNCH READINESS

## 185. MACHINE-CALCULATED MATRIX

Eventually produce:

```text
Consensus
Validator Stability
X3VM
EVM
SVM
Cross-VM
Atomic Settlement
Supply Invariants
X3 Lang
Guardian
RPC/API
Storage Migration
State Sync
Disaster Recovery
Performance
Security
Operations
Explorer/Indexer
Documentation
```

Each status must link to evidence.

---

# PART LV — V1 SCOPE DISCIPLINE

## 186. DO NOT REQUIRE EVERY RESEARCH FEATURE FOR V1

Separate:

```text
v1.0
v1.1
v1.2
v2
Research
```

A smaller rigorously proven mainnet is better than a giant unverified one.

---

# PART LVI — PRIORITY PRODUCTION GAPS

Continuously identify and prioritize high-risk gaps such as:

- authenticated decoders
- versioned bytecode envelopes
- source→compile→decode→execute→receipt proof
- stateful X3 Lang verifier
- capability/version enforcement
- costs-before-profit
- transactional rollback
- private-submission enforcement
- validator key lifecycle
- supply conservation
- pending-supply-zero
- relayer/attestation quorum
- snapshot/state sync
- disk growth
- destructive recovery
- real migrations

Only keep this list if the current repository still lacks them.

Verify first.

---

# PART LVII — MAIN ENGINEERING LOOP

Every autonomous task should follow:

```text
1. SCAN
2. IDENTIFY REAL GAP
3. CHECK DUPLICATE WORK
4. LOAD ARCHITECTURE
5. BUILD CONTEXT
6. IDENTIFY INVARIANTS
7. CALCULATE BLAST RADIUS
8. WRITE/LOAD SPEC
9. CHOOSE MODEL
10. CREATE WORKTREE
11. CREATE REPRO/TEST WHEN POSSIBLE
12. IMPLEMENT
13. RUN TARGETED TEST
14. FIX
15. RUN SUBSYSTEM TEST
16. FUZZ/MUTATE WHEN REQUIRED
17. RUN ADVERSARIAL TESTS
18. RUN PERFORMANCE CHECK
19. RUN SHADOW REVIEW
20. RUN INDEPENDENT REVIEW
21. RUN INDEPENDENT VERIFIER
22. PRODUCE EVIDENCE
23. MERGE ONLY IF GATES PASS
24. UPDATE REPO GRAPH
25. UPDATE FAILURE/SUCCESS MEMORY
26. UPDATE ROUTER METRICS
27. UPDATE TRAINING DATA
28. SELECT NEXT TASK
```

---

# PART LVIII — IMPLEMENTATION PHASES

Do not implement this entire prompt in one gigantic branch.

## PHASE 0 — DISCOVERY

Inventory what already exists.

Output:

```text
Existing
Partial
Missing
Broken
Duplicated
Obsolete
```

---

## PHASE 1 — X3 FORGE

Build:

- repo index
- semantic search
- code graph
- invariant registry
- context compiler
- failure memory
- feature registry

---

## PHASE 2 — ROUTER

Finish:

- provider abstraction
- OpenRouter
- DeepSeek
- Ollama/local
- fallback
- streaming
- Responses compatibility
- cost accounting
- caching
- task feedback
- routing benchmarks

---

## PHASE 3 — AGENT ORCHESTRATION

Add:

- specialist roles
- worktrees
- task DAG
- leases
- task/revision binding
- churn control
- independent verifier
- shadow engineer

---

## PHASE 4 — SMART TESTING

Add:

- targeted test selection
- mutation testing
- fuzz coordination
- test quality
- regression generation
- root-cause clustering

---

## PHASE 5 — X3 LANG INDUSTRIALIZATION

Finish:

- canonical bridge
- Tree-sitter
- LSP
- formatter
- linter
- docs
- package locking
- compile suites
- reference interpreter

---

## PHASE 6 — X3 LANG ADVANCED CORRECTNESS

Add:

- differential testing
- structured fuzzing
- Proptest
- LibAFL
- cargo-fuzz
- AFL++
- Kani
- Miri
- SMT
- translation validation
- mutation testing
- failure shrinking
- bytecode verifier
- authenticated artifacts

---

## PHASE 7 — X3 SIM

Create/finish deterministic:

- simulation
- fault system
- replay
- state digests
- incident bundles

---

## PHASE 8 — X3 GUARDIAN

Implement:

- app registry
- security gate
- trust gate
- manifest
- attestation
- certification
- privilege integration

---

## PHASE 9 — GUARDIAN ADVANCED

Implement:

- exploit corpus
- security epochs
- runtime watchdog
- recertification
- multi-runner attestations
- Guardian self-attacks

---

## PHASE 10 — PROTOCOL MODEL TESTING

Build:

- reference state machines
- atomic model
- asset model
- governance model where appropriate
- generated transition tests

---

## PHASE 11 — VALIDATOR CHAOS

Build:

- fault injection
- Byzantine tests
- traffic storms
- recovery evidence
- distributed incident recorder

---

## PHASE 12 — STORAGE / RECOVERY / MIGRATIONS

Harden:

- snapshots
- sync
- DB
- migrations
- disk growth
- destructive recovery

---

## PHASE 13 — PERFORMANCE FACTORY

Automate:

- finalized TPS
- latency
- resource use
- X3VM
- compiler
- DB
- cross-VM
- rejection throughput

---

## PHASE 14 — RELEASE SYSTEM

Create:

- commit-bound release evidence
- reproducible builds
- ProofGate
- release readiness matrix
- launch ceremony

---

## PHASE 15 — SELF-IMPROVING ENGINEERING

Use accumulated data to optimize:

- model routing
- context selection
- test selection
- workflow choice
- task decomposition
- cost per verified merge

---

# PART LIX — DEFINITION OF DONE

A feature is not DONE merely because implementation exists.

For relevant high-risk features, DONE means:

```text
Implementation
+
Correct Tests
+
Integration
+
Invariant Coverage
+
Adversarial Coverage
+
Failure Coverage
+
Performance Evidence
+
Documentation
+
Operational Evidence
+
Independent Verification
```

---

# PART LX — FINAL REPORT FORMAT

After every meaningful implementation cycle report:

## SCOPE

What task was attempted.

## BASE REVISION

Exact full commit.

## DISCOVERY

Existing/partial/missing findings.

## IMPLEMENTATION

What changed.

## FILES

Relevant modified files.

## AFFECTED INVARIANTS

Invariant IDs.

## BLAST RADIUS

Affected systems.

## TESTS

Exact commands and results.

## FUZZ / MUTATION

If applicable.

## SECURITY

Findings.

## BENCHMARKS

Before/after where applicable.

## MODEL / AGENT COST

When available.

## EVIDENCE

Hashes/artifact paths.

## REMAINING GAPS

Specific unresolved work.

## VERDICT

Use only:

```text
VERIFIED FOR THIS SCOPE
NOT VERIFIED
BLOCKED
```

Do not inflate success.

---

# PART LXI — ENDGAME PRINCIPLE

The final objective is not:

> Get AI agents to write a lot of code.

The final objective is:

> Build an engineering system where weak code has difficulty surviving.

X3 should continuously know:

```text
What are we building?

Why?

Where does the implementation live?

What depends on it?

Which invariants could break?

Has another agent already done this?

What happened the last time this failed?

Which model has historically solved this type of problem best?

What is the cheapest model capable of doing it correctly?

Which tests are required?

Did those tests actually run?

Can the test suite detect deliberate corruption?

Can an attacker break this?

Can a validator failure break this?

Can malformed network traffic break this?

Can a storage migration break this?

Did performance regress?

Did compiler semantics change?

Can another machine reproduce the result?

Is the current evidence still valid for this commit?

Can this patch safely merge?

Can this binary safely release?
```

The system should become better because every cycle produces reusable evidence.

---

# FINAL TARGET

Build X3 toward this operating model:

```text
            SPEC
             │
             ▼
       REPO INTELLIGENCE
             │
             ▼
       CONTEXT COMPILER
             │
             ▼
        SMART ROUTER
             │
             ▼
     SPECIALIST ENGINEER
             │
             ▼
           PATCH
             │
             ▼
      SHADOW ENGINEER
             │
             ▼
      BLAST-RADIUS CHECK
             │
             ▼
     TARGETED VERIFICATION
             │
             ▼
    INVARIANT / PROPERTY TEST
             │
             ▼
      FUZZ / MUTATION TEST
             │
             ▼
       ADVERSARIAL TEST
             │
             ▼
     X3 DETERMINISTIC SIM
             │
             ▼
      CHAOS / FAULT TEST
             │
             ▼
       PERFORMANCE GATE
             │
             ▼
      INDEPENDENT REVIEW
             │
             ▼
      INDEPENDENT VERIFIER
             │
             ▼
           X3-A³
             │
             ▼
       EVIDENCE BUNDLE
             │
             ▼
        VERIFIED MERGE
             │
             ▼
       CANARY / TESTNET
             │
             ▼
      PRODUCTION RELEASE
             │
             ▼
           LEARN
             │
      ┌──────┴──────┐
      ▼             ▼
 REPO MEMORY    ROUTER MEMORY
      │             │
      └──────┬──────┘
             ▼
         NEXT TASK
```

Build X3 so that:

**agents get smarter from verified history**

**agents get faster because they retrieve rather than reread**

**agents get cheaper because the router uses the least expensive capable model**

**the compiler gets harder to miscompile**

**the VM gets harder to feed invalid code**

**DApps get harder to ship with hidden dangerous behavior**

**atomic settlement gets harder to break**

**validators get tested under hostile conditions before production**

**bugs become permanent regression tests**

**exploits discovered elsewhere become X3 defenses**

**new releases carry reproducible evidence**

**completion becomes mathematical rather than subjective**

**mainnet readiness becomes a gate rather than an opinion**

and:

# NO EVIDENCE = NO PROMOTION

This rule overrides speed, convenience, and agent confidence.

Build incrementally.

Do not destroy stable architecture.

Do not duplicate canonical implementations.

Do not hide failures.

Do not weaken gates.

Do not confuse test mocks with reality.

Do not optimize cost by sacrificing correctness.

Make every failure useful.

Make every accepted patch teach the system.

Make every new exploit strengthen the ecosystem.

Make every release reproducible.

Make X3 increasingly capable of verifying itself.
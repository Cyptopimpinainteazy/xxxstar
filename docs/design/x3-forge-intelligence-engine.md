# X3 FORGE INTELLIGENCE ENGINE
## SMARTER • FASTER • CHEAPER • MORE ACCURATE • SELF-IMPROVING

X3 Forge is not merely an LLM router.

It is the engineering intelligence and orchestration layer responsible for making every coding agent:

- understand the repository faster
- receive less irrelevant context
- select the correct model automatically
- avoid repeating previous work
- reuse verified evidence
- parallelize safely
- detect bad approaches early
- write smaller and safer patches
- test intelligently
- learn which models/tools work best for each task class
- spend expensive model tokens only where they produce measurable value
- escalate difficult work automatically
- independently verify important changes
- improve its engineering strategy from accumulated evidence

The target is:

```text
MORE VERIFIED ENGINEERING
PER
TOKEN
MINUTE
GPU HOUR
CI MINUTE
HUMAN HOUR
```

Do not optimize for raw agent activity.

Optimize for:

```text
VERIFIED CORRECT WORK / COST / TIME
```

---

# 1. X3 SMART ROUTER

Build a local OpenAI-compatible routing gateway.

The existing development environment may contain providers such as:

```text
x3-router
deepseek
ollama
openrouter
OpenAI-compatible providers
local inference servers
future providers
```

Never assume configuration.

Discover existing configuration first.

Never overwrite working provider configuration without evidence and backup.

The router should expose one stable endpoint:

```text
http://127.0.0.1:<configured-port>/v1
```

Agents interact with the router rather than knowing which provider ultimately handles the task.

Example logical models:

```text
x3-auto
x3-fast
x3-code
x3-deep
x3-security
x3-review
x3-local
```

These are routing policies, not necessarily actual model names.

---

# 2. TASK CLASSIFIER

Before selecting a model, classify the task.

Possible classes:

```text
REPOSITORY_SEARCH
SIMPLE_EDIT
BOILERPLATE
DOCUMENTATION
TEST_GENERATION
RUST_IMPLEMENTATION
COMPILER_WORK
CONSENSUS
CRYPTOGRAPHY
SECURITY_ANALYSIS
FUZZING
DEBUGGING
ARCHITECTURE
PERFORMANCE
DATABASE
NETWORKING
EVM
SVM
X3VM
X3_LANG
CROSS_CHAIN
CODE_REVIEW
FAILURE_ANALYSIS
```

Estimate:

```text
complexity
risk
blast radius
context requirement
expected token requirement
verification requirement
parallelizability
```

Then route accordingly.

---

# 3. MODEL CAPABILITY REGISTRY

Maintain measured capability profiles.

Example:

```text
model_profile:

provider:
model:

rust_score:
security_score:
architecture_score:
debug_score:
repo_navigation_score:
test_generation_score:
review_score:

average_latency:
average_cost:
failure_rate:
retry_rate:
accepted_patch_rate:
verified_patch_rate:
regression_rate:
```

Do not permanently assume one model is best.

Measure it.

Update routing decisions using actual X3 engineering results.

---

# 4. CHEAP-FIRST ESCALATION

Do not send everything to the strongest/most expensive model.

Pipeline:

```text
LOCAL / CHEAP
      ↓
CAN IT SOLVE?
   ↙      ↘
 YES       NO
 ↓          ↓
VERIFY     MID MODEL
             ↓
          VERIFY
         ↙      ↘
      PASS       FAIL
       ↓          ↓
     DONE      STRONG MODEL
                  ↓
                VERIFY
```

Simple work should remain cheap.

Examples:

```text
formatting
renames
boilerplate
simple tests
documentation
mechanical refactors
```

Complex work escalates:

```text
consensus
cryptography
atomic settlement
compiler correctness
unsafe Rust
storage migrations
state divergence
complex concurrency
security findings
```

---

# 5. MODEL RACING

For difficult problems, optionally dispatch multiple independent models.

Example:

```text
PROBLEM
   │
 ┌─┼─────────────┐
 ▼ ▼             ▼
M1 M2            M3
 │ │             │
 └─┼─────────────┘
   ↓
COMPARE SOLUTIONS
   ↓
VERIFY
   ↓
SELECT EVIDENCE-BACKED RESULT
```

Use only when expected value exceeds additional compute cost.

Do not choose by eloquence.

Choose by:

```text
tests
proof
benchmarks
correctness
minimality
security
```

---

# 6. X3 REPOSITORY GRAPH

Build a persistent repository intelligence graph.

Track:

```text
workspace
crate
module
file
function
trait
impl
type
pallet
runtime API
RPC
contract
program
database
migration
test
benchmark
CI workflow
configuration
dependency
```

Relationships:

```text
CALLS
IMPLEMENTS
DEPENDS_ON
TESTED_BY
MUTATES
READS
EXPOSES
SERIALIZES
VALIDATES
FINALIZES
SETTLES
ROUTES_TO
MIGRATES
```

Agents should query this graph rather than repeatedly searching the entire repository.

---

# 7. X3 CONTEXT COMPILER

This is critical.

Before calling an expensive model:

```text
USER TASK
   ↓
REPOSITORY GRAPH
   ↓
DEPENDENCY ANALYSIS
   ↓
FAILURE MEMORY
   ↓
TEST HISTORY
   ↓
SECURITY INVARIANTS
   ↓
CONTEXT COMPILER
   ↓
MINIMAL HIGH-VALUE CONTEXT
   ↓
MODEL
```

Do NOT blindly send 100 files.

Determine:

```text
files required
symbols required
interfaces required
tests required
previous failures
architectural rules
security assumptions
historical bug analogues
```

The objective:

```text
LESS CONTEXT
+
BETTER CONTEXT
=
BETTER CODING
```

---

# 8. CONTEXT QUALITY SCORING

Measure whether supplied context actually helped.

Record:

```text
context tokens
files included
files actually modified
symbols referenced
extra searches required
model confusion/retries
final verification outcome
```

Use this to improve future context compilation.

---

# 9. SEMANTIC CODE INDEX

Maintain embeddings/indexes for:

```text
source
tests
documentation
architecture
failures
audit findings
historical bugs
commit explanations
```

Queries should support concepts rather than exact filenames.

Example:

```text
"where is duplicate settlement prevented?"
```

should retrieve:

```text
settlement engine
intent state
proof verifier
cross-VM router
relevant tests
relevant invariant
historical failures
```

---

# 10. BLAST-RADIUS ENGINE

Before changing code:

```text
PROPOSED CHANGE
      ↓
DIRECT DEPENDENCIES
      ↓
TRANSITIVE DEPENDENCIES
      ↓
TESTS
      ↓
INVARIANTS
      ↓
PUBLIC APIs
      ↓
STORAGE
      ↓
CONSENSUS
      ↓
SECURITY
```

Output:

```text
LOW
MEDIUM
HIGH
CONSENSUS CRITICAL
```

Use risk to determine verification depth.

---

# 11. SMART TEST SELECTION

Do not run the entire universe after every tiny change.

Map:

```text
CODE CHANGE
→ AFFECTED COMPONENTS
→ RELEVANT TESTS
→ RELEVANT FUZZ TARGETS
→ RELEVANT INVARIANTS
→ HISTORICAL FAILURE CLASSES
```

Then run those first.

If they pass:

```text
targeted
→ subsystem
→ integration
→ release gate
```

This makes development substantially faster.

---

# 12. TEST IMPACT DATABASE

Learn from previous changes.

Store:

```text
file X changed
→ tests A/B/C failed historically

crate Y changed
→ fuzz target D found regression

function Z changed
→ invariant E affected
```

Use historical evidence to predict which tests matter.

---

# 13. BUILD ACCELERATION

Implement shared compilation infrastructure.

Evaluate/use:

```text
sccache
shared Cargo target strategy
incremental compilation where appropriate
dependency caching
prebuilt tool containers
workspace partitioning
parallel compilation
distributed workers
```

Measure before/after.

Do not optimize blindly.

Record:

```text
cold build
warm build
incremental build
test compile
runtime build
WASM build
```

---

# 14. SPECULATIVE PRECOMPUTATION

When agents are working on likely changes, prepare useful work ahead of time when inexpensive.

Examples:

```text
compile likely affected crates
warm dependencies
prepare test environment
load relevant Failure Genome entries
prepare fuzz corpus
start local reference nodes
```

Do not waste substantial compute speculating without evidence.

---

# 15. RESULT CACHE

Cache deterministic results keyed by:

```text
source hash
dependency hash
configuration hash
tool version
compiler version
test inputs
```

If nothing relevant changed:

REUSE VERIFIED RESULT.

Never rerun an identical expensive test merely because another agent asks.

---

# 16. AGENT CHURN CONTROL

Before repeating:

```text
build
test
review
scan
benchmark
fuzz campaign
```

check whether valid matching evidence already exists.

If:

```text
same source
same inputs
same environment
same toolchain
```

reuse it.

If relevant inputs changed:

invalidate only affected evidence.

---

# 17. FAILURE MEMORY

Every failure becomes structured knowledge.

Store:

```text
fingerprint
error
component
commit
trigger
root cause
fix
regression
invariant
historical analogue
successful agent/model
failed approaches
```

Before debugging:

SEARCH FAILURE MEMORY.

Avoid rediscovering known problems.

---

# 18. SUCCESS MEMORY

Also record what worked.

Example:

```text
problem:
Substrate runtime API trait resolution

successful fix:
...

verification:
...

model:
...

context package:
...
```

Future similar tasks receive the successful strategy.

---

# 19. DEAD-END MEMORY

Record approaches that failed.

Agents must not repeatedly attempt:

```text
known broken workaround
known incompatible dependency
failed architecture
invalid build command
obsolete configuration
```

unless relevant conditions changed.

---

# 20. SELF-IMPROVING ROUTING

After each verified task calculate:

```text
TASK CLASS
MODEL
TOKENS
LATENCY
COST
ATTEMPTS
TEST RESULT
REVIEW RESULT
REGRESSION RESULT
```

Use these measurements to update routing.

Example:

```text
Model A:
excellent Rust
weak consensus reasoning

Model B:
expensive
excellent security review

Local Model C:
excellent mechanical edits
```

Route accordingly.

---

# 21. VERIFIED-PATCH RATE

Primary model metric:

```text
VERIFIED PATCHES
----------------
TOTAL ATTEMPTS
```

Secondary metrics:

```text
cost / verified patch
time / verified patch
tokens / verified patch
regression rate
review rejection rate
```

Do not optimize for benchmark scores alone.

---

# 22. SPECIALIST AGENTS

Create specialized engineering profiles:

```text
RUST
SUBSTRATE
CONSENSUS
EVM
SVM
X3VM
X3-LANG
ATOMIC
STORAGE
NETWORK
RPC
DATABASE
PERFORMANCE
SECURITY
FUZZING
FORMAL
OPS
EXPLORER
RELEASE
```

Each profile has:

```text
preferred models
tools
context strategy
verification requirements
owned components
```

---

# 23. PARALLEL AGENT DAG

Represent work as:

```text
TASK GRAPH

A ──► C ──► F
B ──► D ──► F
     E ──► F
```

Run A/B/E concurrently if independent.

Never parallelize work that creates unsafe overlapping writes.

---

# 24. OWNERSHIP ZONES

Lock logical components while agents modify them.

Example:

```text
Agent A
owns settlement engine

Agent B
owns RPC

Agent C
owns EVM tests
```

Agents may read everything.

Writes require ownership coordination.

Prevent merge-conflict factories.

---

# 25. ISOLATED WORKTREES

Each implementation agent receives an isolated Git worktree/branch.

Require:

```text
clean base
defined scope
bounded changes
tests
evidence
```

Integration happens after verification.

---

# 26. PATCH MINIMIZATION

Agents should prefer:

```text
smallest correct change
```

over:

```text
massive rewrite
```

unless architectural evidence requires redesign.

Smaller patches:

```text
review faster
test faster
merge easier
regress less
```

---

# 27. CODE QUALITY ENGINE

Every important patch should be evaluated for:

```text
correctness
clarity
complexity
duplication
error handling
panic paths
unsafe usage
allocation
locking
blocking
serialization
security assumptions
testability
observability
```

Do not accept “works” as the entire quality bar.

---

# 28. SECURITY-AWARE CODING

Before modifying consensus/security-critical code, inject relevant invariants into the coding context.

Example:

```text
SETTLEMENT CHANGE

MUST PRESERVE:

claim XOR refund
single settlement
replay protection
domain binding
asset conservation
authorized mutation
```

The model should know what it must not break BEFORE writing code.

---

# 29. HISTORICAL FAILURE-AWARE CODING

Query the Blockchain Failure Genome before important implementation work.

Example:

```text
building proof verifier
```

Retrieve historical classes involving:

```text
proof validation
signature verification
domain confusion
replay
malformed encoding
bridge verification
```

Provide these lessons to the coding agent.

This makes Forge preventative rather than reactive.

---

# 30. PLAN CRITIC

For high-risk work:

```text
IMPLEMENTER PLAN
      ↓
CRITIC AGENT
      ↓
missing assumptions?
security problems?
simpler solution?
existing implementation?
upstream project?
      ↓
REVISED PLAN
```

Do this before expensive implementation.

---

# 31. UPSTREAM-FIRST ENGINE

Before creating substantial generic infrastructure, automatically search existing dependencies/repository knowledge for reusable implementations.

Classify candidate:

```text
DEPENDENCY
ADAPTER
TEST ORACLE
REFERENCE
REJECT
```

Do not reinvent mature infrastructure without justification.

---

# 32. CODE REVIEW SWARM

For critical changes use multiple review perspectives:

```text
CORRECTNESS REVIEWER
SECURITY REVIEWER
PERFORMANCE REVIEWER
ARCHITECTURE REVIEWER
```

Then deduplicate findings.

Do not waste four agents saying the same thing.

---

# 33. ADVERSARIAL REVIEW

For security-critical patches assign an attacker:

```text
"Assume this implementation is wrong.
Find a sequence that violates its invariants."
```

Then assign independent verifier.

---

# 34. TEST GENERATION LOOP

When code changes:

```text
CHANGE
 ↓
GENERATE TESTS
 ↓
RUN
 ↓
FIND FAILURE
 ↓
MINIMIZE
 ↓
PATCH
 ↓
GENERATE VARIANTS
 ↓
RUN AGAIN
```

Tests should evolve with implementation.

---

# 35. FUZZ CORPUS LEARNING

Every interesting input becomes permanent.

Store:

```text
crashes
near-boundary inputs
rare paths
historical exploit inputs
mutation survivors
differential mismatches
```

Seed future fuzzing campaigns from them.

---

# 36. PERFORMANCE REGRESSION MEMORY

Store baseline performance per commit/release.

Track:

```text
TPS
finality
CPU
RAM
disk
network
compile time
binary size
WASM size
RPC latency
```

Flag statistically meaningful regressions.

Do not optimize from anecdotes.

---

# 37. SMART BENCHMARK SELECTION

Run microbenchmarks for local changes first.

Run expensive distributed benchmarks only when blast radius justifies them.

---

# 38. AUTO-BISECT

When a regression appears:

```text
KNOWN GOOD
    ↕
COMMITS
    ↕
KNOWN BAD
```

automatically bisect where safe.

Run the minimal reproducer at each step.

Return offending commit.

---

# 39. ROOT-CAUSE BEFORE PATCH

Agents must distinguish:

```text
SYMPTOM
vs
ROOT CAUSE
```

Do not paper over failures with:

```text
sleep
retry forever
ignore error
increase timeout
disable test
```

unless evidence establishes that behavior is correct.

---

# 40. NO SILENT TEST DELETION

An agent may not make CI green by:

```text
deleting test
ignoring test
loosening assertion
removing invariant
increasing timeout indefinitely
```

without explicit evidence and justification.

---

# 41. STOP CONDITIONS

Agents must stop when:

```text
requirements conflict
security invariant cannot be preserved
architecture decision required
credentials unavailable
external dependency unavailable
evidence contradicts requested design
```

Return the blocker rather than fabricate progress.

---

# 42. DEVELOPMENT DASHBOARD

Add Forge intelligence to the GUI.

Display:

```text
ACTIVE AGENTS
TASK DAG
WORKTREE
MODEL
TOKENS
COST
LATENCY
CURRENT TEST
BUILD CACHE HIT RATE
CONTEXT CACHE HIT RATE
VERIFIED PATCH RATE
FAILED ATTEMPTS
OPEN FINDINGS
```

Also display:

```text
cost saved by caching
tests avoided through valid evidence reuse
time saved through parallel execution
local-vs-cloud usage
```

---

# 43. ROUTER CONTROL PANEL

GUI controls:

```text
AUTO ROUTING
LOCAL-FIRST
COST CAP
MAX PARALLEL AGENTS
MODEL FALLBACKS
TASK→MODEL RULES
PROVIDER HEALTH
TOKEN BUDGET
DAILY BUDGET
```

Allow manual override.

---

# 44. PROVIDER HEALTH

Continuously track:

```text
reachable
latency
rate limits
errors
context capacity
recent success
```

If provider fails:

```text
retry according to bounded policy
↓
fallback
↓
preserve task state
```

Never let a single dead API stop the engineering pipeline.

---

# 45. ROUTER OBSERVABILITY

Log routing decisions:

```text
task
classification
selected model
reason
fallbacks
tokens
cost
latency
verification result
```

This allows us to determine whether the router actually improves engineering.

---

# 46. LOCAL COMPUTE SCHEDULER

Use available machines intelligently.

Workers advertise:

```text
CPU
RAM
GPU
disk
network
installed tools
current load
```

Scheduler assigns:

```text
builds
fuzzing
benchmarks
local inference
validator simulation
formal verification
```

Do not send GPU workloads to CPU-only nodes when an appropriate GPU worker is available.

---

# 47. FUZZ FARM

Distributed fuzz workers share:

```text
seed corpus
interesting inputs
coverage discoveries
crashes
```

Deduplicate crashes centrally.

Allow:

```text
cargo-fuzz
AFL++
Echidna
Medusa
other adapters
```

to contribute findings to the same evidence system.

---

# 48. BUILD FARM

Allow distributed build/test workers.

Use exact toolchain containers/environments where necessary.

Evidence must identify which worker produced each result.

---

# 49. PRIORITY SCHEDULER

Prioritize:

```text
P0 security
P0 broken build
P0 consensus
P0 state divergence

before

feature work
before
UI polish
```

Agents must not spend expensive compute polishing dashboards while consensus-critical failures remain unresolved.

---

# 50. SMART RETRIES

Retry only when failure appears transient.

Examples:

```text
network timeout
provider unavailable
temporary CI infrastructure failure
```

Do not repeatedly retry deterministic compiler/test failures.

Route those to debugging.

---

# 51. CONTINUOUS LEARNING WITHOUT UNSAFE SELF-MODIFICATION

Forge may automatically improve:

```text
routing weights
context selection
cache policy
test selection
agent assignment
```

based on evidence.

Forge may NOT silently rewrite security policy, invariants, production gates, or trust rules.

Changes to those require review.

---

# 52. X3 FORGE + AUDIT KING SEPARATION

Critical rule:

```text
FORGE
BUILDS

AUDIT KING
ATTACKS
```

Do not let Forge mark its own work secure.

Audit King consumes the resulting artifact independently.

Pipeline:

```text
USER GOAL
   ↓
X3 FORGE
   ↓
SMART ROUTER
   ↓
CONTEXT COMPILER
   ↓
SPECIALIST AGENTS
   ↓
IMPLEMENTATION
   ↓
TARGETED TESTS
   ↓
AUDIT KING
   ↓
FAILURE GENOME
   ↓
MUTATION
   ↓
DIFFERENTIAL
   ↓
INDEPENDENT VERIFIER
   ↓
EVIDENCE
   ↓
GUARDIAN
   ↓
MERGE
```

---

# 53. ULTIMATE ENGINEERING LOOP

The complete development system should become:

```text
                 USER / ROADMAP
                       │
                       ▼
                TASK DECOMPOSER
                       │
                       ▼
                REPOSITORY GRAPH
                       │
              ┌────────┴─────────┐
              ▼                  ▼
        FAILURE MEMORY      FAILURE GENOME
              │                  │
              └────────┬─────────┘
                       ▼
                CONTEXT COMPILER
                       │
                       ▼
                  SMART ROUTER
                       │
       ┌───────────────┼───────────────┐
       ▼               ▼               ▼
   LOCAL AI         MID MODEL       DEEP MODEL
       │               │               │
       └───────────────┼───────────────┘
                       ▼
               SPECIALIST AGENTS
                       │
                       ▼
                 CODE / PATCH
                       │
                       ▼
               SMART TEST SELECTOR
                       │
                       ▼
               TARGETED VERIFICATION
                       │
                       ▼
                   AUDIT KING
                       │
         ┌─────────────┼─────────────┐
         ▼             ▼             ▼
      FUZZING       MUTATION      DIFFERENTIAL
         │             │             │
         └─────────────┼─────────────┘
                       ▼
              ADVERSARIAL REVIEW
                       │
                       ▼
             INDEPENDENT VERIFIER
                       │
                       ▼
                EVIDENCE BUNDLE
                       │
                       ▼
                    GUARDIAN
                       │
                       ▼
                     MERGE
                       │
                       ▼
                 FAILURE/SUCCESS
                     MEMORY
                       │
                       └────► improves next task
```

---

# 54. PERFORMANCE TARGETS FOR FORGE ITSELF

Benchmark Forge.

Measure baseline development workflow before optimization.

Then measure:

```text
time-to-first-correct-patch
time-to-verified-patch
tokens-per-verified-patch
cost-per-verified-patch
build time
test time
duplicate work
context tokens
failed attempts
regression rate
human interventions
```

Our goal is not to say:

```text
"Forge is 10x faster."
```

Our goal is to produce evidence showing exactly how much faster it actually becomes.

---

# 55. IMMEDIATE ROUTER MISSION

Before changing the existing router:

1. Locate current router implementation.
2. Locate `~/.codex/config.toml` and relevant project configuration without exposing secrets.
3. Determine current provider configuration.
4. Determine how `x3-auto` currently resolves.
5. Identify DeepSeek integration.
6. Identify Ollama/local endpoint integration.
7. Identify fallback behavior.
8. Identify `/v1/responses` compatibility.
9. Identify `/v1/chat/completions` compatibility where required.
10. Identify provider health checks.
11. Identify model catalog.
12. Identify reasoning-effort handling.
13. Identify timeout/retry behavior.
14. Identify current logging.
15. Identify existing caching.
16. Identify existing routing metrics.
17. Run tests before modifying anything.
18. Produce a router gap analysis.
19. Preserve rollback configuration.
20. Implement improvements incrementally.

Do not destroy the working DeepSeek configuration while upgrading the router.

---

# 56. FIRST ROUTER RELEASE

Minimum useful release:

```text
x3-router v1

✓ OpenAI-compatible endpoint
✓ DeepSeek provider
✓ Ollama provider
✓ configurable external providers
✓ x3-auto
✓ task classification
✓ provider health
✓ bounded retries
✓ fallback
✓ logging
✓ token accounting
✓ latency accounting
✓ model capability registry
✓ context compiler integration
✓ failure memory integration
```

Then add:

```text
v2
semantic caching
model racing
adaptive routing
cost optimization
test-aware routing
historical performance learning
distributed local inference
```

---

# 57. GOLDEN RULE

Every optimization must answer:

```text
DID THIS MAKE VERIFIED ENGINEERING
FASTER,
CHEAPER,
OR MORE CORRECT?
```

If not, remove it.

Do not build complexity for its own sake.

The final objective is not merely an autonomous coding system.

The objective is:

> **An engineering system that understands X3, chooses the right intelligence for each problem, remembers what has already been learned, minimizes wasted work, uses the available hardware efficiently, independently attacks its own output, and produces evidence before declaring anything complete.**

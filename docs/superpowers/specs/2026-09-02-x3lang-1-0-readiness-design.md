# X3Lang 1.0 Readiness Design

Date: 2026-09-02
Status: Proposed architecture, approved at the high level
Scope: X3Lang compiler, VM contract, conformance, determinism, gas accounting, and developer tooling

## 1. Objective

Bring the existing X3Lang implementation from an experimental/custom language stack to a defensible X3Lang 1.0 readiness baseline without rewriting the compiler unnecessarily.

The first milestone is **Core Compiler Hardening**. The work prioritizes correctness, determinism, verifiability, and executable conformance over adding new syntax or broad new features.

The target source-to-execution path is:

```text
.x3 source
    |
    v
Lexer / Parser
    |
    v
AST / Semantic Analysis
    |
    v
IR / Lowering
    |
    v
IR Verification
    |
    v
Register Allocation
    |
    v
Bytecode Emission
    |
    v
Bytecode Verification
    |
    v
X3 VM
    |
    v
Deterministic state transition + gas result
```

## 2. Existing Architecture to Preserve

The current X3Lang workspace already separates major responsibilities into workspace crates and compiler/VM components. The hardening plan preserves this structure and strengthens the contracts between stages.

Current pipeline responsibilities include:

- lexer/tokenization;
- parser and source diagnostics;
- AST representation;
- semantic analysis;
- IR construction;
- lowering;
- register allocation;
- bytecode emission;
- VM verification and execution;
- CLI/tooling foundations;
- E2E compiler tests.

The design explicitly avoids a wholesale parser/compiler rewrite as a first step. Existing components should be decomposed or refactored only where required to establish clear interfaces, testability, and invariants.

## 3. Architectural Principle: One Canonical Execution Contract

Compiler and VM behavior must not depend on separate handwritten interpretations of the same execution rules.

X3Lang 1.0 should establish canonical definitions for at least:

- opcodes and encoding;
- operand forms;
- opcode versioning;
- gas/cost rules;
- bytecode format/version;
- ABI-visible primitive types;
- VM host-call identifiers;
- verifier constraints;
- deterministic serialization rules.

Where possible, compiler emitter, verifier, executor, disassembler, and test tooling should consume generated or shared definitions from the same canonical source.

This contract is consensus-sensitive. Any incompatible change must require an explicit version bump rather than silently changing semantics.

## 4. Core Compiler Hardening Milestone

### 4.1 Lexer and parser conformance

Create a canonical conformance suite under a dedicated test hierarchy, for example:

```text
x3-lang/tests/conformance/
  valid/
    arithmetic/
    functions/
    control_flow/
    transfer/
    atomic_swap/
    bridge/
  invalid/
    syntax/
    undefined_symbol/
    type_mismatch/
    invalid_swap/
    invalid_bridge/
    numeric_conversion/
```

Each case should define the expected result in machine-checkable form. Depending on the test, this may include:

- successful parse;
- expected AST properties;
- expected error code;
- expected source span;
- expected semantic rejection;
- expected IR properties;
- expected bytecode properties.

Malformed source must never cause an uncontrolled panic. Invalid source must produce a structured diagnostic.

### 4.2 Stable diagnostics

Introduce stable diagnostic identifiers rather than relying only on human-readable strings.

Example convention:

```text
X3E0001 unexpected-token
X3E0101 undefined-symbol
X3E0201 incompatible-types
X3E0301 invalid-numeric-coercion
X3E0401 invalid-cross-chain-route
X3E0501 unsafe-bytecode
```

A diagnostic should carry:

- stable code;
- severity;
- message;
- primary span;
- optional secondary spans;
- optional remediation/help text.

Human-facing wording may improve over time, but the diagnostic code and semantic meaning should remain stable within the same language major version.

### 4.3 Semantic/type validation

The semantic layer becomes an explicit gate before lowering.

The X3Lang 1.0 baseline should prove at minimum:

- symbol resolution correctness;
- function/argument arity checks;
- valid return behavior;
- assignment compatibility;
- explicit numeric coercion policy;
- overflow-sensitive semantics where applicable;
- valid chain/asset/bridge operation shapes;
- preservation of safety-critical intent fields.

Finance- and chain-aware types can be expanded later, but the semantic infrastructure should be designed so types such as `Amount<USDC>` or `Address<Ethereum>` can be added without replacing the compiler architecture.

## 5. IR Contract and Verification

The IR must become an explicitly verified compiler boundary.

Add an IR verifier that runs after lowering and before register allocation/emission.

The verifier should reject at least:

- references to nonexistent values/register abstractions;
- invalid or duplicate block identifiers;
- branch/jump targets that do not exist;
- blocks with invalid termination;
- malformed control-flow edges;
- incompatible operand/result types;
- invalid host-call signatures;
- impossible or missing safety attributes;
- malformed atomic-operation regions;
- malformed swap or bridge intent data.

Safety-critical intent data must survive parse -> AST -> semantic representation -> IR -> bytecode without being silently dropped.

Examples include:

- swap minimum output;
- slippage policy;
- bridge route and destination;
- timeout/deadline;
- refund policy;
- receiver;
- nonce/replay metadata;
- finality requirements.

The verifier is not an optimizer. Its role is to reject malformed compiler output and define a trustworthy boundary between front-end/lowering logic and executable generation.

## 6. Bytecode Verification

The VM must not execute arbitrary bytecode simply because it can be decoded.

A bytecode verifier should validate:

- file/header/version integrity;
- checksum/integrity fields where applicable;
- opcode validity;
- operand width and encoding;
- register/index bounds;
- branch target validity;
- call/return structure;
- host-call authorization classes;
- resource/gas representability;
- malformed or unreachable structural states that could compromise execution safety.

Verification failure must fail closed.

## 7. Determinism Requirements

X3Lang 1.0 execution is deterministic by contract.

For identical:

- source/bytecode;
- input;
- initial state;
- VM version;
- host-environment inputs that are explicitly part of the transaction context;

the system must produce identical:

- bytecode for deterministic builds;
- VM result;
- state transition;
- error category;
- gas/cost consumption.

Sources of nondeterminism must either be prohibited in consensus-sensitive execution or explicitly injected as transaction/context data.

Examples that must not leak uncontrolled nondeterminism into VM execution include:

- wall-clock time;
- random OS entropy;
- thread scheduling;
- unordered map iteration where ordering changes execution;
- local filesystem state;
- ambient environment variables;
- live RPC responses not committed as execution input.

## 8. Gas and Cost Model

Gas accounting should be tied to the canonical opcode/host-call contract.

The first production-quality model should define:

- fixed cost for deterministic primitive instructions;
- size-dependent cost for memory/hash/serialization operations;
- storage read/write costs;
- host-call base costs;
- payload-dependent host-call costs;
- limits for memory, call depth, and instruction count where applicable.

The same operation under the same VM version and inputs must consume the same gas.

The compiler may estimate gas, but the VM is authoritative for execution accounting.

Cost-table changes that affect consensus behavior require explicit versioning.

## 9. Atomicity and Rollback

Atomic operations are a first-class correctness property.

For any operation designated atomic, failure must satisfy:

```text
state_after_failure == state_before_operation
```

unless a specifically documented non-rollback side effect is part of the execution contract.

Tests should cover failures at multiple points in an atomic sequence, including host-call rejection, insufficient output, timeout/finality failure, and out-of-gas behavior.

## 10. Property, Fuzz, and Differential Testing

### Parser fuzzing

Generated or mutated source must result in either:

- a valid parse/semantic result; or
- a controlled structured diagnostic.

It must not produce uncontrolled panics, memory-unsafety symptoms, or hangs.

### Compiler determinism

Repeated compilation of the same source and compiler configuration should produce byte-for-byte identical canonical output where debug timestamps/paths are excluded from the format.

### Semantic preservation

For compiler transformations that claim to preserve behavior:

```text
execute(before) == execute(after)
```

for generated valid programs within the supported subset.

### VM determinism

Repeated execution from identical state/context must produce identical result, state root/state snapshot, and gas.

### Rollback properties

Generated failing atomic programs must leave state unchanged according to the atomicity contract.

### Differential checks

Where two independent implementations or execution paths exist, compare their observable results. Examples may include interpreter-vs-optimized execution or encoder/decoder round trips.

## 11. X3-Native Type-System Direction

Do not make the advanced domain type system a blocker for the first hardening milestone.

However, semantic/type infrastructure should support future X3-native types such as:

```text
Amount<USDC>
Amount<ETH>
Address<Ethereum>
Address<Solana>
Chain<Ethereum>
Deadline
Slippage
Gas
Nonce
```

Long-term objectives include catching cross-chain and finance-specific mistakes at compile time rather than relying on runtime rejection.

Examples:

- mixing incompatible asset denominations;
- passing a Solana address to an Ethereum-only host call;
- using a bridge destination incompatible with the selected route;
- constructing an atomic swap without a minimum-output policy when policy requires one.

## 12. Developer Experience

Developer tooling follows core correctness rather than preceding it.

The desired canonical CLI surface is:

```text
x3 new
x3 check
x3 build
x3 test
x3 run
x3 fmt
x3 inspect-ir
x3 inspect-bytecode
```

Subsequent tooling can include:

- LSP diagnostics;
- completion;
- go-to-definition;
- hover/type information;
- formatter;
- linter;
- REPL;
- documentation generator;
- package tooling.

Tooling should call the same compiler APIs and diagnostic infrastructure as the production compiler rather than reimplementing parsing or semantics.

## 13. Versioning

X3Lang should distinguish at least:

- language/source version;
- bytecode version;
- VM execution version;
- canonical opcode/cost-table version.

A compiler must make target compatibility explicit.

The VM must reject unsupported bytecode versions rather than guessing compatibility.

## 14. Security Model

The compiler must be treated as untrusted input processing software, and emitted bytecode must still be verified by the VM.

Security requirements include:

- no trust in source correctness;
- no trust in compiler-generated bytecode at the VM boundary;
- bounded parsing/compilation for hostile inputs where feasible;
- explicit resource ceilings during execution;
- fail-closed verifier behavior;
- authorization checks for privileged host calls;
- no silent fallback from production backends to dry/mock execution;
- replay/nonce/finality semantics defined for cross-chain operations.

## 15. CI and Release Gates

A change should not qualify for X3Lang 1.0 readiness merely because it compiles.

Required gates should include:

1. formatting;
2. linting;
3. unit tests;
4. conformance tests;
5. compiler E2E tests;
6. VM E2E tests;
7. deterministic-build checks;
8. deterministic-execution checks;
9. IR verifier tests;
10. bytecode verifier tests;
11. gas determinism tests;
12. property/fuzz tests at an agreed bounded duration;
13. rollback/atomicity tests;
14. security-focused malformed bytecode/source tests.

CI infrastructure failure is distinct from a failing test. X3Lang cannot be called release-ready until these gates execute successfully in trusted CI.

## 16. Milestone Sequence

### Milestone 1: Core Compiler Hardening

- stable diagnostics;
- conformance fixture structure;
- parser/semantic conformance coverage;
- explicit numeric-coercion tests;
- IR verifier;
- safety-critical intent preservation tests.

### Milestone 2: VM/Bytecode Safety

- complete bytecode structural verifier;
- canonical execution-contract integration;
- malformed-bytecode adversarial tests;
- version compatibility checks.

### Milestone 3: Deterministic Gas + Runtime Safety

- canonical gas table;
- deterministic gas tests;
- resource limits;
- atomic rollback tests;
- replay/nonce/finality runtime-path validation.

### Milestone 4: Property/Fuzz Readiness

- parser fuzz/property suite;
- compiler determinism suite;
- semantic-preservation tests;
- VM determinism tests;
- rollback properties.

### Milestone 5: Developer Tooling

- canonical CLI cleanup;
- formatter/linter baseline;
- inspect IR/bytecode commands;
- LSP integration using shared compiler diagnostics;
- documentation/examples refresh.

## 17. X3Lang 1.0 Definition of Done

X3Lang 1.0 readiness requires evidence for all of the following:

- source tree and dependencies are complete and reproducibly buildable;
- parser accepts the documented valid grammar and rejects documented invalid grammar;
- diagnostics have stable codes and tested spans;
- semantic/type checks are documented and tested;
- numeric conversion/coercion policy is executable as tests;
- IR verification rejects malformed compiler states;
- safety-critical intent fields survive the full compilation pipeline;
- emitted bytecode is versioned and structurally verified before execution;
- VM behavior is deterministic under the defined execution context;
- gas accounting is deterministic and tied to the canonical execution contract;
- atomic failure semantics are tested for rollback safety;
- parser/compiler/VM property or fuzz testing is part of the gate;
- `.x3 -> compiler -> bytecode -> verifier -> VM -> state` E2E tests pass;
- developer commands use production compiler APIs;
- CI runs the full readiness gate successfully;
- readiness documentation is regenerated from current evidence rather than historical assumptions.

## 18. Explicit Non-Goals for Milestone 1

The first hardening milestone does not require:

- a complete package manager;
- a polished LSP;
- a full advanced chain-aware generic type system;
- EVM and SVM as equal first-class compiler backends;
- broad new syntax additions;
- optimizer sophistication beyond what is needed for correctness;
- rewriting stable compiler components solely for aesthetics.

These can be addressed after the correctness and execution contracts are reliable.

## 19. Key Implementation Risks

### Compiler/VM contract drift

Mitigation: canonical shared definitions and cross-component compatibility tests.

### Existing tests prove examples but not language rules

Mitigation: conformance fixtures and stable diagnostic expectations.

### Safety attributes lost during lowering

Mitigation: explicit IR fields, verifier rules, and source-to-bytecode preservation tests.

### Gas model becomes implementation-specific

Mitigation: versioned canonical cost contract and deterministic gas tests.

### Advanced types cause scope explosion

Mitigation: harden semantic infrastructure now; add domain-aware types incrementally after Milestone 1.

### CI cannot provide trustworthy release evidence

Mitigation: treat CI execution health as a release gate separate from local correctness. Do not claim X3Lang 1.0 readiness until required jobs actually execute and pass.

## 20. Recommended First Implementation Slice

The first implementation plan should target a bounded vertical slice:

1. define stable diagnostic representation and error-code convention;
2. add conformance-test harness and initial valid/invalid fixtures;
3. encode numeric-coercion policy as tests;
4. implement or harden IR verifier API;
5. add intent-preservation tests for swap/bridge safety fields;
6. expose `check`/compiler API behavior needed by tests;
7. run workspace tests and targeted X3Lang gates;
8. document remaining failures as evidence, not assumptions.

This slice creates a reliable foundation for later bytecode, gas, fuzzing, and developer-experience work.
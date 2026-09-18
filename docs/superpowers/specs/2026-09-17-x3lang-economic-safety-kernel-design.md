# X3Lang Economic Safety Kernel Design

Status: Approved design for implementation planning  
Date: 2026-09-17  
Target branch: `codex/x3-trading-core-v1-hardening`

## Purpose

The Economic Safety Kernel makes economic correctness an enforceable property of canonical X3Lang artifacts and `TradingVm` execution. It converts the current atomic trading path into a fail-closed prove-then-execute pipeline:

1. bind a compiled economic policy;
2. bind an execution plan to a committed state snapshot;
3. deterministically simulate and account for all value movements;
4. prove economic and MEV limits before execution;
5. reject if the execution state no longer matches the proven snapshot;
6. execute through the existing atomic host transaction;
7. verify realized economics and conservation;
8. commit and issue a signed, tamper-evident receipt.

This phase does not add route discovery, strategy selection, new source-language grammar, derivatives, or proprietary optimization.

## Authority and Scope

Implementation belongs to the canonical Rust language workspace under `x3-lang/`:

- compiler policy and IR: `x3-lang/compiler`;
- bytecode verification and execution: `x3-lang/vm`;
- canonical tests: compiler and VM integration tests under those workspaces.

Root `crates/x3-*` packages remain compatibility/runtime integration surfaces. They may consume serialized canonical artifacts later but must not independently redefine Economic Safety Kernel semantics.

The existing `TradingVm`, `CompiledTradingPolicy`, host capability manifest, transaction journal, committed-cost accounting, receipt signing, state commitment checks, debt closure, and profit checks are extended rather than replaced.

## Non-goals

This phase does not:

- claim universal elimination of MEV;
- add hidden consensus rules, secret cryptography, or privileged bypasses;
- implement Blackstar or any proprietary optimizer;
- add public or private solver auctions;
- add new trading syntax;
- choose routes or discover opportunities;
- guarantee fiat-denominated values without an explicitly typed oracle source;
- treat test fixtures as production capabilities;
- weaken any existing production gate.

## Core Types

### EconomicPolicy

A compiled, immutable policy included in the artifact and covered by its hash:

- policy version;
- settlement asset;
- minimum verified net profit;
- maximum total cost;
- maximum slippage;
- maximum price impact;
- maximum MEV leakage;
- quote freshness bound;
- execution deadline;
- required submission/privacy level;
- required state-binding mode;
- allowed cost kinds;
- conservation exceptions limited to explicitly authorized fees and burns.

Runtime callers may supply stricter limits but cannot weaken compiled limits.

### EconomicSnapshot

A deterministic snapshot of the economic assumptions used for proof:

- chain and VM identifiers;
- block height and block hash where available;
- host state commitment;
- oracle observations and provenance identifiers;
- quote identifiers and observation heights;
- balances relevant to execution;
- debt state;
- route and operation commitments;
- expected outputs;
- predicted cost entries;
- timestamp/block freshness metadata.

Canonical serialization and hashing are mandatory. Unordered maps must be serialized in deterministic key order.

### EconomicPlan

The exact proposed operation sequence:

- compiled artifact hash;
- ordered operations;
- asset and amount bindings;
- venue/provider identifiers;
- minimum outputs;
- deadlines;
- expected costs;
- expected value-flow edges;
- snapshot hash;
- policy hash.

The VM executes only the plan that was proven.

### CostEntry and CostLedger

Each cost has:

- stable cost-kind identifier;
- asset;
- amount;
- source/provider;
- whether predicted or realized;
- operation index;
- evidence reference when supplied by a production host.

The kernel rejects:

- unknown cost kinds not permitted by policy;
- arithmetic overflow;
- duplicate cost identities;
- missing realized costs required by the plan;
- realized total cost above the compiled ceiling;
- attempts to evaluate final profit before all required costs are committed.

Initial stable cost kinds cover gas, liquidity fee, flash-liquidity fee, solver/infrastructure fee, proof fee, cross-domain fee, slippage, price impact, and declared MEV leakage. New kinds require an explicit versioned policy change.

### ValueFlowLedger

The ledger models value movement per typed asset and operation. At completion, for every asset:

`opening balance + authorized inflows + authorized mint = closing balance + outputs + repayments + fees + authorized burn`

Mint and burn are forbidden unless the compiled policy explicitly authorizes the capability. Debt principal is not profit. Borrowing and repayment must reconcile independently, including lender fees.

Unexplained positive or negative deltas reject the transaction and roll back the host journal.

### MevBudget

MEV resistance is expressed as measurable, policy-bound leakage rather than a boolean. The first version accounts for:

- quote deterioration;
- adverse ordering;
- route substitution;
- slippage;
- price impact beyond the committed reference;
- other explicitly classified execution leakage.

Every component has deterministic units and evidence provenance. Components cannot be silently collapsed into an unclassified total. Execution fails when the sum exceeds the compiled budget or when the environment lacks a required protection capability.

### VerifiedPnl

`VerifiedPnl` is constructed only by the kernel after successful reconciliation:

`realized proceeds - acquisition cost - debt principal - all realized costs`

The exact asset denomination must match the policy settlement asset or use a policy-approved, provenance-carrying conversion. Ordinary integers and host-provided profit claims cannot instantiate `VerifiedPnl`.

### EconomicProof

A deterministic proof record produced before external execution:

- proof version;
- artifact hash;
- policy hash;
- plan hash;
- snapshot hash;
- predicted value-flow commitment;
- predicted cost commitment;
- predicted MEV use;
- predicted verified PnL;
- evaluation result and stable rejection code.

This phase implements a deterministic signed/hashed evidence object, not a zero-knowledge proof. Naming and documentation must not imply cryptographic properties beyond hashing and signatures actually implemented.

### EconomicReceipt

A successful receipt includes:

- proof identity and all bound hashes;
- pre-execution and observed execution state commitments;
- ordered operation outcomes;
- predicted and realized cost ledgers;
- predicted and realized MEV budgets;
- value-conservation result;
- verified PnL;
- deadline/freshness evidence;
- atomic commit result;
- signer identity, signature algorithm, and signature.

Receipt verification must reject field mutation, mismatched policy/plan/state, unsupported versions, invalid signatures, missing costs, and failed conservation.

## Execution Flow

`TradingVm::prove_then_execute` follows this order:

1. Validate production/development capability mode.
2. Validate chain, version, provider, venue, submission/privacy, and protection capabilities.
3. Validate policy, plan, and snapshot versions.
4. Recompute canonical hashes and ensure all bindings agree.
5. Validate deadline, quote freshness, and oracle provenance requirements.
6. Simulate the exact plan against the snapshot.
7. Build predicted cost, MEV, debt, and value-flow ledgers.
8. Check predicted conservation, total cost, leakage, and minimum profit.
9. Re-read or revalidate the host state commitment immediately before beginning execution.
10. Abort if state differs from the proven snapshot.
11. Begin the existing host-side atomic transaction.
12. Execute only the committed ordered operations.
13. Collect all realized costs and outcomes.
14. Reconcile debt, value conservation, MEV budget, and verified PnL.
15. Roll back on any failure.
16. Commit the host transaction only after every invariant passes.
17. Produce and sign the EconomicReceipt.

Commit failure remains a failure. The VM must never issue a successful receipt before host commit succeeds.

## Error Model

Add stable, testable error variants grouped by stage:

- policy/version/capability rejection;
- malformed plan or binding mismatch;
- stale quote or deadline;
- oracle provenance failure;
- state changed after proof;
- missing, duplicate, unknown, or excessive cost;
- MEV budget exceeded;
- conservation violation;
- debt reconciliation failure;
- verified profit below floor;
- host begin/operation/rollback/commit failure;
- receipt signing or verification failure;
- checked arithmetic overflow.

Errors must not contain secrets or unstable provider payloads. Production adapters map provider failures into stable host codes while retaining detailed logs outside deterministic receipts.

## Compatibility and Versioning

The existing trading execution API remains available during migration. The new kernel is introduced as an explicit versioned path and becomes mandatory for production execution only after its tests and migration evidence are green.

Policy, proof, plan, snapshot, cost-kind, and receipt schemas carry versions. Unknown versions fail closed. Hash domains use distinct prefixes so one object type cannot be substituted for another.

Serialization must be canonical and covered by golden vectors.

## Security Boundaries

- Hosts provide external effects and evidence but do not decide whether policy passes.
- Callers cannot weaken compiled policy at runtime.
- Optimizers may propose plans but never self-certify them.
- A future Blackstar service will be treated as an untrusted plan producer behind this same public verifier.
- Private submission is a capability claim bound into the manifest and receipt; it is not inferred from configuration strings.
- Fixture manifests remain categorically unable to satisfy production execution.
- All arithmetic is checked.
- Every failure after host transaction start attempts rollback and records the rollback outcome.
- No secret master key, bypass flag, founder-only opcode, or hidden consensus path is introduced.

## Test Strategy

Development follows red-green-refactor. Tests use real compiler/VM objects and deterministic fixture hosts; mocks are used only where an external transport cannot be exercised.

### Unit tests

- canonical hashes are deterministic;
- hash domains prevent cross-type substitution;
- policy limits cannot be weakened;
- stale snapshots and quotes reject;
- unsupported versions reject;
- cost entries reject unknown kinds, duplicates, and overflow;
- MEV components sum with checked arithmetic;
- unauthorized mint/burn rejects;
- debt principal is excluded from profit;
- verified PnL cannot be constructed from unverified values;
- receipt mutation invalidates verification.

### VM integration tests

- state change between proof and execution rejects before side effects;
- missing realized cost causes rollback;
- excessive realized total cost causes rollback;
- excessive realized MEV leakage causes rollback;
- below-floor realized profit causes rollback;
- conservation failure causes rollback;
- host operation failure causes rollback;
- host commit failure never emits a successful receipt;
- successful execution commits once and produces a verifiable receipt;
- production mode rejects fixture capabilities;
- existing private-submission and state-commitment behavior remains enforced.

### Compiler/E2E tests

- compiled policy survives source/IR/bytecode boundaries already represented by current grammar;
- compile, encode, decode, prove, execute, receipt, and verify completes end to end;
- altered bytecode/policy/plan/snapshot/receipt is rejected;
- deterministic compilation produces identical policy and artifact hashes.

### Property tests

Bounded properties cover:

- accounting never wraps;
- success implies no open debt;
- success implies value conservation;
- success implies realized costs and MEV are within policy;
- failure after transaction begin never leaves a committed VM state;
- valid receipt verification is deterministic;
- single-field receipt mutation never verifies.

## Delivery Slices

1. Versioned economic types, canonical encoding, and hash domains.
2. Cost ledger and value-flow conservation.
3. MEV budget and VerifiedPnl construction.
4. EconomicProof generation and state-revalidation boundary.
5. Atomic prove-then-execute integration.
6. Signed EconomicReceipt and independent verification.
7. Compiler-to-VM E2E coverage, properties, documentation, and CI gate.

Each slice is independently reviewable, begins with a failing test, and preserves all existing gates.

## Acceptance Criteria

The phase is complete only when:

- production prove-then-execute fails closed if any required capability or evidence is absent;
- the executed plan is byte-for-byte/hash-identical to the proven plan;
- state is revalidated immediately before execution;
- every realized cost is accounted for before final profit evaluation;
- success mathematically conserves value for every asset;
- success has no unresolved debt;
- realized total cost and MEV leakage remain within compiled limits;
- realized verified PnL meets the compiled floor;
- failure cannot commit VM state or emit a successful receipt;
- a successful receipt independently verifies and detects mutation;
- the canonical compiler-to-VM E2E and property suites pass;
- CI enforces the new tests without weakening existing checks.

## Deferred Work

Blackstar, route discovery, split routing, strategy competition, shadow/canary deployment, portfolio risk, derivatives, economic debugger UX, zero-knowledge proofs, private solver auctions, and new X3Lang syntax are separate later designs.

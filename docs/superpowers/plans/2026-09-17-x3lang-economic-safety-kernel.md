# X3Lang Economic Safety Kernel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a fail-closed, deterministic prove-then-execute kernel that verifies costs, value conservation, MEV leakage, state freshness, debt closure, realized profit, and signed receipts around canonical X3Lang trading execution.

**Architecture:** Extend the canonical `x3-lang/compiler` policy/IR and split VM economic responsibilities into focused modules. `TradingVm` remains the atomic coordinator; it consumes independently testable policy, snapshot, plan, ledger, proof, and receipt types, revalidates state immediately before host effects, and commits only after realized reconciliation succeeds.

**Tech Stack:** Rust 2021, serde/bincode canonical encoding, SHA-256 domain-separated hashing, ed25519-dalek signatures, proptest, Cargo tests.

**Spec:** `docs/superpowers/specs/2026-09-17-x3lang-economic-safety-kernel-design.md`

## Global Constraints

- Canonical compiler and VM semantics live only under `x3-lang/`.
- No new X3Lang grammar is introduced in this phase.
- Runtime callers may strengthen but never weaken compiled policy.
- Unknown schema versions, cost kinds, and protection profiles fail closed.
- Fixture capability manifests cannot satisfy production execution.
- Every arithmetic operation affecting economics uses checked arithmetic.
- No successful receipt is created before host commit succeeds.
- No hidden bypass, privileged opcode, secret consensus path, or proprietary optimizer is added.
- Existing tests and production gates remain enabled.

## File Map

- Modify `x3-lang/compiler/src/ir.rs`: versioned compiled economic policy fields and stable enums shared with the VM.
- Create `x3-lang/vm/src/economic.rs`: policy limits, snapshot, plan, canonical hashing, cost ledger, value-flow ledger, MEV budget, and verified PnL.
- Create `x3-lang/vm/src/economic_proof.rs`: deterministic proof creation and proof verification.
- Create `x3-lang/vm/src/economic_receipt.rs`: signed receipt construction and independent verification.
- Modify `x3-lang/vm/src/lib.rs`: export the three focused modules.
- Modify `x3-lang/vm/src/trading.rs`: host state revalidation and atomic prove-then-execute orchestration.
- Create `x3-lang/vm/tests/economic_types.rs`: hashing, versioning, and policy tests.
- Create `x3-lang/vm/tests/economic_ledger.rs`: cost, conservation, MEV, and PnL tests.
- Create `x3-lang/vm/tests/prove_then_execute.rs`: failure matrix and successful execution tests.
- Create `x3-lang/vm/tests/economic_receipt.rs`: signing and tamper-detection tests.
- Create `x3-lang/vm/tests/economic_properties.rs`: bounded accounting and receipt properties.
- Create `x3-lang/compiler/tests/test_economic_policy_preservation.rs`: compiler-to-IR policy preservation.
- Modify `x3-lang/compiler/tests/test_trading_core_e2e.rs`: compile/encode/decode/prove/execute/receipt/verify lifecycle.
- Modify the existing X3Lang GitHub Actions workflow discovered during Task 7: enforce the targeted suites without duplicating workflows.
- Create `reports/x3lang/economic-safety-kernel-evidence.md`: exact commands, commit, and results.

---

### Task 1: Versioned Economic Policy and Canonical Commitments

**Files:**
- Modify: `x3-lang/compiler/src/ir.rs`
- Create: `x3-lang/vm/src/economic.rs`
- Modify: `x3-lang/vm/src/lib.rs`
- Create: `x3-lang/vm/tests/economic_types.rs`

**Interfaces:**
- Consumes: existing `AssetKey` and `CompiledTradingPolicy`.
- Produces: `SubmissionProfile`, `StateBindingMode`, `CostKind`, `EconomicPolicy`, `EconomicSnapshot`, `EconomicPlan`, `CanonicalCommitment::commitment()`, and `EconomicError`.

- [ ] **Step 1: Write failing canonical-commitment and policy-strength tests**

Create tests that require these exact public contracts:

```rust
#[test]
fn economic_commitments_are_deterministic_and_domain_separated() {
    let snapshot = fixture_snapshot();
    assert_eq!(snapshot.commitment().unwrap(), snapshot.commitment().unwrap());
    assert_ne!(snapshot.commitment().unwrap(), fixture_plan(&snapshot).commitment().unwrap());
}

#[test]
fn runtime_policy_cannot_weaken_compiled_policy() {
    let compiled = fixture_policy();
    let mut runtime = compiled.clone();
    runtime.max_total_cost += 1;
    assert_eq!(
        runtime.validate_not_weaker_than(&compiled),
        Err(EconomicError::PolicyWeakening("max_total_cost"))
    );
}

#[test]
fn unknown_versions_fail_closed() {
    let mut snapshot = fixture_snapshot();
    snapshot.version = u16::MAX;
    assert_eq!(
        snapshot.validate_version(),
        Err(EconomicError::UnsupportedVersion { object: "snapshot", version: u16::MAX })
    );
}
```

- [ ] **Step 2: Run the target and verify RED**

Run:

```bash
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-vm --test economic_types
```

Expected: compilation fails because `economic` types and exports do not exist.

- [ ] **Step 3: Add stable policy enums and fields**

Extend `CompiledTradingPolicy` with versioned fields while updating every existing constructor:

```rust
pub max_total_cost: u128,
pub max_price_impact_bps: u16,
pub max_mev_leakage_bps: u16,
pub quote_freshness_blocks: u64,
pub submission_profile: SubmissionProfile,
pub state_binding: StateBindingMode,
pub allowed_cost_kinds: BTreeSet<CostKind>,
pub allow_mint: bool,
pub allow_burn: bool,
```

Define enums with `Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize`. Retain `require_private_submission` during migration and enforce consistency with `submission_profile`.

- [ ] **Step 4: Implement focused economic types and domain-separated hashing**

Use exact domain prefixes:

```rust
const SNAPSHOT_DOMAIN: &[u8] = b"X3:ECONOMIC_SNAPSHOT:V1";
const PLAN_DOMAIN: &[u8] = b"X3:ECONOMIC_PLAN:V1";
const POLICY_DOMAIN: &[u8] = b"X3:ECONOMIC_POLICY:V1";

pub trait CanonicalCommitment {
    fn domain(&self) -> &'static [u8];
    fn canonical_bytes(&self) -> Result<Vec<u8>, EconomicError>;

    fn commitment(&self) -> Result<[u8; 32], EconomicError> {
        let mut hasher = Sha256::new();
        hasher.update(self.domain());
        hasher.update(self.canonical_bytes()?);
        Ok(hasher.finalize().into())
    }
}
```

Use `BTreeMap` and `BTreeSet` inside committed objects. Canonical bytes are `bincode::serialize(self)`; serialization failure maps to `EconomicError::CanonicalEncoding`.

- [ ] **Step 5: Run targeted and existing trading tests**

Run:

```bash
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-vm --test economic_types
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-vm --test trading_execution
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-vm --test trading_properties
```

Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add x3-lang/compiler/src/ir.rs x3-lang/vm/src/economic.rs x3-lang/vm/src/lib.rs x3-lang/vm/tests/economic_types.rs x3-lang/vm/tests/trading_execution.rs x3-lang/vm/tests/trading_properties.rs x3-lang/compiler/tests
git commit -m "feat(x3lang): add versioned economic commitments"
```

---

### Task 2: Cost, Value-Flow, MEV, and Verified-PnL Ledgers

**Files:**
- Modify: `x3-lang/vm/src/economic.rs`
- Create: `x3-lang/vm/tests/economic_ledger.rs`

**Interfaces:**
- Consumes: `EconomicPolicy`, `AssetKey`, and `CostKind`.
- Produces: `CostEntry`, `CostLedger::reconcile`, `ValueFlowEntry`, `ValueFlowLedger::verify`, `MevComponent`, `MevBudget::verify`, and `VerifiedPnl::reconcile`.

- [ ] **Step 1: Write failing ledger tests**

Cover one behavior per test:

```rust
#[test]
fn duplicate_cost_identity_is_rejected() {
    let entry = fixture_cost("gas:0", CostKind::Gas, 10);
    let ledger = CostLedger::new(vec![entry.clone(), entry]);
    assert_eq!(ledger.reconcile(&fixture_policy()), Err(EconomicError::DuplicateCost("gas:0".into())));
}

#[test]
fn unexplained_asset_delta_breaks_conservation() {
    let mut ledger = balanced_value_flow();
    ledger.entries.push(ValueFlowEntry::output(asset("USDC"), 1, "unexplained"));
    assert!(matches!(ledger.verify(&fixture_policy()), Err(EconomicError::ConservationViolation { .. })));
}

#[test]
fn mev_budget_rejects_excessive_leakage() {
    let budget = MevBudget::new(asset("USDC"), 100_000, vec![
        MevComponent::new(MevKind::AdverseOrdering, 60, "ordering"),
    ]);
    assert_eq!(budget.verify(5), Err(EconomicError::MevBudgetExceeded { maximum_bps: 5, actual_bps: 6 }));
}

#[test]
fn debt_principal_is_not_profit() {
    let pnl = VerifiedPnl::reconcile(&fixture_pnl_inputs_with_borrowed_principal());
    assert_eq!(pnl.unwrap().amount(), 25);
}
```

Also test unknown cost kind/version, overflow, unauthorized mint/burn, missing realized cost, total-cost ceiling, and settlement-asset mismatch.

- [ ] **Step 2: Run and verify RED**

Run:

```bash
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-vm --test economic_ledger
```

Expected: compilation fails because ledger types are absent.

- [ ] **Step 3: Implement the minimal checked ledgers**

Use stable entry identities and exact checked helpers:

```rust
fn checked_add(lhs: u128, rhs: u128) -> Result<u128, EconomicError> {
    lhs.checked_add(rhs).ok_or(EconomicError::AccountingOverflow)
}

impl ValueFlowLedger {
    pub fn verify(&self, policy: &EconomicPolicy) -> Result<(), EconomicError> {
        for asset in self.assets() {
            let sources = self.sources_for(&asset, policy)?;
            let sinks = self.sinks_for(&asset, policy)?;
            if sources != sinks {
                return Err(EconomicError::ConservationViolation { asset, sources, sinks });
            }
        }
        Ok(())
    }
}
```

Make `VerifiedPnl` fields private. Its only public constructor is `VerifiedPnl::reconcile`; expose read-only `asset()` and `amount()`.

- [ ] **Step 4: Run ledger and property regression targets**

Run:

```bash
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-vm --test economic_ledger
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-vm --test trading_properties
```

Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add x3-lang/vm/src/economic.rs x3-lang/vm/tests/economic_ledger.rs
git commit -m "feat(x3lang): reconcile economic ledgers"
```

---

### Task 3: Deterministic Economic Proof and State Revalidation

**Files:**
- Create: `x3-lang/vm/src/economic_proof.rs`
- Modify: `x3-lang/vm/src/lib.rs`
- Modify: `x3-lang/vm/src/trading.rs`
- Create: `x3-lang/vm/tests/economic_proof.rs`

**Interfaces:**
- Consumes: committed policy, snapshot, plan, predicted ledgers, host capability manifest.
- Produces: `EconomicProof::build`, `EconomicProof::verify`, `TradingHost::current_state_commitment`, and stable proof-stage errors.

- [ ] **Step 1: Write failing proof-binding tests**

```rust
#[test]
fn proof_binds_policy_plan_and_snapshot() {
    let fixture = proof_fixture();
    let proof = EconomicProof::build(&fixture.policy, &fixture.snapshot, &fixture.plan, &fixture.predicted).unwrap();
    assert!(proof.verify(&fixture.policy, &fixture.snapshot, &fixture.plan, &fixture.predicted).is_ok());

    let mut altered_plan = fixture.plan.clone();
    altered_plan.operations.reverse();
    assert_eq!(
        proof.verify(&fixture.policy, &fixture.snapshot, &altered_plan, &fixture.predicted),
        Err(EconomicError::PlanCommitmentMismatch)
    );
}

#[test]
fn state_change_after_proof_rejects_before_host_begin() {
    let (mut vm, mut host, request) = changed_state_fixture();
    assert_eq!(vm.prove_then_execute(request, &mut host).unwrap_err(), EconomicExecError::StateChangedAfterProof);
    assert_eq!(host.begin_calls, 0);
}
```

- [ ] **Step 2: Run and verify RED**

Run:

```bash
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-vm --test economic_proof
```

Expected: compilation fails because the proof API and state commitment method do not exist.

- [ ] **Step 3: Add the host revalidation boundary**

Add a required production-capable host method:

```rust
fn current_state_commitment(&self) -> Result<[u8; 32], HostError>;
```

Update every test host. The default implementation must not return the manifest value; absence is a stable host rejection so production adapters cannot accidentally claim revalidation.

- [ ] **Step 4: Implement proof construction and verification**

Proof creation validates versions, freshness, policy strength, canonical bindings, predicted cost limits, predicted MEV limit, conservation, debt closure, and predicted PnL before returning `EconomicProof`. Store only hashes plus deterministic evaluated values required by the receipt.

- [ ] **Step 5: Revalidate state before host transaction start**

In `TradingVm::prove_then_execute`, call `current_state_commitment()` after proof verification and immediately before `begin_transaction()`. Compare it with the snapshot and manifest commitments. Any mismatch returns `StateChangedAfterProof` without host side effects.

- [ ] **Step 6: Run targeted proof and existing host tests**

Run:

```bash
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-vm --test economic_proof
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-vm --test trading_execution
```

Expected: all pass.

- [ ] **Step 7: Commit**

```bash
git add x3-lang/vm/src/economic_proof.rs x3-lang/vm/src/lib.rs x3-lang/vm/src/trading.rs x3-lang/vm/tests
git commit -m "feat(x3lang): prove economic plan before execution"
```

---

### Task 4: Atomic Prove-Then-Execute Reconciliation

**Files:**
- Modify: `x3-lang/vm/src/trading.rs`
- Create: `x3-lang/vm/tests/prove_then_execute.rs`
- Modify: `x3-lang/vm/tests/trading_execution.rs`

**Interfaces:**
- Consumes: `EconomicExecutionRequest`, `EconomicProof`, exact ordered `EconomicPlan`, and `TradingHost`.
- Produces: `TradingVm::prove_then_execute(...) -> Result<EconomicExecution, EconomicExecError>`.

- [ ] **Step 1: Write the failure-matrix tests before orchestration code**

Add distinct tests proving rollback for missing realized cost, excessive total cost, excessive MEV leakage, conservation failure, unresolved debt, below-floor profit, host operation failure, and commit failure. Each asserts:

```rust
assert_eq!(vm.trading_state, state_before);
assert_eq!(host.rollback_calls, 1);
assert_eq!(host.commit_calls, expected_commit_calls);
assert!(result.is_err());
```

Add success assertions:

```rust
assert!(execution.committed_state.committed);
assert!(execution.committed_state.open_debts.is_empty());
assert_eq!(host.begin_calls, 1);
assert_eq!(host.commit_calls, 1);
assert_eq!(host.rollback_calls, 0);
```

- [ ] **Step 2: Run and verify RED**

Run:

```bash
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-vm --test prove_then_execute
```

Expected: tests fail because realized reconciliation is not wired into the atomic path.

- [ ] **Step 3: Implement exact execution ordering**

Implement the coordinator in this fixed order:

```rust
proof.verify(...)?;
revalidate_state(host, request.snapshot())?;
host.begin_transaction().map_err(EconomicExecError::HostBegin)?;
let result = self.execute_proven_plan(request, host);
match result {
    Ok(pending) => {
        let reconciled = pending.reconcile_realized()?;
        host.commit_transaction().map_err(EconomicExecError::HostCommit)?;
        self.trading_state = reconciled.committed_state.clone();
        Ok(reconciled)
    }
    Err(error) => {
        let rollback = host.rollback_transaction();
        self.trading_state = state_before;
        Err(EconomicExecError::execution_with_rollback(error, rollback))
    }
}
```

Ensure every reconciliation failure takes the rollback branch. If commit fails, restore VM state, attempt rollback, and return a commit-stage error; never mark the execution committed.

- [ ] **Step 4: Run focused and complete VM suites**

Run:

```bash
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-vm --test prove_then_execute
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-vm --all-targets
```

Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add x3-lang/vm/src/trading.rs x3-lang/vm/tests/prove_then_execute.rs x3-lang/vm/tests/trading_execution.rs
git commit -m "feat(x3lang): execute only proven economic plans"
```

---

### Task 5: Signed Economic Receipts and Tamper Detection

**Files:**
- Create: `x3-lang/vm/src/economic_receipt.rs`
- Modify: `x3-lang/vm/src/lib.rs`
- Modify: `x3-lang/vm/src/trading.rs`
- Create: `x3-lang/vm/tests/economic_receipt.rs`

**Interfaces:**
- Consumes: committed `EconomicExecution`, proof identity, predicted and realized ledgers, signing key.
- Produces: `EconomicReceipt::sign`, `EconomicReceipt::verify`, `EconomicReceiptVerifier`, and receipt domain `X3:ECONOMIC_RECEIPT:V1`.

- [ ] **Step 1: Write failing signature and mutation tests**

```rust
#[test]
fn committed_execution_produces_verifiable_receipt() {
    let receipt = fixture_committed_execution().sign(&fixture_signing_key()).unwrap();
    assert!(receipt.verify(&fixture_verifying_key()).is_ok());
}

#[test]
fn mutating_any_economic_result_breaks_verification() {
    let mut receipt = fixture_receipt();
    receipt.realized_total_cost += 1;
    assert_eq!(receipt.verify(&fixture_verifying_key()), Err(ReceiptError::InvalidSignature));
}

#[test]
fn receipt_cannot_be_issued_before_commit() {
    let pending = fixture_pending_execution();
    assert_eq!(EconomicReceipt::from_execution(&pending), Err(ReceiptError::ExecutionNotCommitted));
}
```

Also mutate proof hash, policy hash, plan hash, snapshot hash, PnL, MEV, conservation flag, operation outcome, signer identity, signature, and version in separate table-driven cases.

- [ ] **Step 2: Run and verify RED**

Run:

```bash
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-vm --test economic_receipt
```

Expected: compilation fails because receipt types do not exist.

- [ ] **Step 3: Implement receipt signing after commit**

Serialize an unsigned receipt payload canonically, hash with `X3:ECONOMIC_RECEIPT:V1`, and sign that digest using `ed25519-dalek`. Verification recomputes all structural checks and the signature. Receipt constructors accept only the committed execution type returned after host commit.

- [ ] **Step 4: Attach receipt creation to successful execution**

Return:

```rust
pub struct EconomicExecution {
    pub committed_state: TradingState,
    pub proof: EconomicProof,
    pub reconciliation: EconomicReconciliation,
    pub receipt: EconomicReceipt,
}
```

Supply the signer through an explicit VM execution dependency; do not store a global or hard-coded signing key.

- [ ] **Step 5: Run receipt and VM suites**

Run:

```bash
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-vm --test economic_receipt
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-vm --all-targets
```

Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add x3-lang/vm/src/economic_receipt.rs x3-lang/vm/src/lib.rs x3-lang/vm/src/trading.rs x3-lang/vm/tests/economic_receipt.rs
git commit -m "feat(x3lang): sign verifiable economic receipts"
```

---

### Task 6: Compiler Preservation and Full Economic Lifecycle E2E

**Files:**
- Create: `x3-lang/compiler/tests/test_economic_policy_preservation.rs`
- Modify: `x3-lang/compiler/tests/test_trading_core_e2e.rs`
- Modify only when a test demonstrates loss: `x3-lang/compiler/src/trading_lowering.rs`, `x3-lang/compiler/src/emitter.rs`, or decoder code used by the E2E test.

**Interfaces:**
- Consumes: existing accepted trading syntax and compiled `TradingOperation`.
- Produces: executable evidence for compile → encode → decode → prove → execute → receipt → verify.

- [ ] **Step 1: Write policy-preservation tests using existing grammar**

Assert every currently expressible economic field survives source analysis and lowering. For fields not yet expressible, build `CompiledTradingPolicy` directly and test byte encoding/decoding; do not add grammar.

- [ ] **Step 2: Run preservation target and record RED only for demonstrated loss**

Run:

```bash
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-compiler --test test_economic_policy_preservation
```

Expected: either a focused failure identifying the exact dropped field or PASS proving current preservation. Do not manufacture a failure by changing semantics.

- [ ] **Step 3: Repair only demonstrated preservation gaps**

Trace each failed field through semantic representation → trading lowering → IR → emitter → decoder. Forward the field unchanged at the earliest loss point.

- [ ] **Step 4: Extend the real E2E lifecycle**

The E2E assertion must prove:

```text
accepted source or canonical IR
→ encoded artifact
→ decoded TradingOperation sequence
→ EconomicProof
→ exact-plan execution
→ host commit
→ EconomicReceipt
→ independent receipt verification
```

Add negative subcases for modified decoded policy and modified plan hashes.

- [ ] **Step 5: Run compiler and VM verification**

Run:

```bash
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-compiler --test test_economic_policy_preservation
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-compiler --test test_trading_core_e2e
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-compiler --all-targets
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-vm --all-targets
```

Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add x3-lang/compiler/src x3-lang/compiler/tests
git commit -m "test(x3lang): prove economic lifecycle end to end"
```

---

### Task 7: Property Proofs, CI Gate, and Evidence

**Files:**
- Create: `x3-lang/vm/tests/economic_properties.rs`
- Modify: the most specific existing X3Lang workflow found by repository search
- Create: `reports/x3lang/economic-safety-kernel-evidence.md`

**Interfaces:**
- Consumes: all kernel APIs and suites from Tasks 1–6.
- Produces: bounded invariant evidence and an exact-head CI enforcement gate.

- [ ] **Step 1: Write bounded property tests**

Use `proptest` with checked ranges and these properties:

```rust
proptest! {
    #[test]
    fn success_implies_conservation_and_policy_compliance(case in economic_case()) {
        let result = execute_case(case);
        if let Ok(execution) = result {
            prop_assert!(execution.reconciliation.value_conserved);
            prop_assert!(execution.reconciliation.total_cost <= execution.proof.max_total_cost);
            prop_assert!(execution.reconciliation.mev_bps <= execution.proof.max_mev_leakage_bps);
            prop_assert!(execution.committed_state.open_debts.is_empty());
        }
    }

    #[test]
    fn one_field_receipt_mutation_never_verifies(case in committed_case(), field in mutable_receipt_field()) {
        let (mut receipt, key) = case;
        mutate_one_field(&mut receipt, field);
        prop_assert!(receipt.verify(&key).is_err());
    }
}
```

Add a rollback property: any error after host begin leaves VM state equal to its pre-execution clone and never yields a successful receipt.

- [ ] **Step 2: Run properties with a fixed minimum case count**

Run:

```bash
PROPTEST_CASES=512 cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-vm --test economic_properties
```

Expected: all 512 generated cases pass. If a minimized failure appears, add it first as a named regression test, verify RED, then implement the smallest fix.

- [ ] **Step 3: Discover and extend the existing workflow**

Run:

```bash
git grep -nE 'x3-lang|x3lang' .github/workflows
```

Extend the most specific existing workflow with these exact gates; do not create a duplicate workflow:

```bash
cargo fmt --manifest-path x3-lang/Cargo.toml --all -- --check
cargo clippy --manifest-path x3-lang/Cargo.toml --workspace --all-targets --all-features -- -D warnings
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-compiler --all-targets
PROPTEST_CASES=512 cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-vm --all-targets
```

- [ ] **Step 4: Run the complete local gate**

Run the same four commands locally and retain unedited output for the evidence report. Expected: exit code 0 for every command, zero failed tests, and zero Clippy warnings.

- [ ] **Step 5: Write the evidence report**

Record:

```markdown
# Economic Safety Kernel Evidence

Commit: <full exact HEAD>
Branch: codex/x3-trading-core-v1-hardening
Spec: docs/superpowers/specs/2026-09-17-x3lang-economic-safety-kernel-design.md
Plan: docs/superpowers/plans/2026-09-17-x3lang-economic-safety-kernel.md

| Gate | Command | Result | Evidence |
|---|---|---|---|
| Format | exact command | PASS | concise unedited summary |
| Clippy | exact command | PASS | concise unedited summary |
| Compiler | exact command | PASS | test count |
| VM/property | exact command | PASS | test count and PROPTEST_CASES |
```

Use the actual full SHA and actual output. Never pre-fill PASS before the command completes.

- [ ] **Step 6: Commit**

```bash
git add .github/workflows reports/x3lang/economic-safety-kernel-evidence.md x3-lang/vm/tests/economic_properties.rs
git commit -m "ci(x3lang): gate economic safety kernel"
```

- [ ] **Step 7: Verify the final exact head**

Run:

```bash
git rev-parse --verify HEAD
git status --short
cargo fmt --manifest-path x3-lang/Cargo.toml --all -- --check
cargo clippy --manifest-path x3-lang/Cargo.toml --workspace --all-targets --all-features -- -D warnings
cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-compiler --all-targets
PROPTEST_CASES=512 cargo test --manifest-path x3-lang/Cargo.toml -p x3-lang-vm --all-targets
```

Expected: clean status, format/clippy exit 0, and all compiler/VM tests pass on the reported exact SHA.

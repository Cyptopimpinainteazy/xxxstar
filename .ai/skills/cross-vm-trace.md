# Skill: Cross-VM Trace

Use this skill whenever a task touches Cross-VM logic.

This forces you to document the entire state machine before building it.

## Trace Format

```md
## Cross-VM Trace

### Intent
- What does the user/system want to happen?
- What is the desired end state?

### Source domain
- Native / EVM / SVM / BTC / External

### Destination domain
- Native / EVM / SVM / BTC / External

### Canonical path
Document the exact sequence:

```txt
intent (step 1)
    ↓
X3IR representation (step 2)
    ↓
dispatch decision (step 3)
    ↓
source VM action (step 4)
    ↓
bridge lock/burn (step 5)
    ↓
proof generation (step 6)
    ↓
relay to destination (step 7)
    ↓
destination VM verification (step 8)
    ↓
destination settlement (step 9)
    ↓
finality recorded (step 10)
    ↓
state update (step 11)
```

### State transition

For each critical step:

**Before**
- Source domain state: (balances, nonces, locks)
- Destination domain state: (balances, nonces, claims)
- What invariants must be true?

**During (critical section)**
- What is locked?
- What is at risk if execution fails here?
- Is revert possible?

**After success**
- Source domain state: (what changed)
- Destination domain state: (what changed)
- What new invariants hold?

**After failure**
- What is the failure state?
- Can the operation be retried?
- Is compensation needed?

**Timeout**
- What happens if step 8 never completes?
- Who initiates refund?
- Can refund be authorized by anyone?

**Replay attempt**
- If step 6 (proof) is replayed, what happens?
- Is nonce checked?
- Is the replay rejected with clear error?

### Atomicity rule

Document what guarantees no half-complete state:

```txt
Atomicity achieved by:
- Single-transaction execution on source and destination
- Atomic settlement: lock succeeds only if mint succeeds
- Timeout guards: refund only possible if settlement failed
- Nonce consumption: prevents replay from creating duplicate state
```

OR

```txt
Atomicity acknowledged as eventual:
- Lock on source (committed)
- Settlement on destination (committed separately)
- Compensation ledger records failures
- Refund path available if settlement fails
```

### Tests required

Must have tests for:

- [ ] **Success path:** lock → mint → state update (happy case)
- [ ] **Failure path:** settlement fails, refund initiated (sad case)
- [ ] **Replay path:** duplicate proof rejected (security)
- [ ] **Timeout path:** refund allowed after timeout (liveness)
- [ ] **Rollback/compensation:** partial failure recovered (safety)
- [ ] **Edge case:** [custom edge case for this domain]

### Validation commands

```txt
cargo test --test cross_vm_<name> -- --nocapture
```
```

## How to Use This Skill

1. **Before coding:** Trace the entire state machine
2. **Before writing runtime code:** Trace all possible paths
3. **Before claiming feature works:** Trace what actually happens
4. **During debugging:** Trace where the state diverged
5. **During review:** Trace whether the code matches the documented path

## Red Flags

If you cannot trace a path, something is wrong:

- [ ] Code is too complex (needs refactoring)
- [ ] Path is unclear (needs design clarification)
- [ ] Multiple competing paths exist (needs canonical path decision)
- [ ] Failure modes are not documented (needs error handling design)

Fix these before proceeding.

## Example: EVM → Native Swap

```md
## Cross-VM Trace: EVM to Native Swap

### Intent
User wants to swap EVM USDC for Native ATOM.

### Source domain
EVM

### Destination domain
Native (Atom chain)

### Canonical path
```txt
user submits swap intent (EVM)
    ↓
intent routed to X3IR (validation)
    ↓
dispatch decides: EVM source, Native dest
    ↓
EVM contract locks USDC in escrow
    ↓
bridge relayer creates proof (EVM state root)
    ↓
relayer submits proof to Native chain
    ↓
Native verifier checks proof (EVM root valid?)
    ↓
Native settlement: mint ATOM to user
    ↓
Native records event (finality)
    ↓
EVM learns of settlement (optional: confirms lock permanent)
    ↓
user receives ATOM
```

### Before state
- EVM: user has 100 USDC, nonce 42
- Native: user has 0 ATOM, nonce 5
- Both: no locks

### During (critical section)
- EVM lock: 100 USDC moved to escrow (user cannot undo)
- Proof generated: hash of EVM state
- Relay: proof sent to Native
- Native mint: ATOM created (user gains it)

### After success
- EVM: user has 0 USDC (locked in escrow), nonce 43
- Native: user has 100 ATOM (or agreed exchange rate), nonce 6
- Records: swap completed, finality recorded

### Timeout (if relayer fails)
- After 6 hours: refund initiated
- EVM contract releases USDC back to user
- Native does not mint (no matching action)

### Replay attempt
- If proof is resubmitted 10 seconds later
- Native checks: nonce/proof-hash already consumed? YES
- Native rejects: DuplicateProof error
- No double mint

### Atomicity rule
- Atomicity achieved by: lock on EVM is consumed by proof; proof consumed on Native prevents replay
- Nonce prevents double-execution
- Timeout refund restores consistency if Native fails
```

## Always Include

- Source and destination domains (explicit)
- Every step in the path (testable)
- Before/during/after/failure/timeout states (comprehensive)
- Replay protection details (security)
- Test list (validation)

---

**When this skill is complete:** You can trace your feature from user input to final state, and every step is testable.

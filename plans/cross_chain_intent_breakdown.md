# Cross‑Chain Intents – Full Breakdown

## Core Concept
A **cross‑chain intent** is a high‑level user declaration of *what* they want to achieve across multiple blockchains, together with *safety constraints* that the system must enforce.  The intent is compiled into an execution plan (X3IR) that the runtime, adapters, and GPU validator execute and verify.

---

## Intent Syntax (x3‑lang)
```x3
intent <name> {
  from <chain>.<asset> amount <N> owner <address>
  to   <chain>.<asset>   receiver <address>

  route best {
    prefer lowest_total_cost
    allow dex.uniswap
    allow dex.x3
    allow bridge.x3_kernel
  }

  require slippage <= 0.5%
  require finality eth >= 12
  require max_fee <= 10 USDC
  require receiver == wallet.owner
  require proof eth.lock_event
  require canonical_supply_valid

  timeout 30m
  on_timeout refund eth.USDC to <source>
  on_fail {
    rollback_if_possible
    refund x3.USDC.e to <address>
    fallback refund eth.USDC to <source>
    final quarantine
  }

  receipt verbose
}
```

### Sections
| Section | Purpose |
|---|---|
| **from / to** | Declare source and destination assets, amounts, and owners. |
| **route** | Hint the planner which DEXes/bridges are allowed and optimisation goals. |
| **require** | Safety guards – slippage, finality, fee caps, proof existence, invariant checks. |
| **timeout / on_timeout** | Prevent funds from being stuck; define automatic refund path. |
| **on_fail** | Recovery strategy if any step fails (rollback, refund, quarantine). |
| **receipt** | Desired level of traceability for explorers and wallets. |

---

## Execution Pipeline
1. **User Intent** – Signed high‑level request.
2. **Parse → AST → X3IR** – `x3‑compiler` produces a deterministic intermediate representation.
3. **Simulation** – Planner estimates fees, slippage, liquidity, risk score, and validates that all `require` clauses can be satisfied.
4. **Plan Generation** – DAG of X3IR instructions (Lock, VerifyFinality, VerifyProof, Mint, Swap, Release, …).
5. **Adapter Calls** – Each chain‑specific adapter (EVM, SVM, BTC, Substrate) turns X3IR into real chain calls.
6. **GPU Validation** – Batch‑able jobs (signature checks, Merkle proofs, ZK proofs, route‑search) are sent to the GPU swarm.  The runtime only accepts a compact proof receipt.
7. **Runtime Enforcement** – Checks canonical‑supply invariant, nonce uniqueness, replay protection, fee accounting, and policy guards.
8. **Final Receipt** – Human‑readable trace emitted for explorers and wallets.

---

## Feature Breakdown
| Feature | Why it belongs in the intent | Implementation Hint |
|---|---|---|
| **Bridge** | Moves value across chains; core cross‑chain capability. | `bridge` keyword in X3IR; adapter implements lock/mint/release flow. |
| **Swap** | Converts assets; may happen before, after, or around a bridge. | `swap` instruction; planner selects DEXes based on `route` block. |
| **Lock / Mint / Burn / Release** | Asset lifecycle; enforces canonical supply invariant. | Primitive X3IR ops; runtime validates invariant after each step. |
| **Finality Requirements** | Guarantees that a source lock cannot be reverted. | `require finality <chain> >= <N>` → runtime waits for confirmations before proceeding. |
| **Timeout Handling** | Guarantees funds are not locked forever. | `timeout <duration>` + `on_timeout` clause; runtime schedules a timer task. |
| **Refund Paths** | Defines deterministic recovery when something fails. | `on_fail` block with ordered fallback actions. |
| **Proof Requirements** | Trust‑minimized verification of cross‑chain events. | `require proof <type>` → GPU validator produces `GpuProofReceipt`. |
| **Policy Guards** | Prevents abuse (slippage, fee caps, risk scores). | Compiled into runtime checks before each state transition. |
| **Receipt / Explorer Trace** | Provides transparency to users and auditors. | `receipt` keyword controls verbosity of emitted trace. |

---

## State Machine
```
Draft → Signed → Simulated → Accepted → SourceLocked →
SourceFinalized → ProofVerified → CanonicalMinted → SwapExecuted →
DestinationReleased → Completed
```
Failure branches: `FailedSimulation`, `Expired`, `Refunding`, `Refunded`, `Quarantined`, `Disputed`, `Slashed`.

---

## Minimal MVP Intent (v0.1)
```x3
intent bridge_usdc_to_x3 {
  from eth.USDC amount 100 owner alice.eth
  to   x3.USDC.e   receiver alice.x3

  require finality eth >= 12
  require proof eth.lock_event
  require canonical_supply_valid

  timeout 30m
  on_timeout refund eth.USDC to alice.eth
}
```
*Parses → X3IR → simulated lock → mock proof → mint → receipt.*

---

## Next Steps
1. Extend the parser to recognise `intent`, `require`, `route`, `timeout`, `on_fail` blocks.
2. Add corresponding AST nodes in `crates/x3-ast` and X3IR enums.
3. Implement planner simulation that validates all `require` clauses.
4. Wire GPU‑validator job creation for `require proof` statements.
5. Update runtime pallet (`pallets/x3-atomic-kernel`) to enforce canonical‑supply invariant and timeout handling.
6. Write integration tests in `x3-lang/tests/test_cross_vm_feature.rs` covering success, timeout, and failure paths.

---

*This document provides the detailed breakdown required to implement cross‑chain intents while keeping the language focused and the runtime responsible for enforcement.*
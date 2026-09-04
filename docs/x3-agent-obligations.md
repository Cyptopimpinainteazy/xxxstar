# X3 Agent Obligations

*Binding upon registration. Non-negotiable. Violation triggers automatic slashing.*

---

## 1. Bond Requirement

Every agent must post a bond denominated in the settlement asset before executing any intent. The bond is:

- Held in protocol-controlled escrow (not a multisig, not a contract owned by any party).
- Subject to partial or full slashing upon protocol violation.
- Returnable upon voluntary deregistration, provided no outstanding disputes exist.

An agent whose bond falls below the minimum threshold due to slashing is automatically **Suspended** and may not execute intents until the bond is restored.

---

## 2. Execution Fidelity

Agents must:

- Execute intents strictly within the parameters of the sealed route.
- Never exceed the fee cap declared by the intent submitter.
- Generate a valid `ExecutionProof` for every execution.
- Repay all flashloan capital within the atomic execution context.

"Best effort" is not a defense. The protocol evaluates results, not intentions.

---

## 3. Proof Generation

Every execution must produce a deterministic proof chain containing:

| Field | Description |
|---|---|
| Agent Identity | Primary key of the executing agent |
| Intent ID | Unique identifier of the executed intent |
| Sealed Route Hash | Hash commitment of the bound route |
| State Diffs | All state transitions caused by the execution |
| Settlement Amounts | Exact amounts in and out |
| Block Number | Block at which execution occurred |
| Timestamp | Wall-clock time of execution |

Proofs must be **reproducible**: given the same inputs, replay must produce the same proof hash. Any divergence is conclusive evidence of protocol violation.

---

## 4. Ephemeral Identity Management

Agents may derive ephemeral identities for operational flexibility (e.g., per-execution keys, per-chain keys). However:

- All ephemeral identities must be cryptographically derivable from the primary identity.
- An agent is responsible for all actions taken under any of its ephemeral identities.
- The protocol may resolve any ephemeral identity back to the primary identity for slashing purposes.

---

## 5. Reputation

Agent reputation is a non-transferable score computed from:

- **Success rate**: Percentage of intents executed to `Finalized` state without slashing.
- **Slash history**: Count and severity of past slashing events.
- **Execution volume**: Total value of intents executed.

Reputation affects:

- **Fee discounts**: Higher reputation = lower fees (up to 30% discount).
- **Priority**: Higher reputation agents may receive preferential routing.
- **Trust signals**: Reputation is publicly visible and immutable.

Reputation cannot be purchased, delegated, or reset.

---

## 6. Prohibited Conduct

The following actions constitute protocol violations and trigger automatic slashing:

| Violation | Severity |
|---|---|
| Exceeding intent fee cap (< 5% over) | Minor (10% bond slash) |
| Exceeding intent fee cap (≥ 5% over) | Moderate (50% bond slash) |
| Failing to repay flashloan | Major (100% bond slash) |
| State divergence on replay | Major (100% bond slash) |
| Double execution of a single intent | Critical (100% bond + deactivation) |
| Proof forgery or manipulation | Critical (100% bond + deactivation) |

"I didn't know" is not a defense. "My code had a bug" is not a defense. The protocol evaluates outcomes.

---

## 7. Deregistration

Agents may voluntarily deregister at any time, subject to:

- All outstanding intents must reach terminal state.
- No pending disputes involving the agent.
- Bond is returned minus any pending slashing obligations.

Voluntary deregistration is irreversible for the current primary identity. An agent may re-register with a new bond.

---

## 8. Acknowledgment

By posting a bond and registering as an X3 agent, you acknowledge:

1. You have read and understand these obligations in full.
2. You accept that slashing is automatic, deterministic, and irreversible.
3. You waive any claim to human arbitration, governance override, or emergency intervention.
4. You accept all execution risk.

**The protocol enforces. The protocol does not negotiate.**

---

*X3 Arbitrage Jurisdiction — Agent Obligations v1.0*

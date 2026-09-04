# Rules of the X3 Floor

*Effective immediately. No amendments by vote. Changes require protocol upgrade via deterministic proof.*

---

## Preamble

X3 is a **deterministic arbitrage jurisdiction**. It is not a DAO. It is not governed by votes, proposals, or sentiment polls. It is governed by **law** — encoded in the X3 language, executed by the X3 virtual machine, and enforced by deterministic courts that replay execution proofs.

There is no governance token. There is no multisig. There is no council. The rules are the rules. Violations are punished automatically, proportionally, and irreversibly.

This document constitutes the complete operational charter of the X3 Floor.

---

## Article I — Definitions

| Term | Definition |
|---|---|
| **Agent** | A bonded participant authorized to execute arbitrage intents on the X3 Floor. |
| **Intent** | A declarative instruction specifying desired execution parameters without prescribing execution path. |
| **Bond** | Collateral posted by an agent, held in escrow, subject to slashing for protocol violations. |
| **Proof** | A cryptographic receipt binding agent identity, execution parameters, and state transitions into a single verifiable hash chain. |
| **Court** | A deterministic replay engine that adjudicates disputes without human input. |
| **Slash** | Automatic, irreversible forfeiture of a portion of an agent's bond as penalty for protocol violation. |
| **Fee Vector** | A multi-dimensional fee calculation incorporating base cost, complexity, capital scale, and reputation discount. |
| **Flashloan** | Transient execution capital borrowed and repaid within a single atomic execution context. |

---

## Article II — Participation Requirements

1. Any entity may register as an agent by posting the minimum bond. Registration is permissionless — there are no whitelists, KYC requirements, or approval processes.

2. The minimum bond is denominated in the settlement asset and is subject to periodic adjustment by protocol upgrade only.

3. Upon registration, agents receive a primary identity. Agents may derive ephemeral identities for operational use, but all ephemeral identities are cryptographically linked to the primary identity.

4. Agents assume all execution risk. The protocol guarantees fair adjudication, not profitability.

---

## Article III — Intent Lifecycle

1. An `ArbIntent` progresses through a fixed state machine:

   ```
   Submitted → RouteBound → Executing → Executed → Finalized
   ```

   Terminal states include: `Finalized`, `Slashed`, `Cancelled`, `Expired`.

2. **Route Binding**: Once an agent commits to a route, the route is sealed — its hash is recorded on-chain as a commitment. Deviating from the sealed route constitutes a slashable offense.

3. **Fee Cap**: Every intent declares a maximum fee the submitter will accept. Agents that execute above this cap are automatically slashed.

4. **Expiry**: Intents expire after the deadline specified at submission time. Expired intents cannot be executed or finalized.

5. **No Governance Override**: There is no mechanism to manually advance, revert, or modify an intent's state outside of the defined state machine transitions.

---

## Article IV — Execution Standards

1. All execution must produce an `ExecutionProof` — a hash chain binding the agent identity, intent parameters, sealed route, state diffs, and settlement amounts.

2. Proofs are **deterministic**: replaying the same inputs must produce the same proof hash. Divergence is evidence of protocol violation.

3. Agents must execute within a single atomic context. Cross-chain executions use per-chain flashloan capital — no bridged custody.

4. Double execution of a single intent is a Critical-severity slashable offense.

---

## Article V — Fee Schedule

Fees are calculated as a vector, not a flat rate:

```
TotalFee = BaseFee + ComplexityFee(legs, state_touches)
         + CapitalFee(log₂(capital))
         − ReputationDiscount(success_rate, capped at 30%)
```

| Component | Formula |
|---|---|
| Base Fee | Fixed per-intent cost |
| Complexity Fee | `legs × state_touches × rate` |
| Capital Fee | `log₂(capital) × rate` |
| Reputation Discount | `success_rate × 30%` (maximum) |

### Multipliers

| Condition | Multiplier |
|---|---|
| X3-optimized agent | 0.80× (20% discount) |
| External bot | 1.30× (30% penalty) |
| Flashloan usage | 1.50× premium |
| Cross-chain intent | +surcharge per additional chain |

These multipliers ensure that X3-native agents maintain structural economic advantage over external participants.

---

## Article VI — Dispute Resolution

1. Disputes are filed on-chain and resolved by **deterministic replay**, not by human judgment, committee vote, or arbitration.

2. The Court replays the execution using the submitted proof chains and compares the resulting state.

3. If replay produces a state divergence, the Court issues a **Guilty** verdict and the offending agent is slashed automatically at the appropriate severity tier.

4. If replay confirms the original execution, the dispute is **Dismissed** and no action is taken.

5. Dispute resolution is final. There is no appeal process.

---

## Article VII — Amendments

This document may only be amended through a protocol upgrade that itself passes deterministic verification. No vote, poll, or governance proposal can modify these rules.

**Law > Voting.**

---

*X3 Arbitrage Jurisdiction — Deterministic. Irreversible. Sovereign.*

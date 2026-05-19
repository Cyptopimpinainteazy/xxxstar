# X3 Slashing Constitution

*The law of the floor. Automatic. Deterministic. Irreversible.*

---

## Preamble

Slashing is the enforcement mechanism of the X3 jurisdiction. It is not punitive — it is mechanical. Protocol violations trigger slashing the same way gravity triggers a fall: without judgment, without delay, without appeal.

This constitution governs all slashing events within the X3 arbitrage jurisdiction.

---

## Section 1 — Principles

1. **Slashing is automatic.** No human initiates a slash. The protocol detects violations and executes penalties in the same transaction.

2. **Slashing is deterministic.** Given the same inputs, the same slash outcome is produced. Slashing logic is embedded in the protocol, not in governance proposals.

3. **Slashing is irreversible.** Slashed funds are burned or redistributed according to protocol rules. There is no undo, no refund, no exception.

4. **Slashing is proportional.** Severity tiers ensure that minor infractions receive minor penalties, while critical violations result in full bond forfeiture and permanent deactivation.

---

## Section 2 — Severity Tiers

| Tier | Bond Slash | Reputation Impact | Additional |
|---|---|---|---|
| **Minor** | 10% of bond | -5 reputation points | — |
| **Moderate** | 50% of bond | -20 reputation points | — |
| **Major** | 100% of bond | -50 reputation points | — |
| **Critical** | 100% of bond | Reset to 0 | Permanent deactivation |

---

## Section 3 — Violation Catalog

### 3.1 Minor Violations (10% slash)

- Exceeding the intent fee cap by less than 5%.
- Submitting an intent without sufficient bond margin (corrected before execution).
- Execution latency exceeding the soft deadline (but within hard deadline).

### 3.2 Moderate Violations (50% slash)

- Exceeding the intent fee cap by 5% or more.
- Failing to repay flashloan capital within the execution context.
- Submitting a route that deviates from the sealed commitment.
- Execution producing non-deterministic state diffs.

### 3.3 Major Violations (100% slash)

- State divergence confirmed by court replay.
- Submitting a fraudulent proof that fails verification.
- Executing an expired intent.
- Material deviation from sealed route affecting settlement amounts by >1%.

### 3.4 Critical Violations (100% slash + deactivation)

- Double execution of a single intent.
- Proof forgery — submitting a proof that cannot be reproduced by replay.
- Collusion detected between multiple agent identities to circumvent fee structure.
- Any action that compromises the integrity of the proof chain.

---

## Section 4 — Slash Process

```
Violation Detected
       │
       ▼
  Severity Assessed (from violation catalog)
       │
       ▼
  Bond Slashed (immediate, atomic)
       │
       ▼
  Reputation Updated
       │
       ▼
  Slash Record Created (immutable, append-only)
       │
       ▼
  [If Critical] Agent Deactivated
```

All steps occur within a single atomic operation. There is no delay between detection and execution.

---

## Section 5 — Slash Records

Every slash event produces an immutable record containing:

- **Slash ID**: Unique identifier.
- **Agent ID**: Primary identity of the slashed agent.
- **Severity**: Tier classification.
- **Reason**: Machine-readable violation code + human-readable description.
- **Amount Slashed**: Exact amount forfeited from bond.
- **Proof Hash**: Cryptographic hash of the evidence.
- **Timestamp**: Block time of the slash event.

Slash records are stored in an append-only ledger. No record can be modified, deleted, or hidden.

---

## Section 6 — Cascading Effects

### Bond Depletion

If cumulative slashing reduces an agent's bond below the minimum threshold:

1. Agent status is set to **Suspended**.
2. All pending intents are expired.
3. Agent cannot execute new intents until bond is restored.

### Reputation Collapse

If an agent's reputation drops below 10.0:

1. Agent loses all fee discounts.
2. Agent may be subject to priority demotion.
3. [If critical threshold crossed] Agent is automatically deactivated.

### Deactivation

Deactivated agents:

- Cannot execute intents.
- Cannot register new ephemeral identities.
- Bond remainder (if any) is frozen pending dispute resolution timeout.
- Primary identity is permanently burned after the resolution period.

---

## Section 7 — Interaction with Courts

Slashing that results from a court verdict follows the standard slash process. The court's deterministic replay produces the evidence, and the slash is applied automatically.

Slashing that occurs outside of court disputes (e.g., detected by on-chain monitors) can be challenged by filing a dispute. If the court replays the execution and finds no violation, the slash is **not** reversed — but the dispute record is added to the permanent ledger for transparency.

**There are no reversals.** The design is intentional: it is better to over-enforce than to create ambiguity.

---

## Section 8 — No Exceptions

There are no exceptions to this constitution. No emergency governance. No admin keys. No multisig override.

If the slashing rules need to change, they change through a protocol upgrade that itself passes deterministic verification. The new rules apply to future violations only — retroactive modification is architecturally impossible.

---

*X3 Arbitrage Jurisdiction — Slashing Constitution v1.0*

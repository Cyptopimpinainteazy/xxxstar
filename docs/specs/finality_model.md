# X3 Atomic Star — Finality Model Specification

**Version:** 1.0
**Status:** Adopted (vΩ-1.0 RC-1)
**Date:** 2026-06-09
**Scope:** Deterministic finality guarantees for the X3 Atomic Star chain

---

## 1. Overview

The X3 Atomic Star chain uses a **BABE/GRANDPA hybrid consensus** (as provided by the Substrate/Polkadot SDK framework) with **Flash Finality** acceleration. This document specifies:

- Fork-choice rules
- Equivocation safety guarantees
- View-change and timeout behavior
- Network asynchrony assumptions
- Cross-VM finality binding within X3-native blocks

---

## 2. Consensus Stack

### 2.1 Block Production (BABE)

- **Slot-based** primary block production with VRF-based leader election.
- **Slot duration:** 6 seconds (configurable via `Babe::slot_duration`).
- **Epoch length:** 600 slots (~1 hour).
- A block producer who wins the VRF lottery for a slot MUST produce exactly one block; producing zero or two blocks for the same slot is **equivocation** (slashable).

### 2.2 Grandpa Finality

- GRANDPA finalizes blocks once they receive supermajority (>2/3) pre-votes and pre-commits from the validator set.
- **Finality lag:** Typically 2–3 blocks (~12–18 seconds) under normal network conditions.
- A finalized block is **irreversible**: any fork that conflicts with a finalized block is invalid, and validators who signed conflicting finality votes commit an **equivocation** offense (slashable).

### 2.3 Flash Finality (X3 Extension)

Flash Finality is an X3-specific overlay that accelerates finality for **intra-block** atomic operations within the cross-VM router:

- After a BABE block is authored, a **flash finality sub-protocol** collects BLS signatures from ≥2/3 of the active validator set on the block header hash.
- Once the threshold is reached (typically within 500ms–2s of block proposal), the block is marked **flash-finalized** and cross-VM operations within it can be considered committed.
- If flash finality fails (timeout or insufficient signatures), the block falls back to standard GRANDPA finality.

---

## 3. Fork-Choice Rule

### 3.1 Primary Rule

The chain selection follows the **GHOST-based GRANDPA fork-choice rule**:

1. Start from the last GRANDPA-finalized block (the **finalized tip**).
2. Among all valid forks extending from the finalized tip, select the **heaviest** chain — the one with the most accumulated primary-block weight (sum of BABE primary slots won).
3. If two chains have equal weight, prefer the one whose latest block arrived first (tie-break by timestamp).

### 3.2 Equivocation Safety

- **BABE equivocation:** A validator producing two distinct blocks for the same slot is slashable. The conflicting chain fork with the **lower** VRF output is immediately **discarded** by all honest nodes.
- **GRANDPA equivocation:** Voting for two conflicting blocks in the same round is slashable. Honest nodes reject the conflicting vote-set; the finalized chain continues on the set that first received >2/3 pre-commits.
- **Flash-finality equivocation:** Signing two conflicting flash-finality certificates is treated as a GRANDPA-level equivocation with full slash.

### 3.3 Reorganization Depth

- **Maximum reorg depth:** Blocks beyond the GRANDPA finalized tip are subject to reorg. In practice, honest majority ensures finality within 2–3 blocks (~12–18s).
- **Long-range attacks:** Mitigated by validator set rotation and GRANDPA's mandatory pre-commit on the last finalized block of each session.

---

## 4. View-Change and Timeout Behavior

### 4.1 BABE Slot Timeout

If a designated block producer fails to produce a block within its slot:

- The slot is marked as **empty** (secondary VRF check determines if a secondary author may fill it).
- After `max_skipped_slots` (default: 10 consecutive empty slots), the validator set enters **slow-mode** and reduces expected throughput until a block is produced.

### 4.2 GRANDPA Round Timeout

Each GRANDPA round has a **timeout** that doubles on each failed round (exponential backoff):

- Round 0: 4 seconds
- Round N: 4 × 2^N seconds, capped at 60 seconds

If a round times out without finalizing, GRANDPA increments the round number and restarts the voting process. Validators that fail to vote within the timeout may be penalized via **reputation scoring** (off-chain, not slashable).

### 4.3 Flash Finality Timeout

- Flash finality has a strict 2-second deadline from block proposal.
- If the threshold is not reached within 2 seconds, flash finality is **abandoned** for that block.
- The block proceeds to standard GRANDPA finality without penalty.
- Consecutive flash-finality timeouts (>100 blocks) trigger a node operator alert suggesting network latency investigation.

---

## 5. Network Asynchrony Assumptions

### 5.1 Synchrony Model

The protocol is **partially synchronous**: it operates correctly in periods of asynchrony but requires eventual message delivery for liveness.

- **Safety (finality):** Never violated, even under full asynchrony — GRANDPA will not finalize conflicting blocks.
- **Liveness (progress):** Requires network synchrony for ≥1/3 of the time for GRANDPA rounds to complete.

### 5.2 Byzantine Fault Tolerance

- The protocol tolerates **f < n/3** Byzantine validators (where n is the active set size).
- With f ≥ n/3, liveness may stall but safety is never violated.

### 5.3 Eclipse and Network Partition Recovery

- If a node is eclipsed (>50% of its peer connections are adversarial), it may be served a minority fork. Upon reconnection to the honest majority, it will detect the GRANDPA finalized tip and **automatically reorganize** to the honest chain.
- During a network partition, the partition containing >2/3 of validators continues finalizing blocks; the minority partition halts.

---

## 6. Cross-VM Finality Binding

Within an X3 block, cross-VM operations (X3Native ↔ X3Evm ↔ X3Svm) are **atomically finalized** when the containing X3 block is finalized:

1. All cross-VM debits and credits within a single extrinsics set are applied in-order.
2. The supply-ledger invariant check runs **after** all intra-block operations.
3. If the invariant holds, the block is valid and all cross-VM operations within it are committed at finality.
4. There is no "partial finality" for cross-VM operations — either all succeed or the entire block is rejected.

---

## 7. Adversarial Tests

The following adversarial scenarios must be tested before mainnet launch:

| Scenario | Expected Behavior |
|---|---|
| Single BABE equivocation (two blocks, same slot) | Fork with lower VRF output discarded; equivocator slashed |
| Single GRANDPA equivocation (conflicting pre-commits) | Conflicting votes rejected; equivocator slashed |
| Long network partition (>1 hour) | Minority partition halts; majority continues; minority syncs on reconnect |
| Flash finality timeout (consecutive 100 blocks) | Standard GRANDPA fallback; no slash; operator alert |
| Eclipse attack on a single node | Node follows minority fork during eclipse; auto-reorgs on reconnection |
| >1/3 validators offline | Liveness stalls; safety maintained; no spurious finalization |

---

## 8. CI Gate

- **Gate name:** `finality-spec-gate`
- **Check:** This document exists at `docs/specs/finality_model.md` and all adversarial tests listed in §7 are present in the test suite (verified by a CI script that greps for test function names).

---

## 9. References

- [GRANDPA finality gadget specification](https://spec.polkadot.network/sect-finality)
- [BABE block production specification](https://spec.polkadot.network/sect-block-production)
- [Flash Finality crate](../../crates/flash-finality/)
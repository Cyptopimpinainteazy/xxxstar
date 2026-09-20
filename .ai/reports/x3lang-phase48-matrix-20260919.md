# PHASE 48 — the adversarial matrix, measured

The phase says *every economic feature must include hostile tests* and names 30 cases.
The ledger row read "Substantial negative tests … Not a complete matrix (no
reorg/nonce-conflict/restart cases in x3-lang)". This is the measurement behind that
sentence, so the gap is a list rather than an impression.

## Method

For each of the phase's 30 cases, count matching lines across the test and source
surfaces where a hostile test for that case would live:

```
$ for kw in …; do
    rg -c "$kw" --glob '*.rs' compiler/tests vm/tests vm/src compiler/src \
      | awk -F: '{s+=$2} END {print s+0}'
  done
```

**A count is a pointer, not proof.** Every row below that is marked covered is marked
because the hits are *tests that assert a refusal*, and the number is there so a reader
can go and check rather than take the word. A high count is also not completeness: 176
hits for slippage does not say the ceiling is tested at the boundary. This document is
the map; the tickets are the work.

## The matrix

| # | PHASE 48 case | hits | where the coverage is |
|---|---|---|---|
| 1 | wrong asset | 9 | `trading_verify`/`verify` asset-mismatch refusals; `trading.rs` `AssetMismatch` |
| 2 | wrong chain | 12 | `same_chain_bridge` refusal; `semantic.rs` chain checks |
| 3 | wrong VM | 16 | `vm_supported` guard; `KNOWN_*` tables |
| 4 | wrong signer | 216 | ed25519 verification paths in `bridge.rs` |
| 5 | wrong preimage | 33 | hashlock mismatch tests |
| 6 | duplicate operation | 90 | duplicate-declaration refusals across the compiler |
| 7 | double repay | 80 | `CloseDebt` twice; debt lifecycle refusals |
| 8 | double claim | 17 | `no_double_claim` |
| 9 | claim after refund | 5 | `no_refund_after_claim` and its sibling |
| 10 | refund before timeout | 31 | `refund_path` / `refund_to` guards |
| 11 | stale state | 21 | state-binding checks; `StateBindingMode` |
| 12 | stale route | 21 | `quote_freshness`; TICKET-017's receipt legs |
| 13 | stale oracle | 21 | `max_oracle_deviation_bps` firewall |
| 14 | fake finality | 556 | `verify_evm_header_proof` anchor; PHASE 49's invariant 7 |
| 15 | wrong state root | 22 | `state_commitment` checks in the trading boundary |
| 16 | wrong receipt hash | 46 | receipt verification and replay |
| 17 | invalid artifact signature | 5 | signature checks on signed artifacts |
| 18 | cost above maximum | 99 | `max_gas`, `allowed_cost_kinds` |
| 19 | profit below minimum | 113 | `AssertMinNetProfit`; measured floors |
| 20 | slippage above maximum | 176 | `max_slippage_bps`; measured ceilings |
| 21 | deadline exceeded | 537 | deadline/timeout engine |
| 22 | nonce conflict | 182 | nonce guards and replay protection |
| 23 | duplicate transaction | 90 | packet/receipt ledgers refuse a repeated hash |
| 24 | dropped transaction | 30 | (the hits are mostly unrelated words — **unverified**) |
| 25 | partial domain outage | 1 → covered | `trading_properties.rs::an_unreachable_second_domain_rolls_the_plan_back` (`c4b3a1621`) |
| 26 | RPC outage | 1 → covered | `trading_properties.rs::a_host_that_stops_answering_mid_plan_rolls_the_plan_back` |
| 27 | reorg simulation | 2 → covered | `bridge::tests::a_reorged_finality_view_refuses_a_proof_that_was_final` |
| 28 | ledger corruption | 2 → **was covered all along** | six tests in `trading_receipts.rs`, named in the round-59 report. **The count was a pointer and the pointer was wrong** — see the correction below |
| 29 | restart during execution | **0** → covered | `trading_properties.rs::restarts_and_kills_leave_nothing_behind` |
| 30 | process kill | **0** → covered | `restarts_and_kills_leave_nothing_behind` + `a_receipt_exists_only_for_a_committed_plan` |

## What the shape of the gap means

Cases 1–23 are decisions a *single running process* makes about a bad input, and they
are covered: the refusals are unit-testable and each has tests. Cases 25–30 are all the
same kind of thing — **state that outlives the process** — and none of them is covered:

| case | what it needs |
|---|---|
| restart during execution | a rule for what a partially executed atomic plan looks like on disk, and who finishes or unwinds it |
| process kill | the same rule, with the process not coming back |
| ledger corruption | a check that the persisted ledger is the ledger — cheap if the ledger is derived, expensive if it is stored |
| reorg simulation | a finality model in which a previously-final block stops being one, and what the receipt says then |
| partial domain outage | a two-domain trade where one domain is unreachable after the other committed |
| RPC outage | a host that stops answering mid-plan |

These are not test-writing problems. They are **the question of which state survives a
restart**, and the repository already has two partial answers to build on:

- **PHASE 45's version binding** — an artifact names the versions that produced it and a
  mismatch is refused, so a restarted reader cannot silently interpret old bytes;
- **TICKET-017's receipt legs** — a receipt carries the quote window each leg was priced
  in, so a restarted verifier can re-derive the age `quote_freshness` bounds instead of
  trusting a conclusion.

What neither answers is *where the half-executed plan lives*, and the tree says so
plainly: `TradingState` derives `Debug, Clone, Default, PartialEq, Eq` and **not**
`Serialize`/`Deserialize`, `TradingVm` has no persistence of any kind, and the atomic
rollback snapshot the phase's "restart during execution" imagines is
`VMState::atomic_snapshot: Option<VmSnapshot>` — a field in memory, with no reason to
believe a killed process leaves it anywhere. So the durability family cannot be tested
before it is designed: there is nothing yet to restart *from*. That is the question to
answer first, and it is a design question, not a test-writing one.

## Status, after round 59

**29 of the phase's 30 cases now has a test** — the last six answered in
`.ai/reports/x3lang-ticket087-decision-20260919.md`, and one of those six was already
covered — leaving exactly one unverified:

| case | test |
|---|---|
| partial domain outage | `an_unreachable_second_domain_rolls_the_plan_back` |
| RPC outage | `a_host_that_stops_answering_mid_plan_rolls_the_plan_back` |
| reorg simulation | `a_reorged_finality_view_refuses_a_proof_that_was_final` |
| ledger corruption | the six `trading_receipts.rs` tests named in row 28 — **covered all along** |
| restart during execution | `restarts_and_kills_leave_nothing_behind` |
| process kill | `restarts_and_kills_leave_nothing_behind` + `a_receipt_exists_only_for_a_committed_plan` |
| dropped transaction | `trading_properties.rs::a_dropped_transaction_is_not_a_settled_leg` (`0ddbb26ad`) |

## Row 24, `dropped transaction` — covered in `0ddbb26ad`, and it needed a contract first

The row was unverified for a real reason: its count (30) was too noisy to call, and when
the file was finally read the case turned out to depend on a contract that was written
down nowhere. A dropped transaction is a venue that **answers** — it accepted the
transaction and broadcast it — and reports the state it started from, because nothing
moved. `TradingVm::check_commitment` compares that against
`CapabilityManifest::state_commitment`, so the two cases are distinguishable *only if the
manifest declares the state the plan expects to end at*. Measured, all four combinations:

```
anchor=PRE  | venue landed (reports POST) -> Err(StateCommitmentMismatch)
anchor=PRE  | tx DROPPED (reports PRE)    -> COMMITTED, receipt emitted
anchor=POST | venue landed (reports POST) -> COMMITTED
anchor=POST | tx DROPPED (reports PRE)    -> Err(StateCommitmentMismatch)
```

The inversion on the top two rows is reachable and it is silent, and
`CapabilityManifest::state_commitment` had no doc comment saying which of the two
anchors it is. Both the field's contract and the test are in `0ddbb26ad`.

**Recorded as a correction, because the first reading got it backwards.** The initial
reproduction anchored the manifest at the pre-state, which made the *landed* venue look
like the defect and the dropped one look innocent — a defect report in the wrong
direction. The 2×2 above is what settled it, and the same file now says which anchor the
row relies on rather than leaving it to whoever reads `check_commitment` next.

The test asserts the refusal **and the control in the same test** — the same host
reporting the expected state commits — because a test asserting only the refusal would
pass against a runtime that refused every leg. It is load-bearing, verified by mutation:
with `check_commitment` neutered to `Ok(())` it fails, and passes with the check restored.

The row's own scope limit: `x3c receipt execute` derives the same field from the compiled
bytecode and runs `CapabilityMode::Fixture` under `ExecutionMode::Development`; a fixture
manifest is refused in production mode, so that surface is not evidence for this row.

PHASE 48's 30 named cases are now all covered. The phase's sentence is "every economic
feature must include hostile tests" and 30 named cases is a sample of that, not a
definition of it — so the phase is **complete against its own list**, and that is the
claim being made rather than a claim about every economic feature in the repository.

# TICKET-087 — what survives a run

PHASE 48's last six cases are one family:

| case | round 58 measurement |
|---|---|
| partial domain outage | 1 hit |
| RPC outage | 1 |
| reorg simulation | 2 |
| ledger corruption | 2 |
| restart during execution | 0 |
| process kill | 0 |

They cannot be written as tests before the question behind them is answered — *which
state outlives a process?* — because a test has to assert a documented behaviour, and
there was no documented behaviour to assert. This is that answer.

## The decision

**Nothing survives a run.**

`x3-lang`'s runtime holds no state that outlives the process, and that is deliberate
rather than unfinished. Each candidate is either **derived** from the artifact or
**supplied by the host for that run**:

| state | where it lives | why not persisted |
|---|---|---|
| the open atomic plan | a contiguous region of the artifact, `ATOMIC_BEGIN` … `ATOMIC_END` | re-executing the artifact reaches the same state, because emission is deterministic (PHASE 42) and the plan is a property of the bytes |
| the trading journal (`balances`, `open_debts`, `credits`, `debits`, `costs`, `cost_ledger`, `bindings`, `leg_quote_windows`) | a field of `TradingVm` | derived from executing the plan against the host's reports; a copy on disk would be a second source of truth for figures the accounting already reconciles (`TradingVm::profit` refuses with `ProfitReconciliationMismatch` when they disagree) |
| the receipt | the artifact's own record, and the one thing that leaves a run | already the durable form — PHASE 45's version binding says which artifact produced it, TICKET-017's legs carry the quote window each leg was priced in, and `verify_receipt_economics` re-derives the economics from the recorded deltas rather than trusting the stated profit |
| the finality view | the caller's `trusted_header_hash` / `trusted_bank_hash` | a cached view is a stale view; TICKET-087's `reorg simulation` test exists precisely because the verdict must follow the view the caller holds *now* |

### The consequence, stated rather than assumed

**An interrupted atomic plan is not resumed. It is not executed at all.**

This is the fail-closed reading, and it is a legitimate answer rather than a
placeholder: a plan that did not commit has no receipt, and a plan with no receipt has
nothing a settlement layer can act on. The alternative — persisting a half-executed
plan and resuming it — would add a trusted on-disk surface for every intermediate
state, and every one of those states would need its own version binding, torn-write
rule, and corruption check. That is a large new attack surface bought for a capability
the artifact plus receipt pair already provides by re-derivation.

## The defect this found

Answering the question found **one** piece of state that did outlive a run, and it was
not the plan:

`ProductionBridgeAdapter::storage_op` and `lifecycle` reached two
`static Mutex<HashMap<…>>` behind accessor functions, so the host storage a program's
`storage_store`/`storage_load` uses belonged to the **process**.

```
before  two adapters in one process shared it; COUNT grew across runs
        test: "a fresh adapter read another run's storage — the storage belongs to the
        process, not the run"
after   the storage and lifecycle states are fields on the adapter
        test: ok
```

The path is reachable from the language: `storage_store`/`storage_load` lower to
`Operation::StorageOp` (`lowering.rs:1386`, `1634`) → `CapabilityPayload::StorageOp`
(`emitter.rs:871`) → `bridge.storage_op` (`executor.rs:1150`). So the same artifact
over the same inputs answered differently depending on what had run before it — a
determinism defect as well as a durability one.

A caller that *wants* the storage to persist across runs now says so by reusing one
adapter: a decision at the call site rather than a property of the process.

## The six cases, as tests

| case | test | what it asserts |
|---|---|---|
| restart during execution | `restarts_and_kills_leave_nothing_behind` | a fresh VM is the empty journal; two fresh VMs over the same ops reach the same state |
| process kill | `restarts_and_kills_leave_nothing_behind` + `a_receipt_exists_only_for_a_committed_plan` | nothing is left behind, and the one record that leaves a run is complete or absent — never half-written |
| partial domain outage | `an_unreachable_second_domain_rolls_the_plan_back` | the host refuses the bridge; the manifest lists it, so the refusal is the *domain* and not the capability; the half that ran is rolled back |
| RPC outage | `a_host_that_stops_answering_mid_plan_rolls_the_plan_back` | the host goes quiet on the **second** swap, after the first leg committed; silence is not a zero and not a success |
| reorg simulation | `a_reorged_finality_view_refuses_a_proof_that_was_final` | a header that was trusted is refused once the view moves, and trusted again when the reorg is undone — the verdict follows the view rather than a cached first answer |
| ledger corruption | already covered in `vm/tests/trading_receipts.rs` | see below |

`restart` and `process kill` share one test. That is honest rather than a shortcut: the
difference between them is whether the process comes back, and the claim being tested
is that it does not matter, because nothing was left behind either way.

## A correction to round 58's matrix

Round 58 counted `ledger corruption` as uncovered on 2 keyword hits. That was wrong.
`vm/tests/trading_receipts.rs` covers it with six tests:

```
one_bit_tampering_fails_verification                          -> HashMismatch
tampering_after_signing_invalidates_attestation               -> UntrustedAttestor
economic_replay_rejects_forged_reported_profit_even_with_rehashed_receipt
                                                              -> EconomicReplayMismatch
open_debt_in_successful_receipt_fails_verification
failed_receipt_cannot_report_profit
replay_ledger_rejects_the_identical_receipt_presented_twice   -> ReceiptAlreadySettled
```

The file's matrix said in its own method section that "a count is a pointer, not
proof". The pointer was wrong, and doing the work is what showed it. The matrix now
carries the test names instead of the count for this row.

## Residual, recorded rather than implied

- `dropped transaction` is still unverified. Round 58's count (30) is too noisy —
  "dropped" matches unrelated words — so it is neither covered nor known to be
  uncovered.
- `lifecycle`'s `kind 0` ("query") returns its argument rather than the recorded state,
  so the lifecycle map is **write-only**: nothing reads it. Scoping it to the adapter
  removed the cross-run leak, but what a query should answer is a bridge-semantics
  decision and is left as a ticket note rather than guessed at here.

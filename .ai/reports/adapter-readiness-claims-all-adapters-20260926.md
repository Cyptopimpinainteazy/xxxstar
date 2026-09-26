# All fourteen swap adapters now declare the chain evidence they actually have

Date: 2026-09-26 (second pass)
Subsystem: `crates/x3-atomic-swap` swap adapters, matrix row `X3-CLAIM-003`
Continues: `.ai/reports/adapter-readiness-claims-20260926.md`

## What the first pass left open

The first pass corrected the EVM and SVM adapters and put the other twelve on
`security/adapter-readiness-claims-baseline.txt`. This pass closed them, after checking the one case
the checker's heuristic could have got wrong.

**`bitcoin_htlc.rs` was the check.** It has real RPC usage (13 matches) and reads receipts, so its
flags might have been honest and the marker heuristic wrong. They were not: its `finality_status`
returns a constant — no transport is consulted in that function (`external=False`,
`looks_fabricated=True`), and no adapter in the crate consults a chain there. Every one of the twelve
was the same shape.

| flag | declared true by | evidence against |
|---|---|---|
| `finality_proof` | all 12 | `finality_status` returns `block_number: 42`, `block_hash = sha256(42)`, `finalized: true` |
| `proof_ledger_integration` | bitcoin, cosmwasm, move_vm, substrate, wasm_l1 | `grep -n ledger <file>` matches only the flag itself and the test asserting it — nothing writes a ledger |

## Fix

* Twelve adapters: `finality_proof: false`; five of them also `proof_ledger_integration: false`, each
  with the reason in a comment.
* The tests that pinned the inflated numbers moved with them — this is where the false claim was
  locked in, and updating it is part of telling the truth, not weakening the test:
  `move_vm` 80 → 60, `bitcoin` 80 → 60, `substrate` 80 → 60, `cosmwasm` 80/100/80 → 60/80/60 (the
  IBC-enabled case included), `wasm_l1` 90 → 70, and the remaining seven 70 → 60. The eight tests that
  also asserted `missing_items().len()` moved by the same count.
* `assert!(score.finality_proof)` became `assert!(!score.finality_proof, "<reason>")` in nine files,
  and the ledger equivalent in four.
* `test_e2e_multi_vm_atomic_swap_lifecycle` asserted `overall_score >= 70`, which only held because of
  the over-claim; the same adapter set scores 58 honestly. It now asserts the property that threshold
  was standing in for — every adapter's scoreboard entry reports `finality_proof` missing, and the
  overall is below the old bar — instead of a number that measured the claim.
* `security/adapter-readiness-claims-baseline.txt` is **empty**: `check-adapter-readiness-claims.py`
  is a plain gate now, not a ratchet.

## Evidence

```
$ python3 scripts/ci/check-adapter-readiness-claims.py
check-adapter-readiness-claims: OK - no new over-claims; 0 known, all on the shrinking list

$ cargo test -p x3-atomic-swap
667 lib + 2 + 31 + 44 + ... all "0 failed"   (two runs: the first surfaced the missing_items() counts
                                              and the >= 70 assertion, both updated above)

$ python3 scripts/feature_matrix.py check
feature-matrix check PASS: 146 features
```

## Still open (recorded, not fixed)

`verify_claim` / `verify_refund` still accept any proof with a non-empty `tx_id` and a matching VM
type, and `finality_status` still returns a constant proof. Correcting the declarations stops the
scoreboard lying; it does not make those methods verify anything. That is the live X3<->EVM and
X3<->SVM work, and it is what the corrected scores now make visible: a set of adapters that must not
be trusted with a live swap until a real evidence path exists.

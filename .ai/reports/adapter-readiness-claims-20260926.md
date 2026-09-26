# Fourteen adapters declared chain evidence their own code fabricates

Date: 2026-09-26
Subsystem: `crates/x3-atomic-swap` swap adapters, matrix row `X3-CLAIM-003` (claims hygiene)
Directive: PRIORITY 6 (fake-security behaviour) and the claims-hygiene section — "do not claim live
execution from a simulation", "no fake proofs".

## Finding

`X3VmAdapter::readiness_score()` is a **self-declaration**: every flag set true adds 10 points, and
`AdapterScoreboard` — which the crate's own docs call "the mandatory scoreboard" — reports that
number as the adapter's readiness. So a flag is a claim, and the scoreboard was reporting claims the
same files contradict:

```
crates/x3-atomic-swap/src/evm_htlc.rs
    fn lock(..)  { let tx_id = Self::mock_tx_id(intent.intent_id, 0x01);
                   let block_number = 42;              // "Simulated block number"
                   raw_proof: vec![0x65,0x76,0x6d,0x01], // "evm\x01" - mock proof
    fn finality_status(..) { block_number: 42, block_hash: sha256(42), finalized: true }
    fn readiness_score(..) { event_proof_extraction: true, finality_proof: true,
                             rpc_indexer_support: true, proof_ledger_integration: true, ... }
```

Fourteen adapters were in that state (EVM and SVM with all four flags, twelve with one or two), so an
adapter with no transport at all read as production-ready. `x3vm_htlc.rs` is the model the others
should have followed: it declares those four false, and its non-simulation branch declares everything
false. `feature-matrix/claims-hygiene.toml` already recorded the audit's "simulated adapters /
placeholder proof verification" note; nothing enforced it.

## Fix

* **EVM and SVM adapters** — the two VM types the roadmap targets next (live X3<->EVM, live X3<->
  SVM) — now declare `event_proof_extraction`, `finality_proof`, `rpc_indexer_support` and
  `proof_ledger_integration` false, with the evidence in a comment. Their declared score drops by 40.
* `crates/x3-atomic-swap/tests/adapter_readiness_truth.rs` pins that for both, so it fails if a flag
  is re-declared.
* `scripts/ci/check-adapter-readiness-claims.py` reads every `*_htlc.rs`, finds the ones that build
  mock proofs, and fails when such a file declares any of the four evidence flags true.
  `security/adapter-readiness-claims-baseline.txt` lists the remaining twelve and may only shrink, so
  the rest of the work is visible rather than implied.
* Both are gates: `adapter readiness claims` and `test x3-atomic-swap adapter readiness`.

Measured load-bearing: re-declaring `finality_proof: true` on the EVM adapter makes the checker fail
(`FAIL crates/x3-atomic-swap/src/evm_htlc.rs: claims finality_proof while fabricating chain evidence`)
and the test fail (`the_evm_adapter_does_not_claim_the_evidence_it_fabricates ... FAILED`); the file
was restored.

## Evidence

```
$ cargo test -p x3-atomic-swap
667 lib + 2 + 31 + 44 ... all "0 failed"

$ python3 scripts/ci/check-adapter-readiness-claims.py
check-adapter-readiness-claims: OK - no new over-claims; 12 known, all on the shrinking list

$ bash scripts/local-ci.sh --jobs 4
85 gates, 0 failures   (the run before this that reported 85 caught the new test file unformatted —
                        format check FAILED; rustfmt applied, then all green)
```

## Still open

* Twelve adapters still over-claim. Correcting each means either telling the truth in
  `readiness_score` **and updating the scores its own tests pin** (`move_vm` 80, `cosmwasm` 80/100,
  `wasm_l1` 90), or giving the adapter a real evidence path. `bitcoin_htlc.rs` is the one to check
  first: it has real RPC usage and reads receipts, so its two flags may be honest and the checker's
  marker heuristic wrong for it.
* `verify_claim` / `verify_refund` in the corrected adapters still accept any proof with a non-empty
  `tx_id` and a matching VM type, and `finality_status` still returns a fabricated proof. Declaring
  the flags false stops the *scoreboard* lying; it does not make the methods verify anything. That is
  the live X3<->EVM / X3<->SVM work, and the row records it as such.

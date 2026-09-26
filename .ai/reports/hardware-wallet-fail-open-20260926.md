# A second fail-open verifier: the hardware wallet accepted bytes it never checked

Date: 2026-09-26
Subsystem: `crates/x3-wallet` (hardware signatures), matrix row `X3-OPS-011` (new)
Directive: ROADMAP PRIORITY 6 — wallet signatures, and "no production path may treat non-empty
bytes as a valid cryptographic attestation."

## Finding

`HardwareWalletEngine::verify_signature` refused an empty signature, refused a short one, then:

```rust
let mut verified = true;
if signature.recovery_id > 3 { verified = false; }
Ok(verified)
```

`tx_hash` was never read. The public key was never used. No ECDSA recovery, no signature check, no
binding to the transaction the signature was supposed to authorize. Any 64-byte blob with a
`recovery_id` in range came back `Ok(true)` — a verified hardware signature that had never been
verified. Two tests pinned that behaviour (`test_verify_signature_invalid_recovery_id` asserted
`Ok(false)` only for the out-of-range case), and `crates/x3-wallet` was cited by no matrix row and
run by no gate: `local-ci` gated `pallet-x3-wallet`, not the wallet's own signing crate.

## Fix — the directive's fail-closed branch

Priority 6 allows a real verification against a defined trust root **or** failing closed. There is
no device, no test vector and no recorded signing convention here: the message pre-hash, the
public-key encoding, the derivation-path binding and the recovery-id semantics are all undefined, so
implementing "verification" would mean inventing a convention and calling it one. `verify_signature`
therefore refuses every input, with an error that says verification is not implemented and that no
hardware signature is accepted. The two tests that pinned the old behaviour now assert the refusal,
including a new regression test for the exact case that used to pass (`test_verify_signature_refuses_a_well_shaped_but_unverifiable_signature`).

`approve_signature` still records `verified: true` on a device approval. That is *not* verification
and the field name says otherwise; it is carried in a `HardwareSignature` that derives `Encode`/`Decode`,
so renaming it is an interface decision the row records as open rather than one this change takes.

## Evidence

```
$ cargo test -p x3-wallet
test result: ok. 174 passed; 0 failed; 0 ignored

$ bash scripts/local-ci.sh --only 'test-x3-wallet,test-x3-gpu-validator-swarm,...'
PASS test x3-wallet 78s / PASS test x3-gpu-validator-swarm 204s / PASS clippy workspace 82s
PASS feature matrix check / readiness consistency / audit matrix freshness / script syntax

$ bash scripts/local-ci.sh --jobs 4
83 gates, 0 failures

$ python3 scripts/feature_matrix.py check
feature-matrix check PASS: 146 features, 19 warning(s)
```

`test x3-wallet` is new: the crate's 174 tests now run in a gate. `FEATURE_MATRIX.toml`'s
`feature_count_expected` moves 145 → 146 for the new row, and `scripts/x3_audit_matrix.py`
regenerated the derived artifacts (146 rows, COMPLETE=10, PARTIAL=61, STUB=11, NOT INTEGRATED=8).

## The pattern, and what it says about the rest

Two crates in one cycle, both with a real verifier sitting next to a fail-open one, both ungated:

| Crate | Real verification | The fail-open path | Gate |
|---|---|---|---|
| `x3-gpu-validator-swarm` | `ProofAggregator::verify_attestations` (registered key, 65-byte sig) | `UnifiedProof::validate()` accepted any non-empty signature | added |
| `x3-wallet` | `TransactionSigner::verify_signature` (Ed25519/Sr25519) | `HardwareWalletEngine::verify_signature` accepted any shaped blob | added |

Both were found by searching for the *shape* the directive names — a length check standing in for a
signature check — rather than by reading documentation, and neither had a gate to notice. Remaining
instances of the same shape in this repository have not yet been swept; that sweep is the next task.

## Still open

* `approve_signature`'s `verified: true`, and the two unreconciled signing conventions in the GPU
  crate (`pubkey || sig` in `GpuReceiptValidator` vs `r || s || recovery` in the aggregator), both
  recorded on their rows.
* `GpuReceiptValidator::verify_signature` is a real Ed25519 check that nothing calls.
* A convention (device + test vector + pre-hash) has to exist before hardware verification can be
  implemented rather than refused.

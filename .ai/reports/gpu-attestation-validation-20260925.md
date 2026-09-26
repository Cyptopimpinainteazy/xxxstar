# Any non-empty bytes counted as a GPU attestation, in a crate nothing ran

Date: 2026-09-25
Subsystem: GPU validator attestation (`crates/x3-gpu-validator-swarm`), matrix row `X3-GPU-013`
Directive: ROADMAP PRIORITY 6 — "No production path may treat non-empty bytes or magic header
prefix as a valid cryptographic attestation."

## Finding 1 — the structural validator called a placeholder an attestation

`UnifiedProof::validate()` checked `attestation.signature.is_empty()` and nothing else, so
`is_valid` was `true` for a proof whose only attestation carried `vec![1, 2, 3, 4]`. Two tests said
so out loud:

```
crates/x3-gpu-validator-swarm/src/proof_integration.rs
    let signature = vec![1, 2, 3, 4];      // test_unified_proof_creation
    assert!(proof.validate().is_valid);
    orchestra_evidence_to_unified_proof(&evidence, [7u8; 32], vec![9, 9, 9], ...)
    assert!(proof.validate().is_valid);
```

Meanwhile the crate's own verifier refuses those bytes:

```
proof_aggregator.rs: fn signature_from_bytes(..)  -> errors "expected 65, got 4"
proof_aggregator.rs: fn verify_attestations(..)   -> looks the validator up in `validator_pubkeys`,
                                                     rebuilds the signing message, verifies
```

So the aggregator path was real cryptography, and the *library* path beside it reported evidence
where a placeholder sat. A caller that reads only `ProofValidationResult::is_valid` — the shape the
type invites — was told a GPU validator had attested to a receipt when nobody had.

## Finding 2 — nothing ran this crate

`rg gpu-validator-swarm scripts/local-ci.sh` returned nothing, and the crate is not cited by
`FEATURE_REGISTRY.toml`, so `registry tests are gated` did not reach it either. Its 111 lib tests
plus its integration suites were green and unrun.

## Fix

* `crypto::SIGNATURE_LENGTH = 65` — one named constant for `r || s || recovery`, written by
  `SigningKey::sign(..).to_bytes()` and read by `ProofAggregator::signature_from_bytes`, so the
  producer, the verifier and the structural check cannot drift apart.
* `UnifiedProof::validate()` now errors on any signature that is not that shape, naming the expected
  and found lengths.
* The two tests that used placeholder signatures now use a well-formed one, with the comment saying
  why: authenticity is the aggregator's question, shape is this one's.
* `test x3-gpu-validator-swarm` added to `local-ci`, so the suite runs on every gate pass.
* Matrix `X3-GPU-013` records the finding, the fix and what is still open.

## What was deliberately *not* changed

An empty attestation set stays a warning. The aggregator's flow is submit-first — `test_submit_proof`
submits an unattested proof and expects it collected, and only `add_attestation` past
`finality_threshold` moves the entry — so turning "no attestations yet" into an error would have
broken a real, tested path. The property that makes the warning safe is pinned instead:
`an_unattested_proof_is_collected_but_never_finalized` asserts the proof sits at `Collecting`,
`consensus_count == 0`, `finalized == 0`, `byzantine_finalized == 0`, and that the validation
result says "No GPU attestations in proof" rather than passing silently.

## Evidence

```
$ cargo test -p x3-gpu-validator-swarm
111 passed; 0 failed   (lib)  + 7/6/5/5/20/5 across its integration suites

$ bash scripts/local-ci.sh --only test-x3-gpu-validator-swarm
PASS test x3-gpu-validator-swarm   132s

$ bash scripts/local-ci.sh --jobs 4
82 gates, 0 failures

$ bash scripts/local-ci.sh --only audit-matrix-freshness,readiness-consistency,feature-matrix-check,script-syntax
all PASS
```

Load-bearing, measured: reverting the length check to `is_empty()` turns exactly
`validate_refuses_a_signature_that_is_not_the_shape_the_verifier_reads` red
(`test result: FAILED. 110 passed; 1 failed`), and the file was restored.

## Still open

* `GpuReceiptValidator::verify_signature` is a real Ed25519 verification over the receipt hash that
  **no path calls**. It is defined and re-exported from `lib.rs`; nothing invokes it. Receipt-level
  signature checking is therefore dead code rather than a control. It also uses a second signing
  convention (`pubkey || sig`, 96 bytes) where the aggregator uses 65, and the two have never been
  reconciled — worth deciding before anything is built on either.
* Key ownership is still only checked where the public keys live (the aggregator). A caller of
  `UnifiedProof::validate()` now knows the shape is a signature, not that the signer was a validator;
  that distinction is documented in the code and the row rather than enforced by a trust root the
  library does not have.
* The directive's other PRIORITY 6 lead — production wallet code reporting signature verification as
  unimplemented — has not been audited yet.

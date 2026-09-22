# The runtime signer becomes a library, and the SVM leg is proven in strict posture

Date: 2026-09-22. Base: `origin/master` = `f1f857922`.

## 1. `X3RuntimeSigner` moved from `node/src` into a crate

`node/src/x3vm_runtime_signer.rs` (504 lines) is now `crates/x3-runtime-signer`, and
the node re-exports it, so every existing path still resolves:

```rust
// node/src/lib.rs
pub use x3_runtime_signer as x3vm_runtime_signer;
```

`node/tests/x3vm_live_lifecycle.rs`, `x3vm_evm_live.rs` and `x3vm_svm_live.rs` import
`x3_chain_node::x3vm_runtime_signer::X3RuntimeSigner` and did not change.

### Why it had to be a crate

The relayer's own refusal named the missing piece as a *path*:

> Signing a real extrinsic needs the runtime's `SignedExtra` and call encoding
> (`node/src/x3vm_runtime_signer.rs`) …

A binary crate's module is not something `crates/x3-relayer` can depend on, which is
why the signer was unreachable from the pipeline that needs it. A Substrate signed
extrinsic is a consensus object — the payload is `call ++ SignedExtra-as-encoded ++
additional-signed` — so there has to be exactly one implementation, and everything
that signs has to share it.

### Verified

```
cargo check -p x3-runtime-signer                     # Finished dev profile, no errors
cargo test  -p x3-runtime-signer                     # 3 passed
    tests::the_submit_proof_call_encodes_and_decodes_as_a_runtime_call  ok
    tests::rejects_unbound_local_intent                                 ok
    tests::binding_is_stable_and_cannot_be_repointed                    ok
cargo check -p x3-chain-node --tests                 # Finished in 3m 03s (re-export holds)
cargo clippy -p x3-runtime-signer --all-targets -- -D warnings   # clean
cargo fmt -p x3-runtime-signer -p x3-relayer -- --check          # clean
python3 scripts/check-workspace-membership.py        # OK — 0 crates in the gap
```

The first check was red in a useful way: the crate manifest was missing
`pallet-cross-chain-validator`, and the *strict SVM gate* caught it within minutes
because it builds `node/tests/x3vm_svm_live.rs`, which now goes through the new
crate.

## 2. What the extraction does **not** decide

`pallets/x3-settlement-engine/src/lib.rs:1420` (and `:1656` for the cross-domain
proof set) is the reason a relayer cannot simply submit:

```rust
let who = ensure_signed(origin)?;
let intent = SettlementIntents::<T>::get(intent_id).ok_or(Error::<T>::IntentNotFound)?;
ensure!(who == intent.maker || who == intent.taker, Error::<T>::NotAuthorized);
```

`crates/cross-vm-coordinator/src/settlement_submission.rs` already builds the exact
arguments (`SettlementSubmissionEnvelope::for_claim` / `for_refund`, with
`runtime_intent_id` + `purpose` + `CrossDomainProofSet`) and states that it
deliberately does not sign, "because the runtime pallet index, signed extensions,
nonce, era, and signer belong to the node client/runtime metadata layer" — which is
now the `x3-runtime-signer` crate. Its `requires_intent_party_signature()` returns
`true`.

So the missing piece is not a signer, it is an authority path: either the intent
party signs (and the relayer transports), or the pallet grows a delegation an
operator can be authorized for. That is a security decision, and the relayer's
refusal now says exactly that, with the line numbers, instead of pointing at a file
that no longer exists.

`crates/x3-relayer/src/submitter.rs` test `test_submitting_without_a_runtime_signer_is_refused`
now asserts the refusal names `intent.maker`, `x3-runtime-signer` and
`NotAuthorized`. 9 submitter tests pass.

## 3. The SVM leg in strict posture

`scripts/cross-domain-svm-gate.sh` has honoured `X3_STRICT_CROSS_DOMAIN_PROOFS`
since it was written, but **nothing in `local-ci.sh` ever set it**: the gate list had
`cross-domain EVM (strict posture)` and no SVM twin, so the SVM cross-domain leg was
only ever proven in the dev posture that allows unattested proof sets.

Run directly:

```
$ X3_STRICT_CROSS_DOMAIN_PROOFS=1 bash scripts/cross-domain-svm-gate.sh
test real_x3vm_svm_timeout_refund_atomic_lifecycle ... ok
PASS: real_x3vm_svm_timeout_refund_atomic_lifecycle
cross-domain-svm-gate: both X3VM<->SVM lifecycles passed [strict (/tmp/tmp.CwsfU7IJQD/strict.json)]
exit=0
```

And as a gate, so it is reproducible by one command:

```
$ bash scripts/local-ci.sh --cross --only "cross-domain-svm-(strict-posture)"
```

The entry carries the same `PATH="$HOME/.cargo/bin:$PATH"` prefix as the two other
SVM gates, because `cargo build-sbf` runs `cargo +1.89.0-sbpf-solana-v1.54` and
`+toolchain` needs the rustup shim.

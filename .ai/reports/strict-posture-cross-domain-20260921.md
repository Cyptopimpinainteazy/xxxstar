# Strict cross-domain posture, proven end to end (2026-09-21)

Merged as `#406` / `c6ccdcd50`.

## What was wrong

`pallets/x3-settlement-engine/src/lib.rs::require_verified_external_bundle` returns `Ok(())`
immediately when `AllowUnattestedCrossDomainProofs` is set. The flag is `true` on dev/local and
`false` on every network a validator can join (`node/src/chain_spec.rs::x3_chain_genesis`, and the
five `*_config()` callers). Both `scripts/cross-domain-evm-gate.sh` and
`scripts/cross-domain-svm-gate.sh` boot the dev chain — so the two gates that exist to prove the
cross-domain leg end to end were proving it under the one policy production never uses.

## What now exists

```
X3_STRICT_CROSS_DOMAIN_PROOFS=1 bash scripts/cross-domain-evm-gate.sh
X3_STRICT_CROSS_DOMAIN_PROOFS=1 bash scripts/cross-domain-svm-gate.sh
```

- `X3_TEST_CHAIN_SPEC` is how a spec reaches the harness; `--dev` stays beside `--chain` because it
  is what supplies the node's authority key (without it the node dies with `NetworkKeyNotFound`).
- `scripts/mainnet/strict-cross-domain-spec.py` flips exactly one field and refuses to write when the
  field is absent or is not the dev value, then reads the result back.
- The harness reads `X3SettlementEngine.AllowUnattestedCrossDomainProofs` over `state_getStorage`
  and requires `0x00` before the lifecycle starts.
- `scripts/local-ci.sh` gained the strict EVM gate.

## Evidence

Negative control (harness pointed at the untouched dev spec) — proves the check can fail:

```
panicked: X3_TEST_CHAIN_SPEC was given, so this run must be the strict posture
(allowUnattestedCrossDomainProofs = false), but the chain reports 0x01
FAIL: real_x3vm_evm_lock_claim_atomic_lifecycle
FAIL: real_x3vm_evm_timeout_refund_atomic_lifecycle
```

Strict, EVM: both lifecycles PASS (`49–62s` each). Strict, SVM: both PASS (`76.68s`, `80.18s`).
Dev EVM gate unchanged: 4/4 PASS. `make guard` 3 oks. `cargo clippy --workspace --all-targets --
-D warnings` PASS (4m15s).

## Finding 1 — the strict run does not exercise the external-leg rule

Both lifecycles submit **X3-native** bundles only, and `bundle_needs_verified_proof` never demands a
verified proof for an X3-native leg ("this chain verifies its own escrow"). So the strict run proves
the honest lifecycle does not depend on the permissive flag; it does not exercise
`require_verified_external_bundle` for an external domain.

That rule is covered at pallet-unit level (`the_rule_for_requiring_a_verified_proof_is_explicit`,
`a_bundle_matching_the_verified_proof_is_accepted`, plus two refusal tests). What no live path does is
produce the *input*: `submit_proof` verifies an EVM proof by walking a receipt to a receipts root that
`pallet-cross-chain-validator` stored from an authorized submitter (`AuthorizedSubmitters`), and
nothing in the repo builds that receipt MPT proof. On mainnet this means an EVM/SVM leg settles on
the attester set's root, not on an independent light client — worth stating plainly in the readiness
docs, and the next real piece of work on this path.

## Finding 2 — intermittent anvil rejection on the EVM lock

`real EVM lock: RpcError("JSON-RPC error -32602: Failed to decode transaction")`, ~45s into a run
(before the long waits), on the first EVM send of the gate.

- strict runs: 2 failures in 13
- dev runs: 0 failures in 6
- 6 further strict runs passed after the diagnostic landed

Not root-caused. `Transaction::sign` uses the workspace's canonical RLP (`rlp 0.6.1` strips leading
zeros for integers), so the obvious non-canonical-encoding candidates are ruled out by reading the
encoder. The RPC client's error now carries the request it answered (truncated), which is what the
next occurrence needs.

## Gate-path bugs fixed in the same change

Under a redirected `CARGO_TARGET_DIR` (the local CI's own configuration) the SVM gate looked for
`x3_atomic_swap.so` and `x3-svm-broadcast` only under the program's `target/`, and reported a
successful build as `missing after build-sbf`.

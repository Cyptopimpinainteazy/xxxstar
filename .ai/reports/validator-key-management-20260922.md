# Validator key management: what exists, what is not connected

Date: 2026-09-22. Base: `origin/master` = `c63ae274c`.

Audit of `X3-L1-002` (validator key management, P0, 30%) and `X3-XCHAIN-003`
(relayer framework, P0, 35%), the two weakest rows after the consensus work. One
tooling defect was real and is fixed; the rest of this is what the records now say
instead of "needs hardened ceremony, rotation, recovery, and operator runbooks".

## Fixed: the key-injection tool depended on a tool that is not installed

`scripts/testnet/inject-keystore.sh` derived each validator's public keys by shelling
out to `subkey` and then hand-wrote the keystore files. `subkey` is not installed on
these boxes and is not part of this repository, so the documented fresh-validator
path could not run — the same defect the 7-validator launcher had.

It now injects through the node (`keys insert`), which is the code that reads the
keystore at startup, and it finds a built node the way the other scripts do:

```
$ NODE_BIN=… X3_SPEC=…/fresh/generated/x3-testnet-plain.json \
    bash scripts/testnet/inject-keystore.sh /tmp/x3-inject-test 1
injected validator-1 session keys into /tmp/x3-inject-test/chains/x3_chain_testnet/keystore
  aura:    5FvkxhV6Yy2hLf5T3foTtxqjZkc1vHW9sogvpU7wgTM5GpJf
  grandpa: 5DGfN6waa5BgbjfLpPFoT8jqDrqyXskJK5x1roB93jALSn4t
$ x3-chain-node keys list --keystore-path …/keystore
  aura: 5FvkxhV6Yy2hLf5T3foTtxqjZkc1vHW9sogvpU7wgTM5GpJf
  grandpa: 5DGfN6waa5BgbjfLpPFoT8jqDrqyXskJK5x1roB93jALSn4t
```

## Measured: keystore-only keys do drive authoring

`scripts/testnet/run-fresh-validators.sh` carried the note "verified 2026-09-04:
file-only keystore injection does not drive Aura", and it exists as a second
launcher shaped around `X3_DEV_SEED` because of that belief. Re-measured:

```
$ x3-chain-node keys insert --key-type aura    --seed //Alice --keystore-path $BASE/chains/x3_chain_local3/keystore
$ x3-chain-node keys insert --key-type grandpa --seed //Alice --keystore-path $BASE/chains/x3_chain_local3/keystore
$ env -u X3_DEV_SEED x3-chain-node --chain chain-specs/x3-local3-current-plain.json \
    --base-path $BASE --validator --force-authoring … 
keystore-only (no X3_DEV_SEED): head (starting) -> 9       # ~45s, i.e. authoring from the start
```

So the dev-seed path is a convenience, not the only mechanism. The note is corrected
where it lives.

## Found: key rotation exists twice and is connected zero times

* `node/src/authority.rs` — `SessionKeys`, `KeyRotationSchedule::should_rotate`,
  `schedule_next_rotation`, `set_pending_keys`, `consume_pending_keys`, `rotate_keys`,
  `validators_needing_rotation`, plus unit tests. **No caller anywhere in the
  workspace**: nothing schedules a rotation at a block, nothing consumes pending
  keys, and nothing submits the new keys on chain. It is a library with tests and no
  wiring.
* `pallets/x3-custody` — an on-chain `ValidatorKeyRegistry` with `rotation_due_at`,
  `schedule_rotation`/`rotate` events and tests. Real, and unconnected to the above:
  nothing drives the registry from the node-side schedule, or the node-side schedule
  from the registry.

The honest statement is therefore "rotation is implemented twice and has never run
end to end", not "rotation is missing".

Also measured: nothing registers a new validator's session keys on a running chain.
`session.setKeys` is callable in the runtime (`pallet-session` is in the runtime), and
no script or node command calls it — so onboarding today means editing genesis or
re-running a local launcher, which is not how a live network takes a new validator.

## The relayer row was accurate, and now says why

`X3-XCHAIN-003`'s blocker ("Multi-validator relayer quorum incomplete") turned out to
be *right*, which is worth recording after a run of stale rows. The relayer says so
itself:

```rust
// one signature and quorum enforcement belongs at the aggregator layer.
svm_required_signatures: 1,
```

…and no aggregator exists in this workspace. Quorum *verification* is real
(`crates/x3-validator-attestation`, `crates/x3-bridge/src/cross_chain_proofs.rs` has
exactly-at / one-below / duplicate-signature threshold tests), but nothing in the
relayer produces one. On top of that it cannot submit at all — the settlement engine
requires the intent's maker or taker — so the row's path now points at the production
crate (`crates/x3-relayer`) rather than the second watcher implementation in
`crates/x3-atomic-swap/src/relayer.rs`, which is what the node-side swap tests use.

## The rows now

| row | before | after | what changed |
| --- | --- | --- | --- |
| `X3-L1-002` validator key management | 55 / 30 / 30, paths `["scripts"]` | **75 / 60 / 40**, real paths | the node derives and stores keys itself; injection is fixed and verified; keystore-only authoring measured; blockers are rotation-unwired, no on-chain onboarding, no ceremony/backup/recovery/HSM, no audit |
| `X3-XCHAIN-003` relayer framework | 65 / 40 / 35, path = atomic-swap watcher | **70 / 50 / 35**, production crate + quorum verifiers | blocker confirmed and sharpened: single-key signing, no aggregator, cannot submit, no live run |

# The gateway's withdrawal id was XOR, not a hash

Date: 2026-09-22. Base: `origin/master` = `a5cb237f8`.

Reached while auditing `X3-XCHAIN-008` ("General external gateway"), which cited
`crates/x3-gateway` — the REST/GraphQL indexer service — while neither the gateway
crate nor the gateway pallet appeared anywhere in the matrix. The records pointed
away from the code, which is how this survived: the same mis-mapping as the BTC row
two turns ago.

## The defect

Both copies of `derive_withdrawal_id` — the pallet's, used by the
`request_withdrawal` extrinsic (call_index 5, so runtime code), and the off-chain
crate's — did this:

```rust
let mut out = x3_asset_id;
for (idx, byte) in recipient.iter().enumerate() { out[idx % 32] ^= *byte; }
for (idx, byte) in amount.to_be_bytes().iter().enumerate() { out[idx] ^= *byte; }
for (idx, byte) in block.to_be_bytes().iter().enumerate() { out[idx] ^= *byte; }
out
```

XOR is commutative and self-inverse, so a recipient whose bytes repeat at the same
32-byte slot cancels itself. `"A" * 64` and `"B" * 64` derive the **same** id — this
is asserted in the new test rather than argued. That id is:

* the key of the pallet's `Withdrawals` map (a colliding request overwrites an
  existing record, losing its `burned`/`released` state),
* the value in the `WithdrawalRequested` event, and
* the key of the relayer's processed-withdrawal set (`crates/x3-relayer/src/main.rs`
  inserts every id it sees), where a collision means the second withdrawal is treated
  as already released.

## The fix

One derivation, defined once and mirrored:

```
blake2_256("x3-crosschain-gateway-withdrawal-v1" || asset_id
           || len(recipient) as u64 LE || recipient
           || amount as u128 LE || block as u64 LE)
```

* `x3_crosschain_gateway::gateway_withdrawal_id` is the definition, next to
  `gateway_attestation_statement`, which already used domain-separated Blake2b for
  exactly this reason.
* the pallet derives the same preimage with `sp_io::hashing::blake2_256`.
* the pallet's test helper `expected_withdrawal_id` now **calls the crate** instead
  of repeating the derivation. It used to repeat it — as XOR — so both sides were
  wrong in the same way and 47 tests passed on top of the bug. `the_two_derivations_agree`
  is the test that keeps them from drifting again, and it is what caught the mistake
  described below.
* `spec_version` 12 → 13: ids derived after the upgrade differ. Nothing is migrated;
  stored ids stay as they are.

## Proof

```
cargo test -p x3-crosschain-gateway --lib      # 20 passed
  recipients_that_collided_under_xor_get_different_withdrawal_ids   ok
      (asserts the old algorithm collides for "A"*64 vs "B"*64, and the new one does not)
  every_field_of_a_withdrawal_changes_its_id                        ok
cargo test -p pallet-x3-crosschain-gateway --lib    # 58 passed
  the_two_derivations_agree                       ok
cargo check -p pallet-x3-crosschain-gateway --no-default-features   # no new warning
cargo fmt / clippy -p x3-crosschain-gateway -p pallet-x3-crosschain-gateway --all-targets -- -D warnings
```

## A trap worth recording

The first version of the pallet fix used `frame_support::Hashable::blake2_256`. That
is **not** Substrate's `blake2_256` in this build — measured on `b"abc"`:

```
frame_support::Hashable::blake2_256 : b9f1f266942f471d…  (not standard)
sp_io::hashing::blake2_256          : bddd813c63423972…  (standard Blake2b-256)
python hashlib.blake2b(digest_size=32) : bddd813c63423972…
```

The agreement test failed immediately with two different digests, which is the whole
point of having it. `sp_io::hashing::blake2_256` is the one that matches both the
off-chain implementation (`blake2::Blake2b::<U32>`) and the standard. `sp-io` became
a production dependency of the pallet for it.

## The re-attestation

The pallet is runtime code, so the attested bytes move:

```
./scripts/update-runtime-hashes.sh          # two from-scratch srtool builds agree
compact     8,426,935 bytes  0x871c8feb70f6068a1ba9a98e1a49e23b48e5c895ea7bf0a393c7274fa5490c63
compressed  1,444,353 bytes  0x5a8200f4890d1630dfc2394f6119058a9159c56c545aa2fb8a896e8d5067f5e2
recorded_revision 5e0f86760 -> 9fe02ac37 ; runtime_version x3-chain-12 -> x3-chain-13
```

(The first attempt failed with `cannot update the lock file /build/Cargo.lock
because --locked was passed` — adding `sp-io` as a production dependency of the
pallet changes the lock graph, so `Cargo.lock` is part of this commit.)

## Row

`X3-XCHAIN-008` now cites `crates/x3-crosschain-gateway` and
`pallets/x3-crosschain-gateway` (80/75/70, from 70/55/65), with blockers that are
real: activation is governance-guarded, nothing has run against a public external
chain, the id change takes effect only after this runtime is deployed, and there is
no audit of either side.

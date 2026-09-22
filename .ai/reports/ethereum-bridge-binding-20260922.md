# The lock-mint bridge signed nothing about the money

Date: 2026-09-22. Base: `origin/master` = `4bcc84f65`.

Audit of `X3-XCHAIN-004` (Ethereum bridge adapter, P0, 38%). The row's blocker
("needs external-chain live proof and signer policy") is half stale — the external
proof exists — but reading the adapter found two real defects on the funds path.

## 1. The message validators signed was not a hash of anything

```rust
// Create message hash: keccak256(deposit_id || amount || token || recipient)
let mut hash = [0u8; 32];
let id_bytes = deposit_id.as_bytes();
for (i, &byte) in id_bytes.iter().enumerate().take(32) {
    hash[i] ^= byte;
}
```

No keccak, no amount, no token, no recipient — the first 32 bytes of the deposit id
XORed into zeros. So the value the 5-of-7 signatures authorized was trivially
derivable from the id and bound none of the economic terms: a signature set
collected for `deposit_7` authorized `deposit_7` with any amount, any token and any
destination. The comment described the intended hash; the code did something else.

Now: `keccak256(deposit_id || amount_le || token || x3_recipient)`, and the test
requires the hash to change when the recipient or the amount changes, and to differ
from the XOR artifact it used to be.

## 2. The mint went wherever the caller said

`BridgeDeposit` had no X3 recipient field, and `execute_mint(message_id,
x3_recipient, x3_block)` minted to the `x3_recipient` the *caller* passed. Combined
with (1) — signatures that do not bind a recipient — anyone able to execute a mint
with a valid signature set could send a confirmed deposit to an account of their
choosing, and every signature would still verify.

`lock_on_ethereum` now takes the destination and stores it on the deposit (empty
refused), and `execute_mint` refuses a recipient that is not the deposit's:

```
Mint recipient 5FHneW… is not the deposit's recorded recipient 5Grwva…
```

## Proof

```
cargo test -p x3-bridge --lib          # 122 passed
  bridge_message_hash_covers_the_economic_terms              ok
  mint_to_an_account_other_than_the_recorded_recipient_is_refused   ok
cargo fmt -p x3-bridge -- --check ; cargo clippy -p x3-bridge --all-targets -- -D warnings
```

Negative control: with only the recipient check removed, that test fails
(`FAILED … 0 passed; 1 failed`) and passes again once restored.

Three existing tests failed on the first run of the binding — `test_execute_mint`,
`test_burn_wrapped`, `test_bridge_replay_protection` all minted to `0xAlice_X3` after
being handed a default recipient at lock. Their lock calls now name `0xAlice_X3`, so
they keep testing what they were written to test instead of the binding being
softened to accommodate them.

## What the bridge gets right

The multisig itself is real, which is worth saying after finding the hash defect: a
65-byte signature, `secp256k1_ecdsa_recover` over the message hash, recovered address
compared against the registered validator address, and double-signing refused per
message. And the external-chain proof the row called missing exists — anvil-backed
EVM lifecycles in the cross-domain gates (dev and strict posture), with receipts-trie
inclusion and typed receipts verified through `x3-verification-router`.

## What the row now says

`X3-XCHAIN-004` moves to implemented 75 / tested 60 / mainnet-ready 45, and its
blockers are the ones that are real: the adapter has no caller outside its own module
(so none of these checks run in production yet), `burn_wrapped` debits the account
passed to it without verifying who is asking, nothing has run against a public chain,
the 5-of-7 production key policy is X3-L1-002's gap, and there is no audit.

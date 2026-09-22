# Why the EVM leg failed twice in thirteen runs: non-minimal `r` and `s`

Date: 2026-09-22. Base: `origin/master` = `626dff783`.

## The flake

`/tmp/strict-run-4.log` (a strict-posture EVM gate run) died at
`node/tests/x3vm_evm_live.rs:468`:

```
real EVM lock: RpcError("JSON-RPC error -32602: Failed to decode transaction")
```

Two failures in thirteen runs, always on the first EVM lock. Anvil's
`eth_sendRawTransaction` says "Failed to decode transaction" when the raw bytes
are not a valid transaction, so the bytes the workspace signed were not always
valid.

## The cause

`crates/x3-atomic-swap/src/ethereum_tx.rs` (the one EIP-155 signer the EVM
adapters share — `crates/external-chains/src/signer.rs` calls it) encoded the
signature like this:

```rust
let (r_bytes, s_bytes) = signature.split_bytes();   // 32-byte fixed width
...
stream.append(&r);
stream.append(&s);
```

RLP carries `r` and `s` as *integers*, not as buffers: a leading zero byte is not
the canonical encoding of the value it stands for, and a node that enforces
canonical integers rejects the whole transaction. The buffer is 32 bytes wide for
every signature, so `r[0] == 0` (or `s[0] == 0`) happens in roughly one signature
in 128 — which is exactly the observed rate.

## Reproduced, then fixed

`crates/x3-atomic-swap/tests/evm_tx_minimal_integers.rs` walks fixed `(key,
nonce)` pairs — no RNG, so the result is deterministic — decoding each signed
transaction with the `rlp` crate, asserting that the scan *contains* a
leading-zero signature (so the test cannot pass vacuously), and that no signed
transaction carries a non-minimal `r` or `s`. Before the fix:

```
nonce 83: r is encoded with a leading zero byte, which no EVM node accepts:
0xf86a53843b9aca00830186a09470997970c51812dc3a010c7d01b50e0d17dc79c88084deadbeef82f4f6
  a0 006114f1e9dc786907f731d20ca0b3ecc68e29be8d7c0c12fcc5faec57bf1add
  a064ece41e4e36e8f433448f27ace8ddb8e2f535f103078919bc7d282b361ab683
```

That exact transaction against a real node (`anvil 1.8.3`, chain id 31337, the
dev key that produced it, `anvil_setNonce` to 83 so the nonce is not what fails):

```
$ curl … eth_sendRawTransaction 0xf86a…a0 006114f1…
{"jsonrpc":"2.0","id":1,"error":{"code":-32602,"message":"Failed to decode transaction"}}
```

After the fix the signer emits the value (`9f 6114f1…`, 31 bytes, and the outer
list is one byte shorter):

```
$ cargo test -p x3-atomic-swap --features std --test evm_tx_minimal_integers
the scan contained a leading-zero signature at nonce 83: 0xf869…82f4f69f6114f1…
test result: ok. 1 passed; 0 failed

$ curl … eth_sendRawTransaction 0xf869…9f6114f1…
{"jsonrpc":"2.0","id":1,"result":"0x3ea81e1dd05896b91ecc1a9d39951dd0f0123c3e8b762f19bb309a234e2f8858"}
```

The node accepting the transaction with the account's nonce at 83 also proves the
recovered sender is that account: a different sender would have failed the nonce
check.

## The change

`rlp_integer(bytes) -> &[u8]` strips leading zero bytes (zero → the empty byte
string, which RLP defines for zero), and the signer appends that instead of the
fixed-width buffer. It is `#[cfg(feature = "std")]` with its only caller: a
`no_std` build has no signer, and an unused private function is a `dead_code`
error in the runtime's wasm build.

Verified:

```
cargo test  -p x3-atomic-swap --features std --lib        # 694 passed
cargo test  -p x3-atomic-swap --features std --test evm_tx_minimal_integers   # 1 passed
cargo check -p x3-atomic-swap --no-default-features       # ok, no new warning mentions ethereum_tx
cargo clippy -p x3-atomic-swap --features std --all-targets -- -D warnings
cargo fmt -p x3-atomic-swap -- --check
```

This signer is on every EVM send path — HTLC locks and claims, Arbitrum
`sendTxToL1`, contract deploys — so the fix is not only about the gate: any EVM
transaction whose signature happened to have a leading zero byte was rejected by
the node.

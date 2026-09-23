# The first live Bitcoin run: regtest, against a real node

Date: 2026-09-22
Scope: `pallets/x3-settlement-engine` (BTC SPV), `scripts/btc/`, readiness records

## Why this exists

Two readiness rows said the same thing for months: *"No live Bitcoin run of any kind — not
regtest, not testnet, not mainnet."* Every BTC fixture in `tests.rs` was hand-written — a
header whose fields a person chose, checked against a merkle root a person chose. That is a
test of our own arithmetic, not of our agreement with Bitcoin.

## What was done

Bitcoin Core **v28.1.0** was installed and verified against the project's published
`SHA256SUMS` (`bitcoind --version` → `v28.1.0`), a regtest node was started, 121 blocks were
mined, a wallet transaction was sent and confirmed, and the block, transaction and merkle path
were captured:

```
curl … https://bitcoincore.org/bin/bitcoin-core-28.1/SHA256SUMS
curl … bitcoin-28.1-x86_64-linux-gnu.tar.gz
sha256sum -c <(grep "x86_64-linux-gnu.tar.gz" SHA256SUMS)   # OK
bitcoind -regtest -datadir=/tmp/btc-regtest -daemon
bitcoin-cli -datadir=/tmp/btc-regtest generatetoaddress 120 "$ADDR"
bitcoin-cli -datadir=/tmp/btc-regtest -rpcwallet=x3 sendtoaddress …      # then 1 more block
./scripts/btc/capture-regtest-spv.py --txid 91cbaa84…9e15
```

The capture script **refuses to emit anything the node does not agree with**: it recomputes
the merkle root and compares it to the block's `merkleroot`, and recomputes the header hash
and compares it to the block hash. It also emits the successor block, because a header can
only be *extended* by the header that links to it.

Artifact: `.ai/reports/btc-regtest-capture-20260922.json`.

| field | value |
| --- | --- |
| chain | regtest |
| block | height 121, `2771f546…33af` |
| bits | `0x207fffff` (regtest's `powLimit`, which is also the dev runtime's) |
| transaction | `91cbaa84…9e15`, index 1 of 2 |
| successor | height 122, `4f86134b…c704`, same `nBits`, linking back to 121 |

## What the tests now prove with it

Four tests in `pallets/x3-settlement-engine/src/tests.rs`, on that capture:

| test | claim |
| --- | --- |
| `a_real_bitcoin_block_hashes_to_the_hash_the_node_reported` | `compute_btc_block_hash` reproduces the hash Bitcoin Core reports for block 121 — the wire layout checked against an authority, not against a second copy of our opinion |
| `a_real_bitcoin_merkle_path_verifies_against_a_real_block` | the pallet's direction-aware merkle walk reconstructs the root the node put in the block, and a tampered sibling does not |
| `a_real_regtest_chain_anchors_and_extends_through_the_pallets_rules` | block 122's parent link is block 121's hash; 121 cannot be submitted without an anchor; anchoring it works; 122 then extends it at height 122 |
| `a_real_bitcoin_transaction_proof_is_rejected_until_its_block_is_anchored` | a real transaction's SPV proof is `Ok(false)` while unanchored and `Ok(true)` once its block is anchored |

`cargo test -p pallet-x3-settlement-engine` → **148 + 23 passed, 0 failed**.

## A real defect the live run exposed

The first attempt failed, and the reason matters for anyone writing the relayer:

**A segwit transaction's raw bytes do not hash to its txid.** The node's
`getrawtransaction` returns the marker, flag and witness stack; a txid covers none of that, so
`dsha(raw)` is the *wtxid*. The pallet checks `tx_hash == dsha(tx_bytes)` and walks the merkle
path over txids — so an SPV proof has to carry the **witness-stripped serialization**. A
relayer that forwards what the node hands it would fail every segwit deposit, and it would
fail with "invalid proof" rather than anything that names the cause.

Recorded three ways so it cannot be re-learned the hard way: the capture script emits both
`raw_tx_hex` and `raw_tx_stripped_hex` and asserts the stripped form hashes to the txid; the
test asserts both hashes and names the wtxid; and `submit_btc_proof`'s doc comment says which
one the call takes.

## What this does not claim

Regtest is not testnet. The difficulty is one, the coins are worthless, and the chain is one
node on one machine. No header from a public network has been anchored, the deposit and
withdrawal paths have still never moved a coin, and nothing here says anything about
Bitcoin's real mempool, reorgs or fee market. What it does say is that the pallet's rules now
agree with a real Bitcoin implementation on real Bitcoin bytes — which was not true before.

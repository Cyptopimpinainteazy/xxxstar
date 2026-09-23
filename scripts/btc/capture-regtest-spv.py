#!/usr/bin/env python3
"""Capture a real Bitcoin block, transaction and merkle path for X3's SPV path.

The BTC rows in `FEATURE_REGISTRY.toml` and `feature-matrix/cross-chain.toml`
said "no live Bitcoin run of any kind" for months. The fixtures in
`pallets/x3-settlement-engine/src/tests.rs` were hand-written: a header whose
fields a person chose, proved against a merkle root a person chose. Nothing in
the repo had ever put *actual* Bitcoin bytes through the pallet's rules.

This produces exactly those bytes from a running node, so the test that uses
them is checking against something a Bitcoin implementation agreed to:

    # terminal 1
    bitcoind -regtest -datadir=/tmp/btc-regtest -daemon
    bitcoin-cli -datadir=/tmp/btc-regtest createwallet x3
    ADDR=$(bitcoin-cli -datadir=/tmp/btc-regtest -rpcwallet=x3 getnewaddress)
    bitcoin-cli -datadir=/tmp/btc-regtest generatetoaddress 101 "$ADDR"
    TXID=$(bitcoin-cli -datadir=/tmp/btc-regtest -rpcwallet=x3 sendtoaddress \
             "$(bitcoin-cli -datadir=/tmp/btc-regtest -rpcwallet=x3 getnewaddress)" 1.0)
    bitcoin-cli -datadir=/tmp/btc-regtest generatetoaddress 1 "$ADDR"   # confirm it

    # then
    ./scripts/btc/capture-regtest-spv.py --txid "$TXID"

It prints the header, the merkle path, the txid and the block hash, and checks
its own merkle computation against the node's `merkleroot` and its own header
hash against the node's block hash before printing anything. Byte order:
Bitcoin *displays* hashes reversed; the values written here (`header_hex`,
`merkle_path_hex`) are the wire/internal bytes the pallet's `H256` values are,
which is also what `compute_btc_block_hash` returns. `block_hash` is kept in
display order because that is the string a block explorer shows.

Regtest target is `0x207fffff`, which is exactly the `powLimit` the pallet uses
for dev chains, so a regtest header anchors on a dev chain and on nothing else.
A testnet or mainnet anchor needs a header from that network.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import subprocess
import sys


def dsha(b: bytes) -> bytes:
    return hashlib.sha256(hashlib.sha256(b).digest()).digest()


def strip_witness(raw: bytes) -> bytes:
    """Return the transaction's non-witness serialization.

    This matters more than it looks. A block's merkle root is built from *txids*,
    and a txid is the double SHA-256 of the transaction **without** the witness —
    for a segwit transaction the raw bytes include a marker/flag and a witness
    stack that the txid does not cover. So `dsha(raw_tx)` is the *wtxid*, and any
    SPV path that checks `tx_hash == dsha(tx_bytes)` has to be handed the stripped
    serialization. The pallet's `verify_btc_settlement_proof` does exactly that
    check, so a relayer that forwards the node's raw bytes would fail a segwit
    deposit. Emitting both here makes that visible instead of mysterious.
    """
    if len(raw) < 10 or raw[4] != 0x00:
        return raw  # not segwit: the raw bytes are already the txid preimage

    def varint(b: bytes, i: int) -> tuple[int, int]:
        """Read a Bitcoin CompactSize; returns (value, index just past it)."""
        first = b[i]
        if first < 0xFD:
            return first, i + 1
        if first == 0xFD:
            return int.from_bytes(b[i + 1:i + 3], "little"), i + 3
        if first == 0xFE:
            return int.from_bytes(b[i + 1:i + 5], "little"), i + 5
        return int.from_bytes(b[i + 1:i + 9], "little"), i + 9

    out = bytearray(raw[:4])          # version
    i = 6                             # skip the marker and flag
    n_in, end = varint(raw, i)
    out += raw[i:end]
    i = end
    for _ in range(n_in):
        out += raw[i:i + 36]          # outpoint
        i += 36
        n, end = varint(raw, i)       # script length, kept byte-exact
        out += raw[i:end]
        i = end
        out += raw[i:i + n]
        i += n
        out += raw[i:i + 4]           # sequence
        i += 4
    n_out, end = varint(raw, i)
    out += raw[i:end]
    i = end
    for _ in range(n_out):
        out += raw[i:i + 8]           # value
        i += 8
        n, end = varint(raw, i)
        out += raw[i:end]
        i = end
        out += raw[i:i + n]           # scriptPubKey
        i += n
    for _ in range(n_in):             # the witness stack, dropped entirely
        n_items, i = varint(raw, i)
        for _ in range(n_items):
            item_len, i = varint(raw, i)
            i += item_len
    out += raw[i:i + 4]               # locktime
    return bytes(out)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--datadir", default="/tmp/btc-regtest")
    ap.add_argument("--txid", required=True, help="a confirmed transaction to prove inclusion of")
    ap.add_argument("--search-blocks", type=int, default=25)
    ap.add_argument("--out", default="/tmp/btc-regtest-capture.json")
    args = ap.parse_args()

    bitcoin_cli = shutil.which("bitcoin-cli") or "/tmp/btc-core/bitcoin-28.1/bin/bitcoin-cli"
    if not shutil.which(bitcoin_cli) and not bitcoin_cli.startswith("/"):
        print(f"bitcoin-cli not found ({bitcoin_cli})", file=sys.stderr)
        return 2

    def cli(*a: str) -> str:
        r = subprocess.run([bitcoin_cli, f"-datadir={args.datadir}", *a],
                           capture_output=True, text=True)
        if r.returncode != 0:
            print(f"bitcoin-cli {' '.join(a)}: {r.stderr.strip()}", file=sys.stderr)
            sys.exit(2)
        return r.stdout.strip()

    info = json.loads(cli("getblockchaininfo"))

    # No -txindex on a plain regtest node, so walk back from the tip.
    block_hash = block = None
    for height in range(info["blocks"], max(info["blocks"] - args.search_blocks, 0), -1):
        candidate = cli("getblockhash", str(height))
        b = json.loads(cli("getblock", candidate, "1"))  # verbosity 1: tx list is txids
        if args.txid in b["tx"]:
            block_hash, block = candidate, b
            break
    if block is None:
        print(f"txid {args.txid} is not in the last {args.search_blocks} blocks",
              file=sys.stderr)
        return 1

    header = bytes.fromhex(cli("getblock", block_hash, "0"))[:80]
    if len(header) != 80:
        print("node returned a block shorter than a header", file=sys.stderr)
        return 1

    reverse = lambda h: bytes.fromhex(h)[::-1]
    index = block["tx"].index(args.txid)
    layer = [reverse(t) for t in block["tx"]]
    path, i = [], index
    while len(layer) > 1:
        if len(layer) % 2:
            layer = layer + [layer[-1]]
        path.append(layer[i ^ 1])
        layer = [dsha(layer[k] + layer[k + 1]) for k in range(0, len(layer), 2)]
        i //= 2

    # Refuse to emit anything the node does not agree with.
    if layer[0] != reverse(block["merkleroot"]):
        print("merkle computation disagrees with the node's merkleroot", file=sys.stderr)
        return 1
    if dsha(header) != reverse(block_hash):
        print("header hash disagrees with the node's block hash", file=sys.stderr)
        return 1

    capture = {
        "chain": info["chain"],
        "height": block["height"],
        "block_hash": block_hash,
        "header_hex": header.hex(),
        "version": int.from_bytes(header[0:4], "little"),
        "prev_block_hash_hex": header[4:36].hex(),
        "merkle_root_hex": header[36:68].hex(),
        "timestamp": int.from_bytes(header[68:72], "little"),
        "bits": hex(int.from_bytes(header[72:76], "little")),
        "nonce": int.from_bytes(header[76:80], "little"),
        "txid": args.txid,
        "tx_index": index,
        "tx_count": len(block["tx"]),
        "merkle_path_hex": [p.hex() for p in path],
        "raw_tx_hex": [t for t in json.loads(cli("getblock", block_hash, "2"))["tx"]
                       if t["txid"] == args.txid][0]["hex"],
    }
    capture["raw_tx_stripped_hex"] = strip_witness(
        bytes.fromhex(capture["raw_tx_hex"])).hex()
    capture["wtxid"] = dsha(bytes.fromhex(capture["raw_tx_hex"]))[::-1].hex()
    # The stripped serialization is the one whose hash is the txid — the value a
    # proof has to carry.
    if dsha(bytes.fromhex(capture["raw_tx_stripped_hex"]))[::-1].hex() != args.txid:
        print("witness-stripped serialization does not hash to the txid", file=sys.stderr)
        return 1

    # The block the node mined next, if there is one. A header can only be
    # *extended* by the header that links to it, so a test that exercises the
    # extension path on real data needs this pair, not a single block.
    if block["height"] + 1 <= info["blocks"]:
        next_hash = cli("getblockhash", str(block["height"] + 1))
        next_header = bytes.fromhex(cli("getblock", next_hash, "0"))[:80]
        capture["next_height"] = block["height"] + 1
        capture["next_block_hash"] = next_hash
        capture["next_header_hex"] = next_header.hex()
        capture["next_bits"] = hex(int.from_bytes(next_header[72:76], "little"))
        capture["next_timestamp"] = int.from_bytes(next_header[68:72], "little")
        capture["next_prev_block_hash_hex"] = next_header[4:36].hex()
        if capture["next_prev_block_hash_hex"] != reverse(block_hash).hex():
            print("the next block does not link to this one", file=sys.stderr)
            return 1

    json.dump(capture, open(args.out, "w"), indent=2)
    print(json.dumps(capture, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())

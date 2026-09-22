#!/usr/bin/env python3
"""libp2p peer id for an ed25519 node identity, from its 32-byte public key.

The id is `base58btc(0x00 0x24 || protobuf(ed25519 <pub>))` — the identity
multihash form Substrate prints as `Local node identity is: 12D3Koo…`. It is what a
chain spec's `bootNodes` entries must carry, and it has to be derived from the same
key the node is started with (`--node-key`), which is why the fixture gate asserts
the running node reports the peer id its spec lists.

Usage: peer-id-from-ed25519-pubkey.py <hex pubkey, 0x-prefixed or not>

The same 15 lines used to live inside `make-fixture-live-spec.sh`; one copy is
enough, and the spec builder needs it too.
"""
from __future__ import annotations

import sys

ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"


def base58(data: bytes) -> str:
    n = int.from_bytes(data, "big")
    out = ""
    while n:
        n, r = divmod(n, 58)
        out = ALPHABET[r] + out
    pad = 0
    for byte in data:
        if byte == 0:
            pad += 1
        else:
            break
    return "1" * pad + out


def peer_id(pubkey_hex: str) -> str:
    pub = bytes.fromhex(pubkey_hex.removeprefix("0x"))
    if len(pub) != 32:
        raise SystemExit(f"ed25519 public key must be 32 bytes, got {len(pub)}")
    return base58(bytes([0x00, 0x24, 0x08, 0x01, 0x12, 0x20]) + pub)


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__.strip().splitlines()[-1], file=sys.stderr)
        return 2
    print(peer_id(sys.argv[1]))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

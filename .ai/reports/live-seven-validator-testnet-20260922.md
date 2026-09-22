# Seven validators on a Live testnet spec — one canonical chain

Date: 2026-09-22. Base: `origin/master` = `b6bcd4a4e`.

Closes the blocker recorded in `multi-validator-launcher-20260922.md`: the launcher
consumer of a Live spec could not start because a Live chain spec must carry at
least one bootnode, and a bootnode entry needs a peer id derived from the node
identity the node will be started with.

## What changed

**`scripts/mainnet/peer-id-from-ed25519-pubkey.py`** — the libp2p peer id
derivation (`base58btc(0x00 0x24 || protobuf(ed25519 pub))`) in one place.
`make-fixture-live-spec.sh` had it inline and the spec builder needs the same value,
which is exactly how two copies drift.

**`scripts/testnet/build-x3-testnet-spec.py`** — for each validator it now writes
`validator-keys/validator-<n>.nodekey` (32-byte hex, 0600, beside the seed),
derives that key's peer id, and sets `TESTNET_BOOTNODES` to the seven
`/ip4/127.0.0.1/tcp/<30333+n>/p2p/<peer>` entries before `build-spec`. After
writing the spec it asserts the file carries **all** of them:

```
[spec] x3-testnet-plain.json carries all 7 derived bootnodes (peer ids match the node keys beside the seeds)
```

The old self-check ("loads in the node") did not cover this: `build-spec` accepts a
Live spec with no bootNodes, and the *validator* startup is what refuses it.

**`scripts/testnet/run-7-validators-local.sh`** — `--node-key` now prefers
`validator-<n>.nodekey` from the keys dir (falling back to a per-base-dir key), the
preflight verifies each node's derived peer id is in the spec's bootNodes, and the
spec sanitizer no longer empties `bootNodes` for a **plain** spec:

```
Using chain spec: /tmp/x3-7v3/chain-spec.json (plain form; keeping 7 bootnode(s))
[validate] ok: plain Live spec Aura=7 Grandpa=7; every launcher key (Aura 7, GRANDPA 7) is in the authority sets …
[validate] ok: all 7 launcher peer id(s) are in the spec's 7 bootnode entry(ies).
```

(It still strips them for a raw spec, which is why the strip existed: a raw spec
carries whatever bootnodes it was generated with.)

## Verified

```
$ X3_NODE_BIN=…/x3-chain-node python3 scripts/testnet/build-x3-testnet-spec.py 7
[spec] x3-testnet-plain.json written (17,245,301 bytes) … carries all 7 derived bootnodes

$ COUNT=7 CHAIN_SPEC=deployment/chain-specs/fresh/x3-testnet-plain.json \
  NODE_BIN=…/x3-chain-node bash scripts/testnet/run-7-validators-local.sh
Started node-01 (p2p=30333, rpc=9944) … node-07 (p2p=30339, rpc=9950); 7 × ready

# seven processes, six peers each (full mesh)
rpc 9944 … rpc 9950: finalized heads advancing, peers=6

# one canonical chain — every validator returns the same hash at the same height
height 900, all seven: 0x71a9279bf72ce9a5ffe5e3cd43596dadb38c4dc8124045bfbd27a8b5f01c7405
height 950, all seven: 0x50b771aa6a09f38631b2a81251f4c7d0bdb4abeffe58eba5b711c87b9166accf
```

Their own `chain_getFinalizedHead` values differ by a few blocks while the nodes
are CPU-saturated on one box (7 debug nodes), which is why the invariant to check is
agreement *at a height*, not equality of each node's latest finalized pointer.

Regression check for the shared helper — the testnet genesis gate, which asserts the
running node reports the peer id its spec's bootnode entry names:

```
[genesis-gate] validator 1 peer id matches the spec's bootnode entry
[genesis-gate] all three connected (peers: 2/2/2)
[genesis-gate] PASS — testnet genesis builds, boots, finalizes and agrees at height 95 (0xec8a51b0…)
```

## What this does and does not establish

Does: seven validators, seven authorities, a Live testnet spec, bootnodes derived
from the nodes' own identities, full mesh, GRANDPA finality on every node, and
agreement on one chain. The path is reproducible from two repo scripts with the
node binary as the only tool.

Does not: independence. All seven run on one host under one operator. There is no
slashing/jailing/equivocation evidence from a real multi-operator network, nothing
runs for longer than minutes, and no failure injection (partitions, clock skew,
restarts under vote loss) has been exercised. `X3-L1-001` now says that:
implemented 85, tested 75, mainnet-ready 45.

# The multi-validator launcher could not start a network

Date: 2026-09-22. Base: `origin/master` = `b28406ab2`.

Found while auditing `X3-L1-001` (multi-validator authority network, P0, 25%),
whose only registered path was `scripts/testnet/run-7-validators-local.sh`. Three
separate blockers meant that path could not run at all.

## 1. It required `subkey`, which is not installed and is not part of this repo

```bash
SUBKEY_BIN_DEFAULT="${SUBKEY_BIN_DEFAULT:-/home/lojak/.cargo/bin/subkey}"
...
aura_pub=$("$SUBKEY_BIN" inspect --scheme sr25519 "$suri" | awk ...)
```

`command -v subkey` fails here, and the script exits before doing anything. The
keystore filenames it computed by hand (`61757261<aura-hex>`,
`6772616e<grandpa-hex>`) are exactly what the node's own `keys insert` writes:

```
$ x3-chain-node keys insert --key-type aura --seed //Alice --keystore-path /tmp/k
5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY
inserted aura key into /tmp/k
$ ls /tmp/k
61757261d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d
```

The launcher now inserts through the node and validates with `keys list`, so the
only tool it needs is the node it was going to run anyway.

## 2. Its spec and its keys were produced by two scripts that could not compose

`scripts/testnet/build-x3-testnet-spec.py` builds a plain Live spec from **fresh**
per-validator seeds and writes them to `validator-keys/validator-<n>.suri`
(`seed=`/`aura=`/`grandpa=` lines of hex key material). The launcher hardcoded
`DEV_SEEDS=(//Alice … //One)` and had no way to consume those files — so a spec
built from one and nodes started from the other is a network whose authorities hold
none of its keys. It starts, and authors nothing.

The launcher now reads `--keys-dir` (default
`deployment/chain-specs/fresh/validator-keys`), takes the `seed=` value from each
file, and — before starting anything — derives each seed's Aura and GRANDPA public
keys with the node and requires them to be in the spec's authority sets:

```
[validate] ok: plain Live spec Aura=7 Grandpa=7; every launcher key (Aura 7,
GRANDPA 7) is in the authority sets (seeds from …/validator-*.suri).
```

If they are not, the launch stops with `exit 4` and says why, instead of producing
a network that looks up and does nothing.

## 3. Nodes exited with `NetworkKeyNotFound`

```
Error: NetworkKeyNotFound("/tmp/x3-3v/node-1/chains/x3_chain_local3/network/secret_ed25519")
```

Nothing supplied a libp2p identity and this build does not create one. The launcher
now generates a stable 32-byte key per validator under
`$BASE_DIR/node-keys/node-<n>.key` and passes `--node-key` — the same file a Live
spec's bootnode entry has to be derived from (`make-fixture-live-spec.sh` does
exactly that for its three-node fixture).

## Verified: three validators authoring and finalizing

```
$ COUNT=3 CHAIN_SPEC=chain-specs/x3-local3-current-plain.json \
  KEYS_DIR=/nonexistent BASE_DIR=/tmp/x3-3v \
  NODE_BIN=…/x3-chain-node bash scripts/testnet/run-7-validators-local.sh
Started x3-testnet-node-01 (p2p=30333, rpc=9944, prom=9615) … Node x3-testnet-node-01 ready
Bootnode: /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWFELd46oDmtu3wcmxAqEceTTgy9BrJknKDZVvta5huEhf
Started x3-testnet-node-02 (p2p=30334, rpc=9945, prom=9616) … ready
Started x3-testnet-node-03 (p2p=30335, rpc=9946, prom=9617) … ready

# after ~1 minute, from the three RPC endpoints
rpc 9944 head 406   rpc 9945 head 406   rpc 9946 head 406

# a few seconds later — GRANDPA finality, all three on one block
rpc 9944 finalized#=504 hash=0x8f1a0eba1bcba88bffe06df76c2df6a9d02616f7dacb58a68c3aed504c5e6222 peers=2
rpc 9945 finalized#=504 hash=0x8f1a0eba1bcba88bffe06df76c2df6a9d02616f7dacb58a68c3aed504c5e6222 peers=2
rpc 9946 finalized#=504 hash=0x8f1a0eba1bcba88bffe06df76c2df6a9d02616f7dacb58a68c3aed504c5e6222 peers=2
```

(`KEYS_DIR=/nonexistent` deliberately selects the built-in dev seeds, which are
the `x3-local3-current-plain.json` authorities: Alice, Bob, Charlie in both
schemes. A Live spec uses the seed files instead.)

## What is still missing for seven validators on a Live chain

The launcher consumes a Live spec; it cannot add `bootNodes` to one, and the node's
Live-spec validation refuses a spec with none:

```
Error: Input("Live chain spec requires at least one bootnode")
```

A bootnode entry needs the peer id, which is derived from the node key — so the spec
must be built with those keys in hand. `make-fixture-live-spec.sh` already does this
for its three-validator fixture (it derives `peer_id_for()` from each node key and
exports `TESTNET_BOOTNODES` before `build-spec`). The remaining work is to have
`build-x3-testnet-spec.py` write `validator-<n>.nodekey` alongside the seed files
and set `TESTNET_BOOTNODES` from them, and have the launcher prefer those node keys
over its generated ones. Until then the 7-validator path is runnable for Local
specs (proven above) and blocked for Live ones — which is what the matrix row now
says, instead of "no sustained independent-validator production evidence" pointing
at a script that could not start.

## The row

`X3-L1-001` moves 55→70 implemented, 25→55 tested, 25→35 mainnet-ready, names the
launcher, the spec builder and the two working three-validator gates
(`local-network-smoke.sh`, `make-fixture-live-spec.sh`), and carries the blockers
that are true: one host, no independent operators, no sustained run, the Live
seven-validator gap above, and slashing/jailing evidence that only exists as unit
coverage.

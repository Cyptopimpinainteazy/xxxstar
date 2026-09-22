# Testnet bring-up: one path, and it starts a network

Date: 2026-09-22. Base: `origin/master` = `490bea55a`.

Pushing on "testnet ready". The repo's own punch list for it is
`TESTNET_GAP_LEDGER.md` (104 lines, last re-verified 2026-09-04); this turn re-measured
it, closed two entries, corrected one, and made the entry point operators know
(`scripts/testnet/x3_testnet_up.sh`) actually start a network.

## The launcher was the third copy, and the worst one

`x3_testnet_up.sh` still required `subkey` — not installed here, not part of this
repository — defaulted to `deployment/chain-specs/x3-testnet-raw.json`, a storage-raw
*Live* spec the node's loader refuses, and started nodes with
`--unsafe-force-node-key-generation`, so the peer ids a spec's `bootNodes` name could
never be stable across a restart. Three ways not to start, all three fixed since in
`run-7-validators-local.sh`.

It is now a thin wrapper: resolve a built node, refuse a raw Live spec with a message,
build a plain spec if none is there (via `build-x3-testnet-spec.py`), and delegate with
its CLI intact:

```
$ NODE_BIN=… SKIP_BUILD=1 COUNT=4 CHAIN_SPEC=deployment/chain-specs/fresh/generated/x3-testnet-plain.json \
    bash scripts/testnet/x3_testnet_up.sh --skip-build
[spec] deployment/chain-specs/fresh/generated/x3-testnet-plain.json
[delegate] COUNT=4 run-7-validators-local.sh --chain-spec … --node-bin … --base-dir /tmp/x3-up4 …
Node x3-testnet-node-01 ready … Node x3-testnet-node-04 ready

# one canonical chain — every validator, same hash, two heights
height 1000, all four: 0x5e85c48338e8bd6c5839e8c8239ff6dd3517e3abf09bcac290ee2c7f6d84a571
height 1050, all four: 0x586876229757149e6be811ab16ade85782f94fdd6aad3e2cc27d84276e0a7c25

$ … x3_testnet_up.sh --chain-spec deployment/chain-specs/x3-testnet-raw.json --skip-build
[spec] …/x3-testnet-raw.json is a raw Live spec; the node refuses to load one.
       Build a plain one instead: python3 …/build-x3-testnet-spec.py 7
```

(Before this, the same wrapper on the same machine could not start anything: `subkey
not found`, then `Live chain spec requires at least one bootnode`, then
`NetworkKeyNotFound` — the three failures the ledger recorded as separate gaps.)

## The ledger, re-measured

| entry | then | now |
| --- | --- | --- |
| GAP-CLI-1 (stale flags / no node key / subkey) | "FIXED in place, re-verify" | **CLOSED** — delegates to the one launcher; 4 validators boot and agree through it; raw spec refused |
| GAP-SPEC-1 (stale raw spec invalid) | P0 open | **CLOSED** — default is a generated plain spec; the builder asserts every derived bootnode is in the file; the launcher preflight refuses mismatched authorities or peer ids |
| GAP-AUTH-1 ("file-only keystore does not drive Aura, X3_DEV_SEED required") | P0, "works as designed, deferred indefinitely" | **CORRECTED** — measured 2026-09-22: keystore-only keys author with no `X3_DEV_SEED` (head 9 in ~45 s). The disposition rested on a premise that no longer holds |

## What is still missing, and it is not code

Nothing is deployed. Measured the same day:

* `rpc.testnet.x3-chain.io`, `faucet.testnet.x3-chain.io`, `bootnode.testnet.x3-chain.io`
  do not resolve — from a host whose DNS reaches github.com;
* `gh run list --workflow testnet-deploy.yml` is empty: the deploy workflow has never run;
* every step of `docs/reports/TESTNET_DEPLOYMENT_CHECKLIST.md` is unchecked (validator
  VMs, RPC VMs, bootnode, monitoring, DNS, firewall);
* the only bootnode list in the repository, `deployment/keys/bootnode-info.txt`, is
  three `/ip4/127.0.0.1/tcp/30333/…` entries.

`docs/root/README.md` used to advertise the RPC and faucet as if live; it now says "not
deployed", states what was checked, and points at what does run locally.

## Rows

| row | before | after |
| --- | --- | --- |
| `X3-L1-010` bootnode / peer discovery ops | 35/15/20, path = a roadmap doc | **65/45/35**, path = the peer-id helper, the spec builder, the launcher, the bootnode file |
| `X3-OPS-001` genesis ceremony tooling | 75/45/35 | unchanged scores; evidence now names the local rehearsal that runs today, blockers stay "no tagged ceremony, no published record" |
| `X3-OPS-003` public testnet gate | 65/45/45 | unchanged scores; evidence now carries the measurements above, and the blocker is explicit: there is no public testnet to gate |

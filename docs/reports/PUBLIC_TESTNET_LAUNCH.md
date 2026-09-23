# Public testnet launch — the operator path that exists today

Everything below is runnable, and was exercised locally on 2026-09-22 unless marked
otherwise. It is the runbook for turning this repository into a running public testnet;
`TESTNET_DEPLOYMENT_CHECKLIST.md` remains the infrastructure checklist (hosts, DNS,
monitoring), and this file is the commands that go with it.

## 0. What is already true

* a Live chain spec with fresh authorities, per-validator seeds, per-validator node keys
  and `bootNodes` derived from those keys (`scripts/testnet/build-x3-testnet-spec.py`);
* a launcher that refuses to start unless every session key is an authority in the spec
  (and, for single-host runs, every node's peer id is a bootnode), with per-node
  `--node-key` and a restart path (`scripts/testnet/run-7-validators-local.sh`);
* a bootnode identity tool that produces a publishable address
  (`scripts/testnet/public-node-id.sh`);
* a ceremony manifest tool that records a launch and verifies a running network against
  it, including a negative control (`scripts/testnet/testnet-ceremony.py`);
* gates: `local-ci.sh --testnet` (ceremony), `--soak` (duration), `--failure`
  (minority failure / missing supermajority / recovery), plus `public_testnet_gate.sh`
  for an RPC endpoint.

**Not covered here, and not done:** DNS, hosting, TLS, monitoring, the faucet, the
explorer, and the signed publication of a manifest. The deployment scripts for several
of those exist in `scripts/testnet/deploy-*.sh` and have never been exercised.

## 1. Decide the shape

Pick, and write down, at least: the bootnode's DNS name and p2p port; how many
validators and on which hosts; the escrow addresses (`X3_EVM_ESCROW_ADDR`,
`X3_SVM_ESCROW_ADDR`); the council and treasury accounts (**not** dev seeds — the node
refuses a Live spec whose endowed accounts are dev-seed-derived); and the chain id.

## 2. Bootnode identity, published before anything runs

```bash
NODE_BIN=target/release/x3-chain-node \
  scripts/testnet/public-node-id.sh \
    --host bootnode.testnet.example --p2p-port 30333 \
    --key-file deployment/keys/bootnode.nodekey
```

It prints the peer id and the `/dns4/…/p2p/…` address. Back the key file up; it is
gitignored and it *is* the identity the address names — regenerating it invalidates the
published address. Publish the address where validators and users can read it.

## 3. Build the spec

```bash
PUBLIC_BOOTNODES=/dns4/bootnode.testnet.example/tcp/30333/p2p/12D3Koo… \
X3_NODE_BIN=target/release/x3-chain-node \
python3 scripts/testnet/build-x3-testnet-spec.py <validator-count>
```

It writes, into `deployment/chain-specs/fresh/generated/` (gitignored):

* `x3-testnet-plain.json` — the spec, with the published bootnode(s) and the authority
  set. Assert at the end that it carries every bootnode entry and that the node loads it;
* `validator-keys/validator-<n>.suri` — one seed per validator (0600): the *authority*
  key material. Distributing these is the trust decision of the launch;
* `validator-keys/validator-<n>.nodekey` — one libp2p identity per validator (0600).

Keep the 0600 permissions, back both sets up, and never commit them
(`deployment/chain-specs/fresh/.gitignore` covers that path).

### Pinning Bitcoin's checkpoint (the SPV trust root)

The settlement engine accepts BTC evidence only from a header on a **checkpoint-anchored**
chain, and until one exists it refuses every BTC proof — correct, and useless for a testnet
unless the anchor is part of the launch. Two ways to set it, and the second is the one to
use for a network:

```bash
# Option A — a root call after the chain is live (requires a runtime with `Sudo`):
#   sudo.sudo(x3SettlementEngine.anchorBtcCheckpoint(header))
#   Write-once per height: it will not re-point an anchored height at another branch.

# Option B — pin it in the spec, so the chain is born anchored:
X3_BTC_CHECKPOINTS="<80-byte header hex>@<height>" \
X3_NODE_BIN=target/release/x3-chain-node \
python3 scripts/testnet/build-x3-testnet-spec.py <validator-count>
```

`X3_BTC_CHECKPOINTS` is a comma-separated list of `<header hex>@<height>` (a Bitcoin header
does not carry its own height — a node knows it from where the header sits in the chain).
The generated spec's `genesis.runtimeGenesis.config.x3SettlementEngine.btcCheckpoints`
carries it, which means **the commitment is reviewable in the same diff as the chain id**,
identical for everyone who joins from that spec, and not something a key holder can change
later. The pallet validates every entry as genesis is built: a header that does not satisfy
its own proof of work, or whose `nBits` is easier than this network's `powLimit`, or a second
entry at an already-claimed height, makes the node **refuse to start** rather than launch a
chain whose root of trust is a lie.

Choose the header from the network you are tracking. A regtest header (`0x207fffff`) only
anchors on a chain built with `--features dev`, because that is the only runtime whose
`powLimit` is regtest's; testnet and mainnet specs use `0x1d00ffff` and need a header from
that network. Get one with `scripts/btc/capture-regtest-spv.py` (regtest) or any Bitcoin
RPC (`getblockheader <hash> false`), and check it against a second source before shipping it:
the spec makes this chain *believe* that hash, so a wrong one is a wrong chain.

`scripts/testnet/btc-checkpoint-drill.sh` is the gate for this: it pins a real captured
header, boots a node, reads `BtcCheckpoints` / `BtcHeaderMetaStore` / `BtcBestHeight` back
over RPC, requires the chain to keep authoring, and requires a spec whose checkpoint is not
a mined header to be refused. Run it after changing anything here.

### Making the chain follow Bitcoin (a header relayer)

An anchor is a starting point, not a subscription: until something pushes headers, the chain's
view of Bitcoin stops at the checkpoint and no later deposit can be proven. The receiving end is
`x3SettlementEngine.submitBtcHeaders(headers)` — call_index 35, up to 100 headers per call, atomic
(a batch refused at header *k* applies nothing) — and the sender is
`scripts/btc/push-headers.mjs`, which reads headers from a local Bitcoin node, checks the chain of
them itself, and submits them.

Who may submit is `x3SettlementEngine`'s `BtcHeaderOrigin` config item. This runtime sets
`EnsureRoot`, so the path is root-only: on a dev chain that means `sudo.sudo(...)`, and on a
network with governance it means a governance motion. To run a relayer service, point that item at
the origin you trust — `EnsureSignedBy<BtcRelayerAccount, AccountId>`, a multisig, or a proxy — and
nothing about the receiving rules changes, because the item decides who may **speak**, never what is
**true**: every header still has to satisfy proof of work under this network's `powLimit`, link to
an admitted header, carry its parent's `nBits` for its height, and postdate the median of its
ancestors.

What that means in practice: a Byzantine relayer cannot mint Bitcoin. It can **withhold** headers
(the chain stops advancing — a liveness problem) or push a branch that satisfies the rules (which
the checkpoint and the 4× retarget clamp make expensive). The bond and the slashing that price
withholding are not written yet — TICKET-095 — so run one relayer you control, and expect that to
be a single point of failure until there are several.

## 4. Start each validator

On the host that will run validator *n*: copy the spec, its `validator-n.suri` and
`validator-n.nodekey`, then

```bash
SKIP_BOOTNODE_MEMBERSHIP_CHECK=1 \
NODE_BIN=x3-chain-node \
CHAIN_SPEC=<spec> \
COUNT=<n> KEYS_DIR=<keys dir> \
BASE_DIR=/var/lib/x3-validator LOG_DIR=/var/log/x3-validator \
  scripts/testnet/run-7-validators-local.sh --only <n>
```

`--only <n>` starts exactly one validator from an existing base dir and exits, which is
also the restart path. `SKIP_BOOTNODE_MEMBERSHIP_CHECK=1` is required for a multi-host
launch: the spec's bootnode is a host these validators dial, not one of them — the
authority check still applies and still refuses a validator whose session key is not an
authority.

Start the bootnode itself first, from the same spec, with its key:

```bash
x3-chain-node --chain <spec> --base-path /var/lib/x3-bootnode \
  --node-key "$(cat deployment/keys/bootnode.nodekey)" \
  --listen-addr /ip4/0.0.0.0/tcp/30333 --no-mdns --no-telemetry --validator
```

Confirm the identity you published is the identity it reports:

```bash
curl -s -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"system_localPeerId","params":[]}' \
  http://127.0.0.1:9944 | python3 -c 'import json,sys; print(json.load(sys.stdin)["result"])'
```

If that does not match the published `/p2p/…`, the spec is dialling the wrong identity.

## 5. Record the launch, then verify it

```bash
python3 scripts/testnet/testnet-ceremony.py record <spec> \
  --node-bin x3-chain-node --rpc 9944,9945,… --out ceremony.json
python3 scripts/testnet/testnet-ceremony.py verify ceremony.json \
  --rpc 9944,9945,… --node-bin x3-chain-node --min-finalized 100
```

`record` writes the spec sha256, the binary sha256, the chain name/id/type, the genesis
hash, the runtime version, the authority sets and every validator's peer id, peer count
and finalized height. `verify` re-checks all of it and names the first disagreement —
the check a user should run against a published testnet. Publish the manifest (and its
signature) next to the bootnode address; that is the artifact the network can be held to.

## 6. Gate it

```bash
scripts/mainnet/public_testnet_gate.sh --rpc-base-url https://rpc.testnet.example
bash scripts/local-ci.sh --testnet --soak    # the same checks on a local rehearsal
```

`public_testnet_gate.sh` checks live health, GRANDPA authorities, bridge storage and
block height over RPC. The local gates exist so the ceremony and the soak are exercised
with every change rather than on launch day.

## 7. What is still missing before this is a *public* testnet

* hosts, DNS, TLS and monitoring (the checklist's unchecked half);
* a faucet and an explorer the deployment scripts describe but nobody has run;
* a signed, published manifest;
* independent operators — a testnet that demonstrates anything has validators run by
  people other than the launcher;
* cross-chain flows, which today can only be party-signed: the relayer cannot submit a
  proof until the authority question is decided.

# The multi-node proof, and the runtime blob that made the node unrunnable

2026-09-25. ROADMAP PRIORITY 4's rehearsal, on one host, because the seven servers are not here yet.
Three findings, in the order they appeared.

## 1. The proof script's six documented defects were all still there

`launch-gates/multi-node-testnet-proof.ROOT-CAUSE.md` (2026-04-28) lists six reasons the script
produced zero blocks and closes with *"out of scope for this session — flagged for next focused
work."* Four months later, all six were in the file: `timeout 10` killing every validator ten seconds
in, a fabricated bootnode peer id, `--chain dev` (one authority), validator 0's P2P port equal to its
own RPC port, a bootnode port nothing listened on, and — added by me this cycle — a cleanup step that
ran `pkill -f x3-chain-node`, which kills every node on the host including one the operator is
running.

`scripts/mainnet/boot_local3.sh` had the working pattern all along: boot the `local3` spec with
`--alice/--bob/--charlie`, read alice's real peer id out of her log, pass that as `--bootnodes`, keep
P2P and RPC on disjoint ports. The script now does that, proves **three** authorities rather than the
four it claimed (the node has `dev`, `local` and `local3`; there is no four-validator spec, so "4
validators" was never achievable), and stops only the processes it started.

## 2. The release node binary could not start at all

The first run after those fixes failed with:

```
Cannot create a runtime error=Other("runtime requires function imports which are not present on the
host: 'env:ext_benchmarking_commit_db_version_1', ...")
```

The embedded runtime was built **with `runtime-benchmarks`** and the host was not. The mechanism is in
`runtime/build.rs`:

* `substrate-wasm-builder` decides whether to rebuild from source timestamps, and this crate's *feature
  set* is not a source file — so a blob built with `runtime-benchmarks` looks fresh to a build without
  it and is embedded anyway;
* the script then wrote the feature sidecar *after* `build()` unconditionally, overwriting the record
  with the current build's key. The sidecar therefore claimed the blob matched the host when it did
  not, which is precisely what the sidecar was added to prevent.

Measured on this tree: `target/release/wbuild/x3-chain-runtime/x3_chain_runtime.wasm.features` said
`native-real-vm-adapters,pallet-sudo,runtime-benchmarks` and the blob imported nine
`ext_benchmarking_*` hosts, while `target/release/x3-chain-node` provided none. Every `--chain`
failed, including `dev`. **This is a regression from my own earlier work on the `runtime-benchmarks`
build** (commit `f4357ae025`, which built the node with that feature to verify the fix).

Fixed: the sidecar is now read *before* the build as well. A mismatch removes
`x3_chain_runtime.wasm`, its compact and compressed forms and the sidecar, so `build()` has to produce
a blob for *this* feature set. The rebuild printed the warning and produced a blob with **zero**
benchmarking imports; the node starts and runs.

## 3. The local3 chain spec's bootnode cannot be used by anyone

With the binary working, alice started, produced a real peer id, and then bob refused to start:

```
X3 Chain node terminated with an error: The same bootnode (`/ip4/127.0.0.1/tcp/30333`) is registered
with two different peer ids: `12D3KooWSDG3ssm...` and `12D3KooWJqntxd6...`
```

The `local3` spec that the binary emits carries `/ip4/127.0.0.1/tcp/30333/p2p/12D3KooWSDG3ssm...` — the
same id as the committed `chain-specs/x3-local3-*.json`. Nothing in the repository holds that node
key, so no process can ever be that peer; and passing `--bootnodes` with a *real* identity on the same
address is refused by libp2p. So the local3 spec is unjoinable as committed, which is why the proof
cannot pass yet.

## 4. The deployable specs are not deployable, and the gate that says so was in no gate list

While tracing the bootnode, `scripts/ci/check_deployable_bootnodes.sh` — which asks exactly this
question of the specs that are *shipped* rather than generated — turned out to be in no gate. Run by
hand it reports four problems in the specs `Dockerfile.validator` and `k8s/02-configmaps.yaml` ship:

```
FAIL  deployment/chain-specs/x3-testnet-raw.json: Live spec with no bootNodes
FAIL  deployment/chain-specs/fresh/x3-testnet-plain.json: bootNode /ip4/127.0.0.1/... is on 127.0.0.1
FAIL  k8s/02-configmaps.yaml:x3-testnet-raw.json: bootNode /ip4/127.0.0.1/... is on 127.0.0.1
FAIL  k8s/02-configmaps.yaml:x3-testnet-raw.json: Live spec with an empty genesis.raw.top
```

`deployment/keys/bootnode-info.txt` compounds it: three bootnodes, all on `/ip4/127.0.0.1/tcp/30333`,
with three different peer ids — a list libp2p rejects for the same reason it rejected bob's.

The new `deployable bootnodes` gate runs the checker through a ratchet: the four problems are recorded
in `security/deployable-bootnode-baseline.txt`, a fifth fails the build, and fixing one of the four
also fails it until the baseline shrinks. Fixing them needs addresses and a genesis that only the
ceremony can produce, which is why the ratchet exists rather than the fix.

## State

* **Verified:** the release binary starts and runs; alice starts under the proof harness with a real
  peer id; the proof does *not* pass — it stops at the local3 bootnode conflict, which is finding 3.
* **Tickets:** (a) the local3 spec's unusable bootnode (remove it, or document the key that makes it
  reachable); (b) the four deployable-spec problems, for the ceremony; (c) re-run
  `multi-node-testnet-proof.sh` once (a) is resolved — it should then reach the block-production and
  authority-loss checks; (d) `boot_local3.sh` has the same `NetworkKeyNotFound` gap and the same
  bootnode conflict, and needs the same `--node-key-file` treatment.

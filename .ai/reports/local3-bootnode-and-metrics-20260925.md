# The local3 spec shipped a bootnode no process holds — and the harness found a second defect behind it

Date: 2026-09-25
Scope: `launch-gates/multi-node-testnet-proof.sh`, `scripts/mainnet/boot_local3.sh`,
`chain-specs/x3-local3-*.json`, `scripts/ci/check_deployable_bootnodes.sh`

## What was reported

`launch-gates/multi-node-testnet-proof.ROOT-CAUSE.md` (2026-04-28) listed six reasons
the three-validator proof produced zero blocks and closed with "out of scope for this
session — flagged for next focused work". All six were still present four months later.
Fixing them exposed two more.

## Defect 7 — `build-spec` injects a bootnode, and the constructor declares none

```
bob: The same bootnode (`/ip4/127.0.0.1/tcp/30333`) is registered with two different
     peer ids: `12D3KooWSDG3ssm6N2oc6HLb4obWHDBeYG5J54nS5jTzj4mRpujM` and `12D3KooWJqntxd6…`
```

That peer id is not in the binary (`grep -c -a` = 0) and not in any source file — only in
four checked-in JSON specs. It is not a stray constant. `build-spec --help` documents it:

```
--disable-default-bootnode  Disable adding the default bootnode to the specification.
  By default the /ip4/127.0.0.1/tcp/30333/p2p/NODE_PEER_ID bootnode is added to the
  specification when no bootnode exists
```

`local_three_validator_config()` sets no boot nodes (`with_boot_nodes` appears only in the
staging, testnet and production constructors), so every `build-spec --chain local3` — and
every checked-in spec, which came from one — advertises a peer id derived from a key
nothing holds. Each launcher then handed the same validators the *real* bootnode on that
address, and libp2p refuses the pair. `chain-specs/` is not a source of boot addresses; it
is a record of a command that was run without a flag.

## Defect 8 — three validators, one Prometheus port

With the bootnode gone the harness ran further and:

```
Thread 'tokio-rt-worker' panicked at 'error binding to 127.0.0.1:9615:
error creating server listener: Address already in use (os error 98)'
```

Validators 1 and 2 died during startup, before their P2P listeners existed. Validator 0
then reported `Idle (0 peers), best: #0` for two minutes, which reads exactly like broken
consensus. `rc3_failure_drills.sh` (9615-9617), `rc5_internal_alpha_72h.sh` (9715-9717),
`local-network-smoke.sh` and `testnet-full-launch.sh` all already assign disjoint metrics
ports; the proof harness and `boot_local3.sh` were the two that did not.

## Fixes

* `--disable-default-bootnode` on every `build-spec` call in both scripts, plus an
  assertion in the harness that the generated spec's `bootNodes` is empty.
* `--prometheus-port` 9615/9616/9617 in both scripts.
* `boot_local3.sh` writes a per-validator `--node-key-file` under `logs/` (gitignored):
  `--alice` supplies session keys for the dev genesis, not a libp2p identity, so a base
  path with no network key exits with `NetworkKeyNotFound`.
* `chain-specs/x3-local3-{plain,raw,current-plain,current-raw}.json`: `bootNodes` emptied.
* `check_deployable_bootnodes.sh`: a checked-in `chain-specs/x3-local3-*.json` that declares
  a bootnode is now a named failure, so the file cannot be regenerated without the flag
  and land again. The ratchet baseline is unchanged at four known deployment problems.

## Proof

```
$ bash launch-gates/multi-node-testnet-proof.sh .
PASS: node binary and jq present
PASS: local3 spec generated (plain and raw)
PASS: validator 0 identity: 12D3KooWMPSLjiQUyBr6FbLkv9SwymM64XYZmKjG9CLd8e6pJpas
PASS: all 3 validators started
  block #9 / #33 / #58 (consecutive: 1,2,3)
PASS: consensus: 3 consecutive blocks, head #58
  alice on 9944: 2 peer(s); bob on 9945: 2 peer(s); charlie on 9946: 2 peer(s)
PASS: 3/3 validators responding
PASS: chain continued after losing an authority: #58 -> #76
PASS — 3 authorities produced and advanced the chain, every node answered RPC,
       and the chain continued after one was stopped.
```

```
$ bash scripts/ci/check_deployable_bootnodes_ratchet.sh
check-deployable-bootnodes: OK - no new problems; 4 known, all on the shrinking list

$ bash scripts/local-ci.sh --only 'script-syntax,deployable-bootnodes'   # both PASS
$ bash scripts/local-ci.sh --live --only 'local-node-smoke,local-network-smoke'  # both PASS
```

## Not proven, and what is left

* All three validators ran on one host. This is ROADMAP PRIORITY 4's seven-server
  network; nothing here substitutes for it.
* `boot_local3.sh` is fixed by inspection and syntax only — it starts long-lived nodes,
  so it was not run to completion. The same two defects were reproduced and fixed in the
  harness, which is the script that asserts its own result.
* The harness is still not a gate: it binds fixed ports (30333-30335, 9944-9946,
  9615-9617), which collide with a validator already running on the operator's host.
  `local-network-smoke.sh` avoids this with a random base port. Making the harness
  port-configurable is the prerequisite for adding it to `--live`.
* The checked-in local3 specs still embed the runtime WASM they were generated with,
  which is older than the current runtime. The harness regenerates specs instead of
  using them, so nothing has compared the two.

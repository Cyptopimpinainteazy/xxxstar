# A launch you can verify against a record

Date: 2026-09-22. Base: `origin/master` = `22674564e`.

Continuing the testnet push. Starting a network was the previous turn's problem; this
one is what a *published* testnet needs next — a record of what was launched that
anyone can check a running network against, and a verifier that has been seen to fail.

## The tool

`scripts/testnet/testnet-ceremony.py record|verify`:

* `record` writes what was actually launched: the spec path with its sha256 and size,
  the node binary with its sha256, the chain name/id/type, the genesis hash the network
  reports, the runtime version, the authority sets the spec declares, and every
  validator's libp2p peer id, peer count and finalized height.
* `verify` takes that manifest to a running network and re-checks every claim, one line
  per check, naming the first disagreement. The spec and binary hashes come from the
  files; the chain-side values are compared against the manifest, not against each
  other.

`scripts/testnet/testnet-ceremony-drill.sh` runs the whole thing: build a spec, boot
four validators through `x3_testnet_up.sh`, record, verify, then tamper with a copy of
the manifest and require the verifier to reject it.

## Verified

```
$ bash scripts/local-ci.sh --testnet --only testnet-ceremony-drill
PASS testnet ceremony drill            290s

[ceremony] recorded 4 validator(s): genesis 0xa1005528d8d6ec35693e37f8d72a85408067aabeca8fa7f8a6379184628000ad,
           spec a3cc9e39719fe017…, spec_version 12 -> /tmp/x3-ceremony/ceremony.json
  ok    spec sha256 matches the manifest
  ok    node binary sha256 matches the manifest
  ok    rpc 9944…9947: chain name / genesis hash / spec_version / transaction_version /
        GRANDPA authority set / finality advancing / peer id matches the manifest
  [ceremony] PASS — 4 validator(s) match the manifest
[ceremony-drill] PASS: a tampered manifest is rejected (wrong genesis hash)
[ceremony-drill] PASS: no validator processes left
[ceremony-drill] ALL PHASES PASSED: recorded, verified, and proven able to fail.
```

The manifest for that launch: `X3 Chain Testnet` (Live, `x3_chain_testnet`), spec
17,243,738 bytes, 4 aura / 4 grandpa authorities, four validators with distinct
`12D3Koo…` peer ids and 3 peers each, finalized height 273 at record time.

## One harness fix this exposed

The drill failed twice before it passed, both for reasons worth recording:

* the launcher's `wait_for_rpc` allowed 60 s; a cold debug-build node reading a 17 MB
  spec and a fresh keystore took longer on a back-to-back run, and the launch was
  declared failed while the node was still starting. It is now bounded at 180 s and
  still names the port that never answered.
* the drill's own "no processes left" check was `pgrep … | wc -l` under
  `set -o pipefail`: zero matches makes `pgrep` exit 1, which aborts the script *after*
  it has cleaned up correctly. Same trap as the failure drill; both now swallow that
  status.

## Rows

| row | before | after |
| --- | --- | --- |
| `X3-OPS-001` genesis ceremony tooling | 75/45/35 | **85/65/45** — the local rehearsal now produces and verifies a record; the mainnet tagged-commit ceremony and publication remain |
| `X3-OPS-003` public testnet gate | 65/45/45 | **75/60/45** — two runnable gates (RPC gate, ceremony verifier); mainnet-ready unchanged because there is still no public testnet to point them at |
| `--testnet` local-ci set | — | new, runs the ceremony drill; also implied by `--all` |

## What is still missing, and it is not code

A public testnet needs somewhere to run: validator hosts, a bootnode with a committed
node key and a DNS name, RPC endpoints, a faucet, an explorer and monitoring. The
repo has deployment scripts for most of that and a checklist that is entirely
unchecked. The manifest is the artifact to publish once there is something to publish
about.

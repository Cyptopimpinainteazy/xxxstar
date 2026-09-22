# Full release gate on current master — PASS (2026-09-22)

Run on a clean worktree at `origin/master` = **`006cb5720`** (the tip after this session's twenty-one
PRs), with the pinned toolchain and a dedicated target dir:

```
python3 scripts/mainnet_release_gate.py
```

The gate is all-or-nothing: it collects failures across stages and prints FAIL with the list at the end,
so the verdict covers every stage, including the ones whose output scrolled past in the tail.

## Verdict

```
  ✅ mainnet_release_gate: PASS
```

## Stages observed in the tail

| stage | result |
| --- | --- |
| 2d validator install path | 3 cases accepted, 6 refused, nothing written in check mode |
| 2e release artifacts installable | bundle verifies, extracts, runs, accepted |
| 3 chain-spec / genesis artifacts | both specs found and valid; `production_config()` present |
| 3c shipped genesis boots | 3 validators, finalized height 8, all agree on `0xadf31ff6…` |
| 3b production genesis | builds, boots, finalizes, agrees at height 28 (`0x7fa7a4b9…`) |
| 3d testnet genesis | builds, boots, finalizes, agrees at height 29 (`0xf289e53c…`) |
| 4 critical suites | runtime, supply-ledger, packet-standard, bridge, fees, slash, settlement-engine — all pass |
| 4b panic ratchet | `runtime-hook=0 pallet-call=0 production=515` (baseline `0/0/516`) |
| 5 runtime upgrade rehearsal | migration dry-run passes for every `construct_runtime!` variant |
| 6 reproducible-build prerequisites | srtool installed, docker available, no `SKIP_WASM_BUILD` |
| 6b runtime hash, rebuilt | compact `0x12696572…`, compressed `0x0e6ef363…` — **both match the record** |
| 7 forbidden secrets scan | none |

Earlier stages (2 build, 2b chain runs, 2c validators agree) are covered by the verdict; the release
binaries and the shipped genesis boot are what stages 2d/2e/3x then exercise.

## What this establishes, and what it does not

Establishes: on the current master, the release pipeline's own definition of readiness holds — the nodes
build reproducibly, the shipped/production/testnet genesis boot and agree, the critical pallet suites
pass, the panic ratchet is at or below baseline (one better), runtime upgrades rehearse cleanly, the
srtool rebuild matches the attested hashes, and no secrets are committed.

Does not establish: a published release (nothing is tagged), an external audit, or that the cross-domain
paths are exercised by this gate — the EVM/SVM lifecycles and the anchor/bundle tests live in
`scripts/cross-domain-evm-gate.sh` and the X3-native lifecycle, run separately (`local-ci.sh --cross`).

## The one-command next step for a release

Cutting a release from here means a fresh tag at a committed master revision, then the release-artifact
workflow; that is a user decision (version and visibility), not something to do silently.

# Full release gate on `229d08caa` — PASS (2026-09-22, second run of the day)

`python3 scripts/mainnet_release_gate.py`, run in a clean `/tmp` worktree at
`origin/master` = **`229d08caa`** (the tip after this session's PRs #428–#434), with
the pinned 1.90.0 toolchain and a dedicated target dir. The earlier run of the day
was at `006cb5720`; master has since taken the archival salvages (#428, #429), the
signer extraction (#430), the minimal-RLP EVM signature fix (#431), the
re-attestation (#432), the local-ci classification (#433) and the evidence commit
(#434), so this is the revision the work above actually describes.

The gate is all-or-nothing: it collects failures and prints the verdict at the end,
so the verdict covers every stage even where the output scrolled.

## Verdict

```
  ✅ mainnet_release_gate: PASS
```

## Stages

| stage | result |
| --- | --- |
| 1 required documentation | all required docs present |
| 2 build validation | node 79,296,104 bytes `sha256:a1e37557…`; runtime `sha256:415da4eb…` |
| 2b the chain runs | authored blocks 4 → 64, finalized at 60, CLI answered |
| 2c validators agree | 3 validators, finalized height 8, agreeing on `0xf7b43aff…` |
| 2d validator install path | 3 cases accepted, 6 refused, nothing written in check mode |
| 2e release artifacts installable | bundle verifies, extracts, runs, accepted |
| 3 chain-spec / genesis artifacts | both specs found and valid; `production_config()` present |
| 3c shipped genesis boots | 3 validators, finalized height 10, agree on `0x2a0c2983…` |
| 3b production genesis | builds, boots, finalizes, agrees at height 29 (`0xe28eb828…`) |
| 3d testnet genesis | builds, boots, finalizes, agrees at height 30 (`0x88e55ce9…`) |
| 4 critical suites | runtime, supply-ledger, packet-standard, bridge, fees, slash, settlement-engine — all pass |
| 4b panic ratchet | `runtime-hook=0 pallet-call=0 production=515` (baseline `0/0/516`, one better) |
| 5 runtime upgrade rehearsal | migration dry-run passes for every `construct_runtime!` variant |
| 6 reproducible-build prerequisites | srtool installed, docker available, no `SKIP_WASM_BUILD` |
| 6b runtime hash, rebuilt | compact `0x12696572…`, compressed `0x0e6ef363…` — **both match the record** |
| 7 forbidden secrets scan | none |

## What this establishes, and what it does not

Establishes: on `229d08caa`, the release pipeline's own definition of readiness
holds — the node and runtime build, the shipped/production/testnet genesis boot and
agree, the critical pallet suites pass, the panic ratchet is one better than
baseline, runtime upgrades rehearse cleanly, the srtool rebuild reproduces the
attested hashes, and no secrets are committed.

Does not establish: a published release (nothing is tagged), an external audit, or
the cross-domain paths — those are `local-ci.sh --cross`. In this session the SVM
half of that was run separately and passes in both postures (`cross-domain SVM
(strict posture)` PASS 204s, `SVM contract lifecycle` PASS 191s with 15 assertions
and 0 failures).

## Practical note

The run took 40 minutes, and stage 7 walks the whole tree (`ROOT.rglob("*")` minus
`.git`, `target`, `node_modules`, `.venv`), so the process read ~91 GB over its
lifetime. Output is block-buffered when redirected, so the log stays empty until
the end: run it as `python3 -u scripts/mainnet_release_gate.py` if you want to
watch it, or follow the process tree rather than the log.

# X3 Substrate Security Proof Pack

This directory is for Substrate-native evidence that can be published without overclaiming an external audit.

Run the quick pack:

```bash
./launch-gates/run-substrate-proof-pack.sh
```

Run the heavier compile checks:

```bash
RUN_HEAVY_SUBSTRATE_PROOFS=1 ./launch-gates/run-substrate-proof-pack.sh
```

## Honest Public Labels

Use these labels only when the matching report exists:

| Label | Required Evidence |
| --- | --- |
| Substrate Runtime Upgrade Check: try-runtime PASS | `try-runtime on-runtime-upgrade` log against live or snapshot state |
| FRAME Weights: generated and committed | Benchmark logs plus generated `weights.rs` committed for launch pallets |
| Zombienet Network Smoke: PASS | Zombienet topology/test logs proving validators boot, produce, finalize, and recover |
| Chopsticks Replay Suite: PASS | Fork/replay logs with runtime override or targeted storage mutation cases |
| Runtime Wasm: srtool reproducible build | srtool build log, Wasm hash, compressed Wasm hash, proposal hash |
| Runtime Metadata Diff: subwasm published | subwasm metadata/version/pallet diff for the release runtime |
| Client Compatibility: Polkadot.js/Subxt/PAPI PASS | Client test logs for metadata, storage query, extrinsic submit, event decode |

## Current Non-Negotiable Gaps

- Do not claim external audit completion from these reports. They are internal/self-generated evidence.
- Do not claim try-runtime coverage until the command compiles with `--features try-runtime` and runs against real chain state or a snapshot.
- Do not claim production FRAME weights while launch pallets still rely on manual `Weight::from_parts(...)` placeholders.
- Do not claim deterministic runtime release proof until srtool output and subwasm metadata diff are published.

## Local Tool Status

Installed and verified locally on 2026-04-26:

- `srtool` via `cargo install --git https://github.com/chevdor/srtool-cli --locked`
- `subwasm` via `cargo install --git https://github.com/chevdor/subwasm subwasm --locked`
- `zombienet` via `npm install -g @zombienet/cli`
- `chopsticks` via `npm install -g @acala-network/chopsticks@1.2.8`

The current proof-pack status is tracked in `SUBSTRATE_PROOF_PACK_LATEST.md`.

## Recommended Published Page

For a public security page, link:

- latest `SUBSTRATE_PROOF_PACK_LATEST.md`
- try-runtime upgrade report
- FRAME benchmark report
- Zombienet network smoke report
- Chopsticks replay report
- srtool runtime build report
- subwasm runtime diff report
- client compatibility report
- known risks and external audit status

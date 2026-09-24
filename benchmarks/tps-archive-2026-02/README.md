# X3 chain-level TPS runs, 2026-02

These are the only recorded **chain-level** throughput measurements X3 has:
finalized transactions per second, measured against a running chain, not a
synthetic in-process harness.

They were produced by `scripts/testnet/load-remarks-tps.js` (which this
repository still has) when the workspace was named `atlas-sphere`. They were
absent from the current tree and were recovered on 2026-09-24 from the
`apps/atlas-sphere-clean/benchmarks/` directory of the April 2026 archive on the
external drive, together with `scripts/run-tps-tests.sh` and
`scripts/testnet/load-x3-comit-v2-tps.js`, which were also missing here.

## How they were produced

```bash
RPC_WS=ws://127.0.0.1:9944 \
SENDER_MODE=dev SENDER_COUNT=6 CONCURRENCY=32 DURATION_SEC=20 FINALITY_WAIT_SEC=12 \
node scripts/testnet/load-remarks-tps.js
```

`system.remark` over a loopback WebSocket to one node — the cheapest possible
workload. Finality is inferred from the account nonce delta
(`finalized_method: "account_nonce_delta"`), so "finalized" means the nonce moved
on a finalized block, not merely that the extrinsic was accepted.

## What they recorded

| run | date | concurrency | sent | accepted | finalized | TPS (submit window) | wall TPS | failed |
|---|---|---|---|---|---|---|---|---|
| sweep | 2026-02-19 | 16 | 10,583 | 10,583 | 10,583 | 529.15 | 330.42 | 0 |
| sweep | 2026-02-19 | 32 | 11,510 | 11,510 | 11,510 | **575.50** | 359.17 | 0 |
| sweep | 2026-02-19 | 64 | 11,086 | 11,086 | 11,086 | 554.30 | 345.99 | 0 |
| sweep | 2026-02-19 | 96 | 9,975 | 9,975 | 9,975 | 498.75 | 311.31 | 0 |
| perf-mode | 2026-02-19 | 32–256 | up to 26,175 | 26,175 | 26,175 | up to **581.67** | 435.54 | 0 |
| **7-validator multiprocess** | 2026-02-20 | 1024 (96 senders) | **130,448** | **1,838** | 1,838 | **30.63** | — | **128,610 (98.6 %)** |

Two things stand out and both matter more than the peak:

* the sweep is **flat** — 529 / 575 / 554 / 499 TPS as concurrency goes
  16 → 32 → 64 → 96. The pipeline saturates around 32 and adding concurrency
  makes it slightly worse;
* on a **7-validator** network the same loader at 1024 concurrency had 98.6 % of
  its transactions rejected and finalized 30.6 TPS. The multi-validator number is
  roughly 19x *worse* than the single-host number, which is the gap worth
  investigating before any higher target is discussed.

`x3_vs_solana_chain_tps*.json` is the repository's own comparison against
observed Solana mainnet statistics in the same period (non-vote average
1,279–1,480 TPS, total average ~3.2–3.4k). Both files record
`"winner": "solana"`.

## What these numbers are not

* Not a release-build measurement of current code. The host, build profile and
  exact toolchain of the February runs were **not recorded**, which is the gap
  `TICKET-099` describes. `benchmarks/x3_chain_tps_2026-09-24_debug_2core.json`
  is a current-code rerun of the same configuration with the host and command
  attached, but on a debug build and 2 cores, so it is a floor, not a comparison.
* Not a peak or a guarantee: they are single-host, loopback, `system.remark`.
  No transfer, `.x3`, EVM, SVM or cross-VM workload has a chain-level number.
* Not 100k TPS, and nothing here supports that figure.

# Archived benchmark runs

Moved here from `tools/cross-chain-gpu-validator/benchmarks/` when the two copies
of the validator were merged (2026-10-24). They are kept as history, not as
current results: the dashboard does not read this directory.

**What they measure.** `scripts/benchmark_all_chains_tps.py` posts batches of
synthetic transactions to this host's GPU accelerator lanes
(`localhost:9001-9003/accelerate/batch`) once per chain *profile* and records how
fast the lanes answered. That is local verification throughput. It is not the
throughput of any chain: nothing is submitted to a network.

| file | run | notes |
| --- | --- | --- |
| `all_chains_tps.json` | 2026-02-19, 2362 profiles, one 10k level | `our_chain_max_tps` is 0.0 |
| `all_chains_tps_local.json` | 2026-04-12, 3 profiles, levels 1k/5k | |
| `live_benchmark_report.json` | undated | targets plus processed counts, no per-chain data |
| `COMPLETION_STATUS_REPORT_2026-04-12.md` | 2026-04-12 | status notes from that run |

To show current numbers on the dashboard, run the benchmark on the target host;
it writes `benchmarks/all_chains_tps.json`, which `ccgv dashboard` serves at
`/chain-benchmarks.json` (override with `CCGV_BENCHMARK_PATH`).

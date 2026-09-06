#import "../style.typ": *
#import "../components.typ": *
#import "../data.typ": *

= Performance Evidence

This audit's standard for a performance chapter: show what was actually measured, show what was not, and never fill an empty chart with an invented number. This chapter is a gap table, not a chart, for exactly that reason.

== What Is Actually Measured

#table(
  columns: (1.2fr, 1.4fr, 2.2fr),
  fill: (x, y) => if y == 0 { c-brand } else if calc.even(y) { c-bg-panel } else { white },
  [*Metric*], [*Value*], [*Evidence & scope*],
  [Finalized-transaction throughput], [110.6 finTPS], [`TESTNET_VERIFICATION.md:99` — 2,000 remarks submitted, 0 lost, 18s wall time. Concrete inputs given, not a bare assertion (INFO-04). *Single-host loopback only* — not a cross-host or production-network result.],
  [Workspace compile time], [1m 52s], [Measured live in this audit: `cargo check --workspace`, default features, exit 0.],
  [EVM contract test suite runtime], [approx. 4.6s for the slowest suite], [Measured live in this audit: `forge test --summary`, 169/169 tests passing across 12 suites.],
  [Pallet test suite runtimes], [Sub-second per pallet], [Measured live: cross-vm-router (81 tests, 0.18s), settlement-engine (23 tests, 0.04s), supply-ledger (33 tests), dex (14 tests), lp-locker (19 tests).],
)

== What Is Not Measured — Honestly

#table(
  columns: (1.4fr, 2.4fr),
  fill: (x, y) => if y == 0 { c-brand } else if calc.even(y) { c-bg-panel } else { white },
  [*Metric*], [*Status*],
  [Multi-validator sustained TPS], [Not measured. `.testnet-audit/mesh-2026-09-04/` contains real cold-start/kill-survival evidence (7 validators, loopback), but no sustained-throughput figure for that configuration.],
  [Transaction latency (p50/p95/p99)], [Not measured anywhere found in the repository.],
  [Time to finality under load], [Not measured beyond the single 110.6 finTPS data point's implicit \~18s window for 2,000 remarks.],
  [Mempool capacity under adversarial load], [Not measured. No fuzz/load harness targeting the transaction pool was found.],
  [State growth / disk usage over time], [Not measured. No X3-specific pruning or disk-growth projection exists (state/storage domain finding).],
  [RPC throughput under concurrent load], [Not measured.],
  [Recovery time after crash/restart], [Not measured. `scripts/snapshot-restore.sh` is a manual tool with no timed drill on record.],
  [CPU / RAM / network consumption profile], [Not measured. `crates/x3-bench` exists but benchmarks the x3-lang compiler/optimizer, not node resource consumption.],
)

== Why This Table, Not a Chart

A bar chart with seven empty slots and one real data point would visually imply those slots simply await filling in — normalizing the gap rather than naming it. Presenting the gap as a table with an explicit "Not measured" status, next to the one metric that *is* measured, keeps the honest signal: this repository has exactly one real, correctly-scoped performance data point, and building the rest requires standing up multi-host infrastructure this audit did not have access to.

== What Real Benchmarking Would Require

A credible multi-host performance evaluation needs, at minimum: 3+ physically separate validator hosts (not loopback — `MED-12` in Chapter 6 notes this is the same gap blocking consensus-resilience claims), a load-generation harness driving realistic transaction mixes (not just `system_remark` calls, which the 110.6 finTPS figure used), and a fixed methodology committed to the repository so the next run is comparable to this one. `crates/tps-tracker` exists in the workspace and was not traced in depth this session; it is a reasonable starting point for that harness if it is not already one.

`benchmark-regression.yml` runs `cargo bench` for six named benches (atomic_swap, dex_route, bridge_proof, vm_dispatch, rpc_encoding, signature_verify) but every invocation is suffixed `|| true` — regressions are recorded, never blocking (LOW-06). This is reasonable as a trend-tracking tool but should not be described as an enforced performance gate in any external communication.

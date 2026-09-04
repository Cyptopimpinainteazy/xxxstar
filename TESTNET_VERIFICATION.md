# TESTNET_VERIFICATION.md

Evidence log for this audit run. Every completion claim must trace to a command+output
recorded here. Prior report files in the tree are untrusted until re-run.

Formats:
```
## <VERIFICATION-TOPIC> <date>
Command: <exact>
Result: PASS|FAIL
Evidence: <hash/output/link>
```

## Baseline

### Toolchain
- rustc 1.90.0 / cargo 1.90.0 (rust-toolchain.toml pinned 1.90.0). Confirmed 2026-09-03 via
  `rustc --version`, `cargo --version`.
- Hardware: 32 cores, 109 GiB RAM, 1.8 TB free on /home.

### Git/baseline snapshot
- Repo had NO .git (unversioned snapshot). Created git repo, committed full tree as
  `091dbe3 "Baseline snapshot ..."` on 2026-09-03. 21687 files staged. Working tree clean at baseline.

### Canonical node dev-chain boot (LIVE baseline) — PASS
- Command: `target/release/x3-chain-node --dev --alice --tmp` (prebuilt release binary)
- Result: node booted, JSON-RPC server on ws 127.0.0.1:9944, embedded WASM runtime loaded
  (`require_embedded_wasm("dev")`), began producing blocks from runtime init.
- Evidence: `.testnet-audit/run1/dev-node.log`. First blocks #1-#6 at 23:39:41-42; by 23:58 the
  node had imported #5556+ with `Block finalized` staying ~4 blocks behind head (healthy).
  `Running JSON-RPC server: addr=127.0.0.1:9944` present. Log shows X3 value-add path active:
  "Starting parallel proposer on top of parent"/"Prepared parallel proposal"/"Pre-sealed block"
  on every block, and blocks seal almost instantly (very high head height / minute).
- Interpretation: canonical boot path is real and functional (no fake code on the fundamental
  dev path); dev-spec embeds a working WASM runtime. Block-finalization gap small = healthy.
- Open item: very high block rate vs nominal Substrate 6s — confirm intended (parallel-proposer
  pre-seal is fast by design) vs any timestamp/consensus anomaly; and test across MULTI-validator
  (Alice/Bob/Charlie) + external submit + SOV/quit before calling testnet-ready.

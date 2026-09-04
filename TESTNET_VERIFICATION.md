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

### 7-VALIDATOR FRESH-KEY TESTNET — FULL CONSENSUS (LIVE BASELINE) PASS — 2026-09-04
Reproduced from scratch (no reliance on stale artifacts). Toolchain: added subkey v35 via cargo
install (fixed libclang by symlinking libclang.so in ~/.local/libclang).

Commands (each verified):
- `./scripts/testnet/build-x3-testnet-spec.py 7 --skip-raw`  -> fresh single-master-seed keys +
  plain spec via the env-gated `build-spec --chain=testnet` (X3_TESTNET_AUTHORITIES etc).
- `TESTNET_BASE=/tmp/x3-net4 ./scripts/testnet/run-fresh-validators.sh 7` -> boots 7 validators,
  node1 bootnode (/ip4/127.0.0.1/tcp/30533), peers 2..7 get `--bootnodes`, each started with
  X3_DEV_SEED=<its master seed> so the service inserts Aura+GRANDPA keys (the ONLY mechanism that
  reaches the aura worker; file-only keystore injection does NOT drive authoring on this binary).

Result (verified live):
- All 7 nodes connected; node1 peers=6 (full mesh).
- GRANDPA finalizing continuously: `Block finalized: #473..#484` with head ~#482-484.
- Canonical agreement: chain_getFinalizedHead identical on ALL 7 nodes
  (0x14103f1aba3acea614...) — no divergence; genuine BFT.

Real bugs/defects reproduced & fixed during this bring-up (all in launcher/ops, not runtime):
1. scripts/testnet/x3_testnet_up.sh used stale CLI: --ws-port, --ws-external (rejected),
   --execution=NativeElseWasm (must be native-else-wasm), and no forced node-key (first boot
   dead-ends NetworkKeyNotFound). Fixed + bash -n clean.
2. Node-side `validate_live_json_chain_spec` (load_json_spec) rejects RAW Live specs because it
   requires genesis.runtimeGenesis.config.* which raw specs lack; so plain spec is the correct
   launch file. (Not a runtime defect; a documented launch requirement.)
3. Multi-authority plain spec authoring requires X3_DEV_SEED programmatic insert of aura/gran
   keys; aura authorities are present in block-0 storage but the aura worker only signs when the
   key was inserted through the service (insert_dev_keys_with_seed via maybe_insert_dev_keys).
4. P2P on one host: boot addr must be /ip4/127.0.0.1 (0.0.0.0 is not dialable as a target).
5. (launcher authoring bug, mine) run-fresh-validators.sh dropped ${boot_args[@]} -> peers
   launched without --bootnodes. Fixed; after fix, peering + finality established.

Remaining (recorded as open gaps):
- GRANDPA finality beyond single-node windows is confirmed; longer soak + tx throughput + forced
  failure/recovery tests still to run.
- Validator node keys are random each boot (fine for ephemeral testnet); production nets need
  stable persisted node keys.

- Open item: very high block rate vs nominal Substrate 6s — confirm intended (parallel-proposer
  pre-seal is fast by design) vs any timestamp/consensus anomaly; and test across MULTI-validator
  (Alice/Bob/Charlie) + external submit + SOV/quit before calling testnet-ready.

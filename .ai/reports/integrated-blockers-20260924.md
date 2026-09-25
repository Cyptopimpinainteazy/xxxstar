# X3 integrated blockers — 2026-09-24

One list of what stands between current `master` and Public Testnet Alpha / Mainnet
RC, after the storage, RPC/data-plane and validator-operations audits. Every claim
here is tagged with how it is known:

* **measured** — a command was run in this repository on this date; the command is
  given.
* **recovered** — a recorded result found in the April-2026 archive, with its
  provenance.
* **assumed** — nobody has measured it; treat as unknown.

Nothing here rests on "we think" or on a document that asserts a number without a
producer.

---

## 1. Merged in this pass

Eleven commits across four pull requests: #501, #503, #504, #505.

| area | what landed | evidence |
|---|---|---|
| state snapshots | `crates/x3-state-snapshot`: manifest, chunk verifier, state-root recomputation, exporter, `hash-manifest`/`root`/`build`/`verify` CLI | 40 unit tests; `launch-gates/snapshot-murder-test.sh` 13/13 |
| snapshot root | recompute matches a **chain-produced** root | measured: header `0x46b7cb92…` vs rebuilt from the node's own `build-spec --chain dev --raw`, byte-identical (130 entries) |
| node RPC | authority refuses `--rpc-methods unsafe` on a non-loopback listener | 5 unit tests; `sc_rpc_server::deny_unsafe` is address-independent for `Unsafe` |
| node storage | free-space guard, startup gate, watchdog | `node/src/disk_guard.rs`, 9 unit tests |
| RPC cost | `eth_getLogs`/`x3_getEvmLogs` block-range cap; both priced in the limiter | 4 + 2 unit tests |
| migrations | treasury/agent-memory/agent-accounts read the declared `STORAGE_VERSION`; five unreachable `migrations.rs` deleted; gate added | `scripts/ci/check_migration_modules_are_wired.sh` |
| per-block metrics | `x3_imported_block_bytes`, `x3_imported_block_extrinsics`, `x3_block_interval_seconds` | measured live (below) |
| orchestrator | the exported-but-unused `ProofVerifier`/`VmExecutor` traits are now the adapters' real extension point; honest default preserved | `cargo test -p x3-orchestrator` 7 passed |
| external-chains | Arbitrum test's anvil readiness loop retries instead of panicking | 4 passed, needs `anvil` |
| tooling | pytest `testpaths`/`.kilo` fix, `tomli` fallback in two scripts, `requirements-dev.txt`, `proof_report` enforcement, explorer + super-ide dependency alignment | see §2 |

---

## 2. Measurements established this session

**State root.** `x3-state-snapshot` reproduces a chain-produced genesis root
exactly: `0x46b7cb9219500cbe54c5d32b216376e6e1f17acc874108cad18951214e5b4038`
(dev chain, spec_version 20). `scripts/mainnet/verify-snapshot-root-against-chain.sh`
automates this against any node → MATCH.

**Per-block storage, on a dev chain (measured, live `/metrics`):** ~215.6 bytes per
block and ~1.44 extrinsics per block while idle; **893.9 bytes and 6.48 extrinsics
per block under load**; **200 ms block interval** in both cases. Consequence: raw
block bodies are ~34 GB/year at idle sizes, **not** the ~1.58 TB/year the audit's
10 KB/block estimate implied. A dev chain is a floor, not a production workload.

**Chain-level finalized TPS** (`system.remark`, loopback WebSocket — the cheapest
possible workload):

| run | source | finalized TPS | failures |
|---|---|---|---|
| sweep, concurrency 16/32/64/96 | recovered (`benchmarks/tps-archive-2026-02/`) | 529.2 / **575.5** / 554.3 / 498.8 | 0 |
| perf-mode, concurrency 128 | recovered | **581.7** (best ever recorded) | 0 |
| 7-validator multiprocess, concurrency 1024 | recovered | **30.6** | **128,610 of 130,448 (98.6 %)** |
| current code, debug build, 2 cores | measured 2026-09-24 | 38.2 | 0 |
| **current code, release build, same host** | measured 2026-09-24 | **47.1 / 51.3** (two runs) | 0 |

Two facts matter more than the peak: the sweep is **flat** (adding concurrency past
32 makes it worse), and the multi-validator number is **~19x worse** than the
single-host one. The repository's own comparison file also records
`"winner": "solana"` against observed Solana mainnet (non-vote avg 1,279–1,480 TPS).

**The February figures are not reproducible on the current dev chain.** Same loader,
same nominal configuration (6 senders, concurrency 32, 20 s, loopback), release
build: **47–51 finalized TPS**, where the February archive recorded **575.5**. A
release build is only ~25–35 % faster than the debug build here (38.2 → 47–51), so
the build profile was never the constraint. The block cadence is not the constraint
either: the chain holds **200 ms** blocks. What it packs into them is — **6.48
extrinsics per block**, i.e. ~32 tx/s of block space, against the ~115 per block
that 575 TPS would require. Finding out why a 200 ms block carries ~6 remark
extrinsics is the single highest-value throughput question this session surfaced,
and it needs the attribution metrics below (or a txpool ready-queue metric, which
the node does not export either).

**100K TPS is not supported by any measurement here.** It is ~172x the best
single-host figure and ~3,270x the 7-validator figure.

**Storage metrics that exist versus not** (measured by scraping a live
`--prometheus-external` endpoint):

* present: `substrate_block_verification_and_import_time`, `substrate_block_height`,
  `substrate_state_cache_bytes`, `substrate_database_cache_bytes`,
  `trie_cache_{shared,local}_{hits,fetch_attempts}`, `trie_cache_shared_update_duration`,
  the `x3_*` counters, and the three new per-block histograms.
* absent: `storage_reads`, `storage_writes`, `storage_read_bytes`,
  `storage_write_bytes`, `trie_nodes_read`, `trie_nodes_written`, `state_root_us`,
  `db_commit_us`, `db_flush_us`, `state_growth_bytes` (host disk metrics are out of
  scope for a node).

**Node default database.** Running the node with no `--database` flag logs
`Database: ParityDb at <base>/chains/<chain>/paritydb/full`. The validator runbook
said RocksDB was the default; that is corrected. Which backend *should* be the
default is unmeasured.

**Repository hygiene.** `python -m pytest` → 201 passed (was 1412 collection errors
and zero tests run). `npm test`, `pnpm test`, `pnpm build` → exit 0. `cargo test
--workspace --no-fail-fast` → 6,224 tests passed, 2 failed, and both failures are
the load-marginal throughput assertions in `x3-gpu-validator-swarm` (19.8K/38.1K
TPS under load; the same file passes 7/7 when run alone on an idle box).

---

## 3. Open blockers

### Infrastructure — cannot be closed in code

1. **No deployable bootnodes.** `scripts/ci/check_deployable_bootnodes.sh` reports
   4 findings: `deployment/chain-specs/x3-testnet-raw.json` is Live with **no**
   bootNodes; `fresh/x3-testnet-plain.json` and the k8s ConfigMap carry `127.0.0.1`;
   the ConfigMap is additionally a Live spec with an **empty `genesis.raw.top`**.
   Every candidate hostname checked does not resolve
   (`bootnode.testnet.atlas-sphere.io`, `bootnode.testnet.x3-chain.io`,
   `rpc.testnet.atlas-sphere.io`). *Closes when real multiaddrs exist and the gate
   is wired into `local-ci`.*
2. **Release signing is claimed, not present.** `.artifacts/release-v1.1/` (v1.1.1,
   commit `55d09edb6`) has checksums that verify, and `RELEASE_MANIFEST.json` says
   `"signed": true` — but **no signature file exists anywhere in the bundle**.
3. **The shipped release binary cannot start.** `--dev` panics
   (`Authorities are already initialized!`); the bundled spec fails with
   `SessionKeys_generate_session_keys is not found`. Both reproduced.
4. **`solana-gpu-validator-v1.0.tar.gz` does not exist** on the archive drive that
   holds the other April artifacts, confirming the ledger's `GAP-GPU-CLAIMS`.
5. **`docker` and `srtool` are absent** on the machine used here, so the `--release`
   and `make mainnet-check` gates cannot run at all. `make srtool-install` restores
   `srtool`; `--release` failures that need reproducibility cannot be measured until
   then.
6. **WAN / geographic proof absent.** Every number in this document is loopback or
   single-host. No evidence exists for 200 ms slots across WAN latency.

### Code — reachable, not yet done

7. **Attribution metrics.** The node times a whole import and nothing inside it. To
   answer "which stage is the bottleneck" needs `execution_us`, `state_root_us`,
   `db_commit_us` and storage read/write counts, which means wrapping the executor
   or `BlockImport`, or patching the client. *This is the audit's §3 question and it
   is still unanswerable.*
8. **`state_growth_bytes`.** Not measurable from the import notification; needs the
   state diff. Block bodies turned out to be small (§2), so state growth is the
   number that decides disk economics — and it is unmeasured.
9. **Snapshot restore path.** Verification and export exist; nothing writes a
   snapshot's entries back into a database and re-reads the root. The audit's
   export/import pair is half-built.
10. **Disk-pressure authoring stop.** The guard warns and gates startup, but does
    not take an authority out of authoring. `sc-keystore`/`sp-keystore` expose **no
    key-removal API**, so the usual mechanism is unavailable; this needs a
    keystore-wrapper or `BlockImport` design, not a small patch.
11. **Power-loss and disk-full drills.** None executed. The audit's §14–15.
12. **Storage-blob economics.** No storage deposit/rent/TTL mechanism exists, so
    state-bloat cost is unbounded for the attacker side of §22–23.

### Repository artefacts that mislead

13. **`crates/import-queue-wrapper` (616 lines) is wired into nothing.** The only
    references anywhere are inside a stale nested `.kilo` checkout.
14. **`substrate` is a symlink to `.`** (mode 120000, added by the 2026-09-03 bulk
    snapshot `143f7a6b`). Nothing references `substrate/<path>`; it makes any
    symlink-following tool count this 137 GB tree twice. The 127 GB behind it is
    this workspace's own `target/`, not a second checkout.
15. **`x3-gpu-validator-swarm`'s throughput assertions are load-marginal.** They fail
    under contention and pass on an idle box. This is a maintainer decision —
    benchmark assertions in the default test set — not something to silently
    re-tune.

### Documentation that still contradicts the code

16. `LAUNCH_SCOPE.md` declares itself authoritative and supersedes
    `CURRENT_MAINNET_STATUS.md`, which in turn declares `FEATURE_REGISTRY.toml`
    canonical. Only the registry is checked by a gate
    (`scripts/check-readiness-consistency.sh`).
17. The GPU phase reports (`.artifacts/P4_GPU_TEST_PHASE{1,2}_REPORT.md`, 2026-03)
    quote pure-Python CPU mocks against GPU targets — one threshold was explicitly
    **lowered** to 1M hash/sec to pass — and state "100+ TPS CPU (GPU target:
    >1000)". They are not evidence of accelerator throughput.

---

## 4. What this document does not claim

* **No production workload number.** Every TPS figure is `system.remark` on
  loopback. Transfers, `.x3` execution, EVM, SVM and cross-VM paths have no
  chain-level measurement at all.
* **No multi-node throughput or storage number.** The 7-validator TPS figure is
  from the recovered archive; every storage observation in this document is a
  single dev node.
* **No comparable-to-February number.** The release build was measured on this
  2-core desktop, and the February host and build are unrecorded. 47–51 here versus
  575 then is a discrepancy to explain, not a like-for-like regression figure.
* **No claim that the snapshot format is complete:** it verifies, exports and
  cross-checks against a real chain root, and cannot yet restore.
* **No reproducibility claim.** `srtool` is installed, but it builds inside Docker
  and docker is absent on this machine, so the release-reproducibility gate has
  never executed here.

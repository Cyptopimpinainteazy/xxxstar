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

Eleven commits across four pull requests: #501, #503, #504, #505 — then #506
(this document), #507, #508 (txpool gauges) and #509 (snapshot restore).

| area | what landed | evidence |
|---|---|---|
| state snapshots | `crates/x3-state-snapshot`: manifest, chunk verifier, state-root recomputation, exporter, **restore**, `hash-manifest`/`root`/`build`/`verify`/`restore` CLI | 48 unit tests; `launch-gates/snapshot-murder-test.sh` 20/20; `scripts/mainnet/verify-snapshot-restore.sh` PASS |
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

**State root.** `x3-state-snapshot` reproduces a chain-produced root exactly, at
genesis (`0x46b7cb9219500cbe54c5d32b216376e6e1f17acc874108cad18951214e5b4038`,
dev chain, spec_version 20) **and at a real block**: `export-state` at block 6290
of a dev chain (12,526 entries) recomputes to that header's own root,
`0xbe94def3c9cdaecc8ab5efb94e9a3a49a526dff6b881a1b35f7869de2c60ecbb`.
`scripts/mainnet/verify-snapshot-root-against-chain.sh` automates the genesis
comparison against any node → MATCH.

**Snapshot restore, end to end against a running chain.** #509 closed the
export/import pair. `scripts/mainnet/verify-snapshot-restore.sh` starts a chain,
exports state at a real block, builds a snapshot anchored to it, restores it into
a raw chain spec, and boots a second node from that spec — the node recomputes
the chain's own state root out of its genesis header and reports the restored
`:code`'s `spec_version`. A chunk with one flipped byte is refused and leaves no
spec behind. `launch-gates/snapshot-murder-test.sh` went 13/13 → **20/20**, with
the restore refusals (corrupt chunk, substituted state under a forged manifest,
wrong chain, stale, and no silent overwrite of an existing spec) demonstrated on
disk through the shipped binary.

**Two limits found while proving it.** (a) A restored spec's genesis header is
number 0 while its state is block N's, so a node booted from it serves that state
and then refuses to author — `frame-system` panics, "Block number must be
strictly increasing" (reproduced). Restored state is state *transport*, not a
join-at-height path. (b) `export-state` on a node with bounded state pruning
fails with `UnknownBlock("State already discarded")` for any block whose state has
been pruned (reproduced at block 2616 of a node that had run for 10 h, and it
succeeds at the head). A snapshot source has to archive state, or export at the
head.

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

**The February figures are not reproducible here — and the limit is the load
generator, not the chain.** Same loader, same nominal configuration (6 senders,
concurrency 32, 20 s, loopback), release build: **47–51 finalized TPS** where the
archive recorded **575.5**. Three candidate explanations were tested and all three
are excluded by measurement:

* *build profile* — release is only 25–35 % faster than debug here (38.2 → 47–51);
* *block cadence* — the chain holds **200 ms** blocks, exactly the target;
* *block space* — **excluded**. The txpool drained at least as fast as it filled:
  `submitted_transactions` +1106 against `block_transactions_pruned` +1309 over the
  same window, 0 failures, 0 invalid. The chain included everything it was offered,
  so **6.48 extrinsics per block is the load, not the capacity**.

What moves the number is the number of signers, not concurrency:

| senders | concurrency | finalized TPS |
|---|---|---|
| 6 | 32 / 128 / 512 | 50.5 / 46.1 / 53.1 |
| 24 | 256 | 77.0 |
| 48 | 512 | 99.6 |
| 96 | 1024 | 91.4 |
| 192 | 1024 | 149.6 (submit window) / 73.2 (wall) |

A 16x increase in concurrency changes nothing; quadrupling and then doubling the
signer count raises it steadily. That is the signature of a **client-bound**
measurement: the JavaScript load generator signs and submits from the same two
cores the node is using. The ceiling measured here is therefore the harness's, and
**the chain's own ceiling is still unmeasured** — including whether it is anywhere
near the 575 the February host recorded.

At 192 senders the chain's own counters show **14.12 extrinsics per 200 ms block**
(up from 6.48 at the light load), and once again **no backlog**: pool submissions
3,082 against 3,318 included over the same window, 0 failed. The chain absorbed
everything it was offered again. Its measured inclusion rate was ~70 tx/s, which
matches the wall figure of 73.2 and is the number to quote; 149.6 is the
submit-window artifact described next.

**Running more loader processes does not change it.** Three processes, 85 senders
each (255 total, prefunded once via `ONLY_PREFUND=true` to avoid a nonce race),
produced 3,418 finalized transactions, 0 failed, and an aggregate wall rate of
**74.6 TPS** — against 73.2 for a single process with 192 senders. Across every
configuration tried, from 6 senders in one process to 255 across three, the wall
figure sits between **73 and 97 TPS**, and the pool never fills.

The machine is the limit, and it says so: `nproc` is **2**, and the load average
during these runs was **4.8–6.9**. The node and the load generator are competing for
the same two cores. That is why every configuration converges on the same number —
it is this box's capacity to run a node and a client at once, not a property of X3.

**A metric caveat that applies to every TPS figure in this document.** The loader
reports two numbers: `finalized_tps_submit_window`, which divides finalized
transactions by the *submit* duration and ignores the finality wait, and
`finalized_tps_wall`, which divides by wall time including it. For a chain that
keeps producing blocks while a backlog drains, only the wall figure describes
steady state. The archive's headline 575.5 is a submit-window number — its own wall
figure was 359.2 — so the February comparison is 359 versus 73, about 5x, rather
than 575 versus 47.

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
9. ~~**Snapshot restore path.**~~ **Closed by #509** and proven against a live
   chain (§2). The half that remains is the one this does not pretend to be: the
   restore writes a raw chain spec whose genesis *state* is block N's state, so a
   node that boots from it recomputes the same root and then refuses to author
   (its header is number 0). Installing that state behind block N's header — the
   path a validator joining at a height actually needs — is state/warp sync, and
   does not exist here. `export-state` also needs unpruned state or the head
   (measured).
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
  2-core desktop, where the load generator competes with the node for both cores.
  47–51 here versus 575 then is a harness-capacity difference, not a like-for-like
  regression figure — and disentangling the two needs a load host that is not the
  node's host.
* **No claim that state sync is finished.** The snapshot format verifies,
  exports, cross-checks against real chain roots and restores into a bootable
  chain spec, but it is state *transport*: nothing installs that state behind a
  block header, so a validator still cannot join a running chain at height N with
  it. Warp/state sync is the missing client-side half.
* **No reproducibility claim.** `srtool` is installed, but it builds inside Docker
  and docker is absent on this machine, so the release-reproducibility gate has
  never executed here.

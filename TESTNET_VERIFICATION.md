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

### END-TO-END EXTRINSIC FINALITY ON 7-VALIDATOR NET — PASS — 2026-09-04
After consensus established on the 7-validator net, submitted signed `system.remark` extrinsics
from an endowed fresh-key account (sr25519 of validator-1 master seed) to node-1
(ws://127.0.0.1:9950) via packages/ts-sdk/batch-remark.mjs, nonce-tracked, awaiting per-tx
finalization.
- Result: SENT=120 FINALIZED_EVENTS=120 finalNonce=120 delta=120 (0 lost). Nonce delta confirms
  every tx reached a GRANDPA-finalized block. ~1.04 finalized/s is an artifact of the sequential
  await-per-tx loader, NOT a chain ceiling (peak measured separately ~50+/s parallel).
- Account funded: free=1000000000000000000 (1 unit); nonce advanced 0->120.

### FAILURE-INJECTION / RESILIENCE — 2026-09-04 (OS-level single-node kill)
Deterministic solo-join net (run-solo-join.py 7 40) converged 7/7 (identical head #283 and
finalized head 0xf1a6f7fa…). Killed one leaf validator (sj3). Survivors continued authoring +
finalizing (~#283 -> ~#0x1c6) but the majority net PARTITIONED into two finalized branches
({sj1,sj2,sj4}=0xb3334567… vs {sj5,sj6,sj7}=0xf93d5110…); sj1 had 5 peers yet did not bridge.
=> FINDING GAP-P2P-1: default local libp2p graph is sparse/star-ish and does not auto-heal into
a full mesh on this host; single-node loss can fork a >2/3-majority network. Production/public
resilience needs explicit reserved/full-mesh or public-addr+kademlia wiring.
Also reproduced (and recorded in ledger): cold-start from empty genesis with all validators
concurrent can race light forks; deterministic recipe = solo-lead node-1 then join (run-solo-join),
which converges 7/7 cleanly.

### SUSTAINED FINALIZED TPS ON 7-VALIDATOR NET — PASS — 2026-09-04
Parallel loader packages/ts-sdk/par-load.mjs (bounded in-flight, nonce-delta finalized count).
Senders = separate fresh-key endowed accounts (validator-2/3 master seeds).
- count=300 limit=128: delta=300 lost=0, 23.3 finTPS (13s)
- count=2000 limit=200: delta=2000 lost=0, 110.6 finTPS (18s)  <- genuine sustained result
Canonical finalized head identical on all 7 nodes during load (0x9e96984ab550e508); heads within
1 block. Verified 100% finalized inclusion with zero loss at >100 finalized TPS.
- Validator node keys are random each boot (fine for ephemeral testnet); production nets need
  stable persisted node keys.

- Open item: very high block rate vs nominal Substrate 6s — confirm intended (parallel-proposer
  pre-seal is fast by design) vs any timestamp/consensus anomaly; and test across MULTI-validator
  (Alice/Bob/Charlie) + external submit + SOV/quit before calling testnet-ready.

### RESERVED FULL-MESH — FIX FOR INTERMITTENT COLD-START FORK — PASS — 2026-09-04

**Purpose.** Close GAP-CONSENSUS-REPRO-1 / GAP-P2P-1 / GAP-MESH-1: the default local libp2p
graph (sparse/star) intermittent-forks on cold-start and partitions on a single-member loss.
The fix gives each validator a DETERMINISTIC `--node-key` and wires a RESERVED FULL MESH so
every node holds a hard-reserved connection to every other node before authoring begins — no
leaf, no single point of failure.

**Deterministic PeerId derivation (ground truth, from the node itself).** This binary requires
the `/p2p/<PeerId>` suffix on `--reserved-nodes` (otherwise "Peer id is missing"). PeerId is not
derivable inside the CLI from the key alone, so `run-mesh.py` boots each node once (sequentially,
throwaway state) with a fixed deterministic `--node-key` (the 32-byte ed25519 secret equal to the
node index: `0x…0001` … `0x…0007`), reads `system_localPeerId`, and shuts it down; the result is
reproducible because the key is fixed. Confirmed stable & distinct:

| node | node-key (ed25519)            | PeerId | dial addr port (reserved) |
|------|-------------------------------|--------|---------------------------|
| 1 | `0x…0001` | `12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp` | 30633 |
| 2 | `0x…0002` | `12D3KooWHdiAxVd8uMQR1hGWXccidmfCwLqcMpGwR6QcTP6QRMuD` | 30634 |
| 3 | `0x…0003` | `12D3KooWSCufgHzV4fCwRijfH2k3abrpAJxTKxEvN1FDuRXA2U9x` | 30635 |
| 4 | `0x…0004` | `12D3KooWSsChzF81YDUKpe9Uk5AHV5oqAaXAcWNSPYgoLauUk4st` | 30636 |
| 5 | `0x…0005` | `12D3KooWSuTq6MG9gPt7qZqLFKkYrfxMewTZhj9nmRHJkPwzWDG2` | 30637 |
| 6 | `0x…0006` | `12D3KooWMz5U7fR8mF5DNhZSSyFN8c19kU63xYopzDSNCzoFigYk` | 30638 |
| 7 | `0x…0007` | `12D3KooWE3quQCP6Xu7eXpcmmpwVS1KofWnPCBWYNHCswgaqwCso` | 30639 |

Dial address form used: `/ip4/127.0.0.1/tcp/<port>/p2p/<PeerId>`.

Reproducibility: node-1's derived PeerId matches what `system_localPeerId` reports for the same
key on an independent earlier boot (deterministic). Cache keyed by node-key under the run base.

**Harness.** `scripts/testnet/run-mesh.py` (modes `derive` / `cycles` / `kills`). Launch flags per
node: `--node-key` (fixed) + `--reserved-nodes /ip4/127.0.0.1/tcp/<p>/p2p/<PeerId>` for ALL other
nodes, plus `X3_DEV_SEED=<suri> --unsafe-force-node-key-generation --validator --force-authoring
--allow-private-ip`; PLAIN spec (NOT raw) `deployment/chain-specs/fresh/x3-testnet-plain.json`;
loopback dials; `--execution=native-else-wasm`. Same key/spec/authoring path already validated.

**Proof A — REPEATED COLD-START CONVERGENCE (8/8 clean).** Each cycle boots all 7 concurrently
from wiped genesis with the full reserved-mesh wiring, waits for GRANDPA finality:

| cycle | converged | unique finalized heads | head (per-node all-identical) | ~time |
|-------|-----------|------------------------|-------------------------------|-------|
| 1 | YES | 1 | `0xbd701e692f9b77` (peers=6/6) | 20s |
| 2 | YES | 1 | `0xdc236eb9abb4df` | 16s |
| 3 | YES | 1 | `0x9f74e8b5e9d265` (peers=6/6) | 18s |
| 4 | YES | 1 | `0x316b63e9a4d350` (peers=6/6) | 20s |
| 5 | YES | 1 | `0x5cd8e418cdcaed` (peers=6/6) | 20s |
| 6 | YES | 1 | `0xba080887b7e6c3` (peers=6/6) | 18s |
| 7 | YES | 1 | `0xe481ee0ee6a5b2` | 14s |
| 8 | YES | 1 | `0xf04c1f57ed2eca` | 14s |

8/8 cold-start cycles produced exactly ONE GRANDPA-finalized head across all 7 nodes — this is
the property the old sparse path intermittently failed (GAP-CONSENSUS-REPRO-1).

**Proof B — SINGLE-MEMBER-LOSS SURVIVAL (7/7).** For each victim 1..7, boot a fresh converged
full mesh, kill that one validator, and require the 6 survivors to keep GRANDPA-finalizing ONE
chain within 60s:

| killed node | survivors finalized a SINGLE chain? | survivor set / finalizing head |
|-------------|--------------------------------------|--------------------------------|
| mesh1 | YES (6/6) | `0x8cfcfcbb1302c8` |
| mesh2 | YES (6/6) | `0x826e0db2cab669` (8s) |
| mesh3 | YES (6/6) | `0x113f56d2b8b121` |
| mesh4 | YES (6/6) | `0xcfefdc35f428c3` |
| mesh5 | YES (6/6) | `0xd6d1ba606063d3` |
| mesh6 | YES (6/6) | `0xf92b65b65b0bb5` |
| mesh7 | YES (6/6) | `0xdaaba1d50478f9` |

7/7 single-member losses left the fully-connected >=5/7 majority finalizing ONE chain (no fork)
— versus the pre-fix sparse graph that partitioned into two finalized sets on a single leaf loss
(GAP-P2P-1).

**Conclusion.** RESERVED FULL MESH with deterministic node-keys fixes the intermittent
cold-start fork and single-loss partition on this host's loopback testnet. Logs/output captured
from `run-mesh.py cycles --count 7 --cycles 8` (8/8) and `run-mesh.py kills --count 7` (7/7).
Note: proof is on a single 127.0.0.1 host (loopback); production public-net resilience also
needs real cross-host connectivity + stable on-disk node keys, but the P2P-structure root cause
(sparse graph) is addressed deterministically here.

---

# X3-LANG RECONCILE 2026-09-04

Reconciled the `x3-lang/` compiler + x3-vm subsystem between the LOCAL tree
and the USB lineage (`/media/lojak/USB-Drive/home/x3star/Desktop/xxxstar-main`).
Both are listed Critical X3 Systems (`.x3` language compiler + x3-vm
executor/verifier). Scope = `x3-lang/` only. Commits: `f55fa53a`.

## Direction facts (independent of this task, verified during work)
- `x3-lang/` is its own Cargo **workspace** (members: x3-common, x3-lexer,
  x3-ast, x3-tools, compiler, vm). No crate in the repo root workspace has a
  path/`[patch]`/name dependency on any `x3-lang-*` crate, so the reconciliation
  is fully self-contained: nothing outside `x3-lang/` can regress from it.
- USB and LOCAL forked from a common base. Measured relationships:
  - **USB lineage (June)** = the *finished cross-VM* compiler: keeps the B-52
    DSL in-language and the full B-52 `Operation` set in `ir.rs`
    (`RouteScore`, `SolverBid`, `RelayerAttest`, `RpcConsensus`, `RiskScore`,
    `InvariantCheck`, `PrivacyCommit`, `ProofRequired`, `VmAdapterCall`,
    `ModeCheck`, `PackageImport`, `RefundPolicy`) and the full inline
    `semantic.rs` safety/verify engine (refund paths, explicit finality,
    proof requirements, invariant rules `InvariantRule`/`get_builtin_invariants`,
    route-score, mainnet-mode gating `CompilationMode`, risk scoring
    `RiskScore`/`compute_risk_score`), plus `formatter.rs`, `linter.rs`,
    `risk.rs`, B-52 examples, and its own test corpus.
  - **LOCAL lineage (Sept)** independently *removed* the B-52 IR surface + mode
    gating and added its own **orthogonal** compiler layers: `verify.rs`
    (structural IR verifier `verify_ir`), `numeric.rs` (integer literal /
    coercion policy `verify_numeric_policy`), `diagnostic.rs`
    (`CompilerDiagnostic`/`DiagnosticCode`/`DiagnosticSeverity`) + their tests
    + a `tests/conformance/` suite.
  - => BIDIRECTIONALLY diverged, NOT a simple superset.

## Disposition
Adopted / kept-local / reconciled:
- **Adopted from USB (base):** `compiler/src/{emitter,ir,lowering,parser,
  semantic,regalloc,intent_emit}.rs`, the USB-only modules
  `compiler/src/{formatter,linter,risk}.rs`, `crates/x3-{ast,common,lexer}`,
  `spec/opcodes.{rs,yaml}`, `x3-tools/src/bin/x3c.rs`, `vm/{executor,
  verifier,x3_lang_vm}.rs`, their test/fixture corpora, and `examples/*`.
- **Kept from LOCAL (orthogonal first-class modules):**
  `compiler/src/{verify,numeric,diagnostic}.rs` plus the compiler test targets
  `test_ir_verifier.rs`, `test_numeric_policy.rs`, `test_diagnostics.rs`,
  `test_conformance.rs` and the `tests/conformance/` fixtures. All remain green.
- **Reconciled (no duplication, no dead code):**
  - USB semantic verify engine and LOCAL `verify_ir` are **complementary
    layers**, not duplicates (whole-program semantics vs structural IR
    scoping/amounts). Both are kept; the semantic engine is the top-level
    compile/check gate (USB canonical), while `verify_ir`/numeric/diagnostic
    are exposed as first-class public modules exercised by their integration
    test targets.
  - Extended LOCAL `verify::verify_ir`'s match to be exhaustive over the
    restored B-52 leaf `Operation`s (added to its no-op group) so it compiles
    against the merged IR.
  - Merged `compiler/src/lib.rs`: module set = USB modules + `verify`,
    `numeric`, `diagnostic`; preserves USB's public API (`compile_with_mode`,
    `check_source_with_mode`, `CompilationMode`/`InvariantRule`/`RiskScore`
    re-exports, four-arg `verify_with_config`).
  - Deliberate conflict resolution: USB's sanctioned example
    `examples/flagship_b52.x3` lowers a swap `min_output` to default `0`, which
    LOCAL's `verify_ir` rule ("swap min_output must be > 0") rejects. To not
    regress the USB-finished example corpus, `verify_ir` is kept as an
    exported/tested structural pass rather than force-injected into the
    canonical `compile_*` pipeline (which it would break). The USB semantic gate is
    the compiler's production safety gate.

## Green proof (run in `x3-lang/`, all clean)
```
cargo build --workspace                                   # EXIT 0
cargo clippy --workspace --all-targets -- -D warnings     # EXIT 0 (no warnings)
cargo test --workspace                                    # 34 suites, 0 failed
cargo fmt                                                 # EXIT 0
```
Fake-code scan on authored/merged files (`lib.rs`, `verify.rs`, `numeric.rs`,
`diagnostic.rs`): clean.

Test proof highlights (all `ok`): compiler lib unittests 43 (USB semantic
engine incl. mode/risk/invariant), `b52_test` 33, `executor_tests` 30,
`test_compiler_pipeline` 16, `test_parser_coverage` 24, `cli_integration` 8,
`test_e2e_examples` 10, and LOCAL-side `test_ir_verifier` 8, `test_numeric_policy`
5, `test_conformance` 1, `test_diagnostics` 3.

## Files changed (commit f55fa53a)
23 modified + 21 added inside `x3-lang/` (USB adoption + merged lib.rs +
verify_ir exhaustiveness). No files changed outside `x3-lang/`.

## Remaining blockers / out-of-scope
- The two lineages genuinely disagree on whether a swap may lower `min_output=0`
  (structural verifier) vs default-accepted (USB semantic). Resolved in favor
  of the USB-finished example; revisit if a stricter emission gate is desired.
- USB source working tree contains stray non-compiler artifacts (`ralph.py`,
  `ralph_*.txt`, `x3_dashboard.html`) — excluded from this merge (out of
  scope, not part of the compiler/VM subsystem).

## X3-LANG-RECONCILE-GREEN-2026-09-04
Command (x3-lang/): `cargo build --workspace` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo test --workspace`
Result: PASS (all EXIT 0)
Evidence: build Finished 0.18s; clippy 0.23s no warnings; test: every `test result: ok`, 0 failed (incl USB b52_test/executor_tests/parser_coverage + LOCAL test_ir_verifier(8)/test_numeric_policy(5)/test_conformance(1)/test_diagnostics(3)).
Root-integration: x3-lang is a self-contained separate workspace; no root-workspace crate path- or name-depends on any x3-lang-* crate (only x3-compiler/tests/schema_validator.rs reads x3-lang/schema.json as a data fixture). Reproduced 2026-09-04 by parent session; commits f55fa53a + f4c4ddc8 from reconcile subagent, tree clean.

## X3-ATOMIC-SWAP-X3VM-RECONCILE 2026-09-04
Reconcile of USB-finished `crates/x3-atomic-swap` + `crates/x3-vm/src/bridge.rs` vs local
audited/testnet tree. Working tree was git-clean at 90d16bff. Source of record (READ-ONLY):
`/media/lojak/USB-Drive/home/x3star/Desktop/xxxstar-main`.

Commands (all from repo root):
- `cargo clippy -p x3-atomic-swap -p x3-vm --all-targets --features x3-atomic-swap/std -- -D warnings` → EXIT 0
- `cargo test -p x3-atomic-swap --features std` → 0 failed (lib 649, atlas_htlc 1, chaos 31, integration 44)
- `cargo test -p x3-atomic-swap` (default) → 0 failed (lib 638, chaos 31, integration 44)
- `cargo clippy -p x3-vm --all-targets -- -D warnings` → EXIT 0
- `cargo test -p x3-vm` → 0 failed (lib 148 incl. new universal_escrow suite, gpu 8)
- `cargo check --workspace` → EXIT 0 (no regression across all x3-vm / x3-atomic-swap reverse deps incl. node, x3-relayer, x3-bridge-adapters, pallets, chain-health-daemon)
- `cargo test -p x3-chain-health-daemon` (only reverse dep of x3-atomic-swap) → EXIT 0 (4)

Direction-facts verified per file: every one of the 18 differing *_htlc.rs/scoreboard files differed
from local ONLY by the new `cross_adapter_atomicity_test` readiness field (+ matching readiness
test assertions); adapter.rs & lib.rs differ structurally; x3-vm src/bridge.rs local was a pure subset
of USB. Not re-derived by me (parent measured 2026-09-04); I confirmed each file diff directly.

Adopted from USB (commit b1cb60ed): typed `VmFamily` engine + readiness incl.
`cross_adapter_atomicity_test`, `VmType::WasmL1`, whole-file readiness propagations across all
*_htlc adapters + scoreboard + chaos test, cross-adapter atomicity tests, scoreboard dynamic
overall-% fix, clippy lint fixes (atlas test borrow; no behavior change).
Adopted from USB (commit ac70bbb4): x3-vm bridge symmetric `unwind_*_lock` hooks + 0x32/0x33
universal cross-VM hostcalls + the single required supporting module `universal_escrow.rs`.

Kept local verbatim: `evm_live.rs`/`btc_live.rs` (+ their std-gated lib.rs mods) — USB lacked them.
Not adopted (noted conflict): `tests/state_machine_proptest.rs` (USB-unvalidated: fails against the
shared, out-of-scope `intent.rs` where `Expired` is terminal yet lists `Refundable/Failed` as valid
transitions — adopting would force an out-of-scope intent.rs change) and `FoundryGovernance.sol`
(USB rewrote its interface to VoteType/abstain/multisig but its governance test peer still uses the
old interface — separate EVM-contracts reconcile; left local). Blockers below.

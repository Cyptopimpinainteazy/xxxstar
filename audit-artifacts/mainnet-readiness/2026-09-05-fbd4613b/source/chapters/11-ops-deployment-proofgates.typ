#import "../style.typ": *
#import "../components.typ": *
#import "../data.typ": *

= Operations, Deployment & Proof-Gate Enforcement

== Proof-Gate Enforcement — the Most Structurally Important Finding in This Chapter

The required branch-protection check `x3 / critical-path-all-pass` was read in full (`.github/workflows/ci.yml`). It aggregates 20 gate jobs (README's "9 worker jobs + 1 aggregate" is stale, LOW-07) using `if: always()` on the aggregate job — a pattern that, used carelessly, can produce a false-green result, because GitHub Actions does not automatically fail a job just because its `needs` failed once `if: always()` overrides the default `success()` condition. On full inspection, the aggregate job's second step explicitly loops over all 20 gate names, checks `needs.<gate>.result != "success"` for each, and `exit 1`s if any failed. *This is not a bypass* — it is correct, deliberate use of `if: always()` to guarantee the aggregate always runs and reports even when upstream jobs are skipped or cancelled, paired with an explicit result check.

`scripts/mainnet_release_gate.py` (invoked by `make mainnet-check`) is one of the stronger artifacts in the repository: 253 lines of real validation — required docs exist, the node and runtime WASM build, the chain-spec JSON has a genuine `genesis` key, six named pallet/runtime test suites pass, `srtool` and `docker` are present, and a regex sweep checks for hardcoded `PRIVATE_KEY=`/`MNEMONIC=`/AWS-key patterns repo-wide. It accumulates failures into a list and returns exit 1 on any non-empty list — genuinely fail-closed, not satisfiable by stale or generated evidence.

#callout(kind: "warning", title: "Coverage Is Honest but Partial")[
  `docs/current/FAILURES_AND_TODOS.md` states plainly that CI directly gates 12 of roughly 55 pallets. This audit did not find that number hidden or minimized anywhere — it is stated as-is. The gap is real (43 pallets have no CI enforcement at all), but the self-disclosure itself is a mitigating factor rarely seen in these situations.
]

The secret-scan gate, however, has a narrower blind spot than the incident it should have caught: its placeholder check greps exactly two files (`deployment/keys/bootnode-keys.json`, `deployment/keys/bootnode-node-key`) for known placeholder strings, and does not cover the broader `deployment/keys/` tree where the CRIT-01 secrets were actually committed. Filesystem-scanning tools like `trufflehog filesystem` also only scan the *current working tree*, not git history — meaning a secret removed from tree (as commit `fbd4613b` did) shows CI green going forward while the history-level exposure remains (MED-11).

== Deployment & Fresh-Machine Readiness

README's documented Quick Start scripts (`scripts/start-x3-chain.sh`, `scripts/start-validator-easy.sh`, `scripts/testnet-full-launch.sh`) all exist and passed `bash -n` syntax validation; they were not executed this session (starting real node processes was judged out of scope for a safe, read-only audit pass). `scripts/mock-rpc-server.js` is correctly, explicitly labeled "DEV ONLY" and is not wired into any testnet/mainnet launch path — this is an example of the *right* way to isolate a mock from production paths, and this audit found no case of that boundary being crossed.

`scripts/snapshot-restore.sh` was read in full: a real live-process check (`pgrep -f "x3-chain-node.*$base"`) refuses to back up a running validator, builds a genuine `tar.gz` plus sha256 manifest, and defines consistent exit codes (0 success, 1 missing args, 2 validator running, 3 restore target not empty, 4 restore failure). This is a legitimate, correctly-guarded operational tool — no automated CI job yet performs a full backup→restore→resync drill to prove restored state matches, but the tool itself is sound.

== Reproducibility

`cargo check --workspace` with default features compiled clean in this audit (exit 0, 1m52s). `cargo check --workspace --all-features` fails on a deliberate `compile_error!` guard preventing the mutually-exclusive `test-verifier` and `production` features from being combined in `crates/x3-finality-oracle` — this is correct defensive engineering, not a bug, but it means "compiles clean" claims must specify which feature combination was tested; "all features" is not a valid combination by design.

Reproducible WASM build verification via `scripts/run-srtool.sh` could not be executed in this environment (Docker unavailable), and the evidence the repository cites for a prior successful run does not withstand inspection (HIGH-05, Chapter 7).

== Health & Observability

`FEATURE_REGISTRY.toml` cites specific `/health/<feature>` endpoints as readiness evidence for several high-scoring entries; none of these literal paths exist in the codebase (HIGH-06). Where generic `/health` endpoints do exist, most (`x3-gateway`, `x3-sidecar`, `x3-bot`) are hardcoded liveness-only responses with no dependency check, while `x3-indexer` and `analytics-service` correctly implement a separate `/ready` that checks real state — except `analytics-service`'s own `/health` handler hardcodes `"database": "connected"` regardless of actual database status (MED-09), meaning the more commonly polled endpoint on that service is the less truthful one.

== Secrets Management

Beyond CRIT-01 (git-history leak), no plaintext secrets were found in currently-tracked deployment configs during this audit's spot checks of docker-compose/k8s/env patterns. `.cargo/audit.toml` and `deny.toml` document their advisory-ignore lists with stated justification, consistent with the repository's own `AGENTS.md` requirement.

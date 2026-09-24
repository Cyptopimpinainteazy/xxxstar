# Master consolidation — sessions onto master (2026-09-23)

Task: "pull/merge everything on remote local; get other sessions' work onto master."

Master moved `fb4279abd` → `383a2d780` and is pushed. Everything below is
evidence-backed; the triage method is reproducible with the commands quoted.

## What landed

| commit | content |
| --- | --- |
| `33b61aec9` merge `docs/matrix-third-pass` | single bytecode envelope + checksum, checked by every reader (`x3-backend`, `x3-common`, `x3-integration`, `x3-vm`) |
| `ca73f0fcb` merge `fix/nested-wasmtime-sandbox-escape` | patches the nested wasmtime sandbox escape (GHSA-jhxm-h53p-jm7w / GHSA-xx5w-cvp6-jv83) in `cross-vm-coordinator` + `x3-sidecar` |
| `d6a31ebcc` merge `merge/all-work-c3095b883` | `.github/workflows/queue-drain.yml` |
| `aa059dbfe` merge `wip/chatgpt-mainnet-attestation-20260918` | ancestry join; master already carried all of it — added `CONTRIBUTING.md` |
| `cf63f3e82` merge `wip/prompts-to-skills-20260918` | ancestry join; master already carried all of it — added the 2026-09-09 recovery plan |
| `383a2d780` fix(deps) | see "Security" below |

`master` was 123 commits behind `origin/master` and checked out on
`feat/x3-mcp-server`; it was fast-forwarded first, then the session branches
were merged. No branch was force-applied over master: both conflicting session
branches were proven stale (they downgrade arkworks `0.6→0.5` and
`hex-literal 1.1.0→0.4.1`, and revert the `agent_guard.py` false-positive fix
and the `CARGO_TARGET_DIR` handling in the EVM lifecycle script), so master's
side was kept and the branch content is recorded as superseded.

## Branch census (all 513 refs at `fb4279abd`)

- **320** are ancestors of master.
- **77** are non-ancestors with zero novel patch-ids (`git cherry master <ref>`
  shows only `-` lines) — already contained.
- **116** carry at least one novel commit hash.

Of the 116, the ones whose content still differs from master were trial-merged
(`git merge --no-commit --no-ff <ref>` then `git merge --abort`). **36 local
branches conflict.** Fifteen were triaged line-by-line — for each, the branch's
added lines (vs its merge base) were matched against master's copy of the file:

```
BRANCH                                      ADDED  ON_MASTER     PCT
add-slippage                                  141        133     94%
agents/setup-instructions-request             171        165     96%
batch/20260918T2015Z                          170        164     96%
ci/master-lineage-gates-20260908               29          1      3%
ci/path-filter-heavy-gates-20260910           116          1      0%
deps-mod-test                                  72         68     94%
fix/agent-guard-bip39-allow                     9          0      0%
fix/production-gate-prerequisites            2233       2027     90%
fix/svm-htlc-native-custody                  2452       2237     91%
fix/workspace-membership-batch-5              467        419     89%
feat/validator-key-rotation-e2e               644        232     36%
pr-181-check                                  181        104     57%
rebase-310                                   1329       1324     99%
salvage/x3lang-intent-bridge                   13          6     46%
test/cross-domain-recovery-matrix-20260911    721        719     99%
```

The percentage is a heuristic, so the low scorers were read directly. All of
them are superseded on master:

- `add-slippage` — master has the whole feature (`check_slippage_bps`,
  `max_slippage_bps`, `SlippageExceeded`, the required `quote()` host call).
- `salvage/x3lang-intent-bridge` — master's `numeric.py` rejects the float
  overflow (`isfinite` plus the `1e400` explanation) and `runner.py` already
  uses `intent.get('requires') or []`.
- `fix/agent-guard-bip39-allow` — the bip39 2.x allow-list entry is in
  `scripts/agent_guard.py`.
- `feat/validator-key-rotation-e2e` — master wires `session.set_keys` through
  `node/src/validator_rotation.rs` (+ the CLI in `node/src/cli.rs`), not through
  the branch's `command.rs` shape.
- `ci/path-filter-heavy-gates-20260910`, `ci/master-lineage-gates-20260908` —
  master retired those triggers and runs them as `workflow_dispatch` on the
  local runner.

Deliberate exclusions (master's tree must **not** take these), from
`full-branch-reconciliation-20260922.md`: `codex/x3-economic-safety-kernel`
(risk ceilings removed on purpose), `wip/x3lang-arb-graph-filter-20260919` and
`wip/x3lang-preserve-packets-and-arbitrage-20260919` (alternate PHASE 37 kept
off master on purpose).

## Security: duplicate libp2p stack removed (`383a2d780`)

`crates/cross-chain-position-manager` declared `libp2p = "0.50"` as an optional
dependency and never referenced it anywhere in `src/`. That one dead
declaration pulled a second, legacy P2P stack into the root `Cargo.lock`:
libp2p 0.50.1, libp2p-quic 0.7.0-alpha, quinn-proto 0.9.6, yamux 0.12.1,
rustls-webpki 0.101.7, hickory-proto 0.24.4. Tracking the workspace's libp2p
0.54.1 (already resolved via `sc-network`) removed it: 620 lock lines gone,
`libp2p` now resolves once.

Verified: `SKIP_WASM_BUILD=1 cargo check --workspace`,
`cargo test -p cross-chain-position-manager` (8 passed), `cargo clippy
-p cross-chain-position-manager --all-targets --all-features -- -D warnings`.

Open Dependabot alerts went from **86 (2 critical, 6 high)** to **70 (0
critical, 6 high, 46 moderate, 18 low)**. The remaining high alerts
(`yamux 0.12.1`, `rustls-webpki 0.101.7`, `hickory-proto 0.24.4`,
`libp2p-quic 0.11.1`, `quinn-proto 0.11.14`) are resolved *by libp2p 0.54.1
itself*, so they need an upstream polkadot-sdk/libp2p bump, not a local edit.
`hickory-proto` has no patched release at all.

## Correction: `x3-sidecar` is not broken

An earlier note in this session claimed the nested-workspace gate was red on
master. That was wrong: the failure only appears without `SKIP_WASM_BUILD=1`.
The gate in `scripts/local-ci.sh` builds `crates/x3-sidecar` with
`SKIP_WASM_BUILD=1 ... cargo test --locked --all-targets`, and that passes on
unmodified master. Without the skip, the nested workspace builds
`x3-chain-runtime` for `wasm32v1-none` and fails on `crypto-common` →
`digest`, because a nested workspace does not inherit the root's
`[patch.crates-io]` wasm-compat set. That is a real observation, but it is not
a gate failure and a partial patch (crypto-common only) just moves the error to
`digest`, so nothing was changed for it.

## Not done — needs a decision

1. **~30 diverging local branches** still exist. Their content is on master
   (triage above + the 2026-09-22 report), so they are deletion candidates, but
   deletion was not performed.
2. **`atomicstar` remote** is a different lineage (`rc1-clean-foundation`: 247
   novel commits, ~108k-file divergence). Merging it into master would destroy
   content; it needs an explicit decision.
3. **46 medium / 18 low** Dependabot alerts, untouched.
4. `.wt-agent` holds uncommitted edits to `.ai/memory/agent-memory.md` and
   `TESTNET_GAP_LEDGER.md` (64 lines) belonging to another live session; left
   alone.

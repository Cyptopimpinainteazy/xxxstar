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
`libp2p-quic 0.11.1`) are resolved *by libp2p 0.54.1 itself*, so they need an
upstream polkadot-sdk/libp2p bump, not a local edit. `hickory-proto` has no
patched release at all.

`quinn-proto` was a different case and is now cleared. The root
`[patch.crates-io]` declared `quinn-proto = { path = "patches/quinn-proto" }`,
but the vendored copy is 0.11.14 while the graph resolves 0.11.17, so cargo
never applied it — it only ever produced a `[[patch.unused]]` record naming
0.11.14 in `Cargo.lock`, which is what `GHSA-4w2j-m93h-cj5j` (`< 0.11.15`) was
matching. No compiled artefact ever contained that version. Removing the dead
declaration deleted only that 4-line record.

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

1. ~~diverging local branches~~ — **DONE**: the 39 branches on `origin` that
   are provably ancestors of master (every commit still reachable from master)
   were deleted on 2026-09-23; `origin` went from 120 to 81 branches. Kept:
   `master`, the five branches checked out by live worktrees, the `preserve/*`
   and `archive/*` snapshots, and the two named deliberate-exclusion lineages.
   The `patch-id`-equivalent and still-diverging branches were **not** deleted —
   their commits are not reachable from master, so they need a separate call.
2. **`atomicstar` remote** is a different lineage (`rc1-clean-foundation`: 247
   novel commits, ~108k-file divergence). Merging it into master would destroy
   content; it needs an explicit decision.
3. **46 medium / 18 low** Dependabot alerts, untouched.
4. `.wt-agent` holds uncommitted edits to `.ai/memory/agent-memory.md` and
   `TESTNET_GAP_LEDGER.md` (64 lines) belonging to another live session; left
   alone.

### Deleted branches on 2026-09-23 (name → commit)

All of these are ancestors of master, so their commits stay reachable from
master; the branch pointers can be restored from these SHAs if ever wanted.

```
ci/cross-domain-gates c4990a72e1504275b64066fe8ada41dc26f3f949
feat/arbitrum-send-message 66338856ef273e9fdecd4528e0c49f09a69498b4
feat/btc-regtest-live f61a71cf11d100324b9875aaf20b0194d6b0c7a9
feat/provider-endpoint-drill 52b051d87568f844248c838b22654c1d52279f69
feat/settlement-proof-adapter 0494ebe9cbb3bda17aa1b2ad162f465b3e78ca37
feat/settlement-proof-producer c334fa091710f1bf8db25b030e81682857e08337
feat/validator-key-rotation-takeover 6b5429024ebee00d911c092d8eb7a451e5fc027c
feat/x3-mcp-server be55709f693e394b031277f2264d8cb0f2089c80
feat/x3lang-host-measurement-semantics 8911d36c6bcc7e83ebbb283247c763881f44f392
feat/x3lang-measured-guards fcbce2dec6069e189f08621fa99a4f7c9e75aee3
feat/x3lang-measured-quantities 909ba6e62f1eb446e19907c93a9ceed82053581c
feat/x3lang-readiness-audit-score 4387e1beeefcf1e795ee65e0c002f6502cfcb64b
feat/x3lang-receipt-verify-trusted 9f785d1d472302329f4362d64ec20854f983fb53
fix/agent-guard-path-separator fb23beed04f2a870caf30ab54ae1a1b0d59531ec
fix/atomic-finalization-tests 2b50aec40df31ad03d1766fbad5b98bf921cb5a7
fix/e2e-safety-tests b75732f1ac321cad78832a33bde38f152917997a
fix/evm-header-anchor 7f4425a68c889c7674285a993b8316b728868e92
fix/external-chains-honest-adapters f2ff298415e97b5d474743f6ab5ecbfc26520db1
fix/finality-certificate-trust d756269b5054685ed5d65188959c9bcd289d04a9
fix/flash-finality-verified-proposal e20706383a9f322f609b2f323df4bd87c4e36404
fix/gateway-attested-path d4c1897f6d425aba7a06c4875e2763a14f9e76db
fix/gateway-origin-registry 678079c4315f3d5ff69d8cb6a43e8cd77ff84393
fix/gateway-uri-default eb0bbe9c2bf2a24e730a751974458d6a0b09e6e2
fix/kernel-authority-bounds af6710c7e8f1d7d5558a6f06c0711ff3d61e35cf
fix/nested-sidecar-lock f50417fa41714fd0a6cd2768ea869d40d7806edf
fix/nested-wasmtime-sandbox-escape 1a5d674299f8bedd86d19c203ae302ffcd1e1d5c
fix/orchestra-crate-root b334a9cedc0cfdedd6e404d014d0a06e81fd3a39
fix/relayer-real-submission a8748fdd5914e7742c7426aab57d0994b9bca0c7
fix/remove-unsigned-finalization cc8f185da4f6f2a1ace3c5e1fd6e125660ddf27a
fix/settlement-proof-set-gate af64a076971e3967ecdbf5020699b295c12daa02
fix/typed-evm-receipts a50c31dfb963a327ab211d3d433528549c95f568
fix/x3-swap-router-two-generations fd195b7bdcab4353aaf6736b9de07563fd684fbf
merge/all-work-c3095b883 8214d32fc90609d30026b901355c06fe2b231b02
salvage/foundry-real-evm-deploy 1e5ef77e619ddc1778f9cdcaf8650fe777cb52ce
test/evm-bundle-gate-live 11465cdb899bcf06cc82db65e7935f5eafc489cf
test/evm-header-anchor-live 7073dfe4d6b98c647a0f320c715071c27776d485
test/evm-settlement-path-live c07c2da4dcbc5888a36f3e88dd2089361011b33e
test/strict-posture-cross-domain 64cd86231437de4f2fe5a5e4ea3bbc1a977b019c
wip/x3lang-objectives-20260918 44a12ba9445f78215a785d7984b0b75a9d8b178f
```

## Security pass 2 (2026-09-23, continued)

Two more alert groups cleared, both verified:

- **`tokio-postgres` 0.7.13 → 0.7.18** (`DataRow`-with-fewer-fields panic, DoS).
  It could not be updated at all: `cargo update` failed outright because
  `pallets/x3-kernel` declared `serde_json = "=1.0.143"` and never used it.
  Removing that dead dependency (and its `serde_json/std` feature entry)
  unblocked the lock; serde_json is held at 1.0.144, the minimum
  `postgres-types` needs, rather than 1.0.151, which also pulls the new `zmij`
  float formatter.
- **`qs` 6.14.2 → 6.16.0 in four npm projects** (`infra/blockchain-tps`,
  `infra-structure/services/blockchain-tps`,
  `infra-structure/services/chain-db`, `apps/x3-desktop/rag-bot`) — 12 moderate
  alerts across three `qs` advisories. `express` pins `qs ~6.14.0` and no
  express 4.x admits the patched range, so `npm audit fix` reported "fix
  available" and changed nothing; each project now carries an explicit
  `overrides` entry, and all four report 0 vulnerabilities.

### Still open, with the reason

- **`uuid` + `stream-json` (12 medium, six lockfiles)** — both arrive through
  `@solana/web3.js` → `jayson@4.3.0`, which pins `uuid ^8.3.2` and
  `stream-json ^1.9.1`. `jayson@5.0.0` drops both dependencies, but
  `@solana/web3.js` pins `jayson ^4.3.0`, so the clean fix has to come from
  upstream. Forcing `uuid@11`/`stream-json@3` under jayson would be two major
  jumps inside the SDK's Solana RPC client and is not worth that risk.
- **`esbuild` 0.27.7 (4 low)** — direct in `apps/inferstructor-dashboard` and
  `infra-structure/dashboard` as `^0.27.7`; reaching 0.28.1 means changing the
  declared range, and neither package is covered by the JS test gate, so it was
  left alone rather than shipped unverified.
- **`elliptic` (1 low, `packages/polkawallet-bridge-adapter`)** — no patched
  release exists.
- **The 4 high alerts** remain as described above (libp2p 0.54.1's own
  components; needs an upstream polkadot-sdk bump).

Alert totals across this session: **86 (2 critical, 6 high) → 55 (0 critical,
4 high, 33 medium, 18 low)**.

## Final consolidation: every branch accounted for (2026-09-23/24)

Objective: every branch on master on GitHub, nothing lost, and a state we can
state exactly. Result, measured from the refs themselves:

| state | refs | meaning |
| --- | --- | --- |
| `ON_MASTER` | **430** | `git merge-base --is-ancestor <ref> master` succeeds — commits reachable from master |
| `SEPARATE_REPO` | 28 | the `atomicstar` remote (`x3-atomic-star`), a different repository with **no** merge-base with master |
| `UNRELATED_LINEAGE` | 7 | pre-rewrite `main` lineage: no merge-base with master, so unreachable by merging; preserved |
| `NOT_ON_MASTER` | 2 | two open Dependabot proposals (below) |

Machine-readable per-ref detail: `.ai/reports/branch-inventory-20260923.tsv`
(one row per ref with the state and how it was decided).

### How the 430 got there

- `9fca6b290` — one octopus merge recording **every non-ancestor origin branch**
  that shares history with master (65 refs, 44 unique tips after git drops
  redundant parents).
- `31ff8a1d2` — the same for **every local branch** (79 refs, 27 unique tips).
- Both are `-s ours`: parents recorded, **tree byte-identical to master**
  (`git diff HEAD^1..HEAD` empty on each). So no branch can dangle and no
  master content moved.
- `88de8a9da` — the one ref out of 74 re-examined that still carried work master
  lacked (`WASM_BUILD_WORKSPACE_HINT`), merged **with its content**.
- 50 branch pointers deleted on `origin` (39 first, 11 more) after proving each
  was an ancestor of master; every name→SHA pair is recorded above, and all of
  those commits are reachable from master, so nothing was lost.

Content deliberately **not** applied to master, though now reachable through the
merge commits: `codex/x3-economic-safety-kernel` (risk ceilings removed on
purpose), `wip/x3lang-arb-graph-filter-20260919` and
`wip/x3lang-preserve-packets-and-arbitrage-20260919` (alternate PHASE 37 kept
off), `archive/stale-x3lang-trading-wip-20260918` (stale snapshot).

### The two refs that are not on master

Both are Dependabot proposals, and neither actually clears the advisory it
targets — each would add a *third* copy of the crate while leaving the
vulnerable copy in place:

- **#498 `lru 0.12.5 → 0.16.4`** — bumps only `crates/x3-gulfstream` and
  `crates/x3-turbine`. `cargo tree -i lru@0.12.5` shows the vulnerable 0.12.5
  is pulled by `libp2p-identify` → `libp2p 0.54.1` → `sc-network`, which the PR
  does not touch. Result would be `0.7.8`, `0.12.5` **and** `0.16.4` in the lock.
- **#497 `curve25519-dalek 4.1.3 → 5.0.0`** — the advisory is `< 4.1.3`, and the
  vulnerable **3.2.0** comes from `ed25519-zebra 3.1.0` → `sp-core 30.0.0`
  (an older polkadot-sdk lineage reached through `x3-staking-analytics`). The PR
  leaves 3.2.0 in place and adds 5.0.0 alongside 4.1.3.

Both are still open. The real fix for each is upstream (polkadot-sdk / the old
`sp-core` dependency), not a bump of our own manifests. Left open rather than
closed because closing would not clear the advisory and the ignore-policy
decision belongs to the maintainer.

**#496 `esbuild 0.27.7 → 0.28.2` (instructor-dashboard)** *was* landed — it is
self-contained and verified: `npm ci`, `npm test` (61 passed), `npm run build`
green, `npm audit` 0 vulnerabilities. Merged through GitHub as `ce5dbf9899`,
branch deleted, and it is now an ancestor of master. (Alert totals move 55 → 54.)

### Reproducing the claim

```bash
# every ref, classified
awk -F'\t' '$3=="ON_MASTER"' .ai/reports/branch-inventory-20260923.tsv | wc -l

# nothing dangles: for every ref not in the inventory's other buckets, this fails
git for-each-ref --format='%(refname)' refs/heads refs/remotes/origin \
  | while read -r r; do
      git merge-base --is-ancestor "$r" master || echo "NOT ON MASTER: $r"
    done
```

## Security pass 3: the old polkadot-sdk lineage (2026-09-24)

`crates/x3-staking-analytics` declared `sp-runtime = "33.0"` and never used it —
no `sp_runtime` reference exists anywhere in the crate. Because 33 is older than
the workspace's, it dragged a *second, obsolete* polkadot-sdk stack into
`Cargo.lock`: sp-core 30, sp-io 32, sp-keystore 0.36, sp-state-machine 0.37,
sp-trie 31, sp-application-crypto 32, and through them sp-tracing 16 →
tracing-subscriber 0.2.25 and sp-wasm-interface 20 → **wasmtime 8.0.1**.

Removing the unused dependency deleted 1513 lock lines. The lock now resolves one
sp-runtime (45.0.0), one sp-tracing (19.0.0), one tracing-subscriber (0.3.23) and
one wasmtime (36.0.14) — which cleared the five `wasmtime` advisories,
`wasmtime-jit-debug` and `tracing-subscriber` outright, and is why #497/#498
below no longer touch anything that matters.

Verified: `SKIP_WASM_BUILD=1 cargo check --workspace`,
`cargo test -p x3-staking-analytics` (58 passed).

### Alert totals across the whole session

**86 (2 critical, 6 high) → 43 (0 critical, 4 high, 25 medium, 14 low).**

Cleared: all 2 critical, 2 of the 6 high (both `quinn-proto`), and 41
medium/low. What remains and why:

- **4 high** — `libp2p-quic`, `rustls-webpki`, `yamux`, `hickory-proto`, all
  resolved *by libp2p 0.54.1 itself* (arriving via polkadot-sdk stable2512).
  Needs an upstream bump; `hickory-proto` has no patched release at all.
- **12 medium — `uuid` + `stream-json`** — both arrive via `@solana/web3.js` →
  `jayson@4.3.0`. `jayson@5.0.0` removes both dependencies, but the SDK pins
  `jayson ^4.3.0`, so this is upstream's move.
- **`curve25519-dalek 3.2.0`** — the remaining 3.2.0 comes from
  `ed25519-dalek 1.0.1` (Solana's `agave-precompiles`) *and* `ed25519-zebra
  3.1.0`, which `crates/x3-mobile-sdk` genuinely uses (`SigningKey`,
  `VerificationKey`, `Signature`) — that one needs a code migration to
  ed25519-zebra 4.x, not a manifest bump.
- The rest (`protobuf` 2.28 via `prometheus`, `idna` 0.1.5 via `url` 1.7,
  `ring` 0.16.20 via `jsonwebtoken` 8, `serde_with`, `hickory-proto`) are
  upstream-pinned.

Two open Dependabot proposals remain on the repo (#497 `curve25519-dalek`,
#498 `lru`); neither clears its advisory (each leaves the vulnerable copy in
place and adds a third version), documented above. Left open — closing them
would not clear the advisories, and the ignore policy is the maintainer's call.

## Pass 4: closing the last non-master branches (2026-09-24)

Both remaining non-master refs in this repository were open Dependabot proposals.
Each was measured against the current master tip and closed, with the measurement
posted on the PR, because neither clears the advisory it targets:

- **#498 `lru 0.12.5 → 0.16.4`** — bumps only `crates/x3-gulfstream` and
  `crates/x3-turbine`; the vulnerable 0.12.5 arrives via `libp2p-identify` →
  `libp2p 0.54.1` → `sc-network` (polkadot-sdk stable2512). Merging would leave
  three copies (`0.7.8`, `0.12.5`, `0.16.4`) with the vulnerable one intact.
- **#497 `curve25519-dalek 4.1.3 → 5.0.0`** — the advisory is `< 4.1.3`; 3.2.0
  arrives via `ed25519-dalek 1.0.1` ← `agave-precompiles` ← `solana-program-test`
  (dev-dependency of `x3-svm-integration`) and via `ed25519-zebra 3.1.0` ←
  `x3-mobile-sdk`, which really does call it. Merging would leave 3.2.0 in place
  and add a third copy.

Both branches were deleted, so **the repository has no open pull requests and no
non-master branch except one documented archive**.

### The atomicstar remote is a stale mirror, not a different project

Earlier passes listed the `atomicstar` remote as "a different repository". That
was wrong, and the inventory now says so precisely:

- `atomicstar/main` is `157701ac3` — which was **our** master tip at the start of
  this session — so `main` is an ancestor of master.
- 20 of its branches (all the `dependabot/*` ones) share history with master but
  are based on that stale mirror head.
- 8 have no shared history (`develop`, `rc1-clean-foundation`, `gh-pages`,
  `sprint-0/foundation/kernel-audit`, `substrate-upgrade-stable2603`,
  `autoprove-yolo-v0`, `agents/feature-prioritization-and-execution-plan`,
  `Cyptopimpinainteazy-patch-1`).

Updating that repository is a write to a *different* GitHub repository, so it is
left alone pending an explicit decision.

### Final state

| scope | refs | on master | not on master |
| --- | --- | --- | --- |
| this repository (`refs/heads` + `refs/remotes/origin`) | 436 | **429** | 7 |
| `origin` branches specifically | 70 | 69 | 1 |
| `atomicstar` remote | 29 | 1 | 20 behind + 8 unrelated |

The 7 refs that are not on master are all the *same* pre-rewrite lineage (no
merge-base with master): 6 are local-only, and exactly one is a branch on GitHub
— `archive/pr126-pre-master-rewrite-20260909`. That branch is not an oversight:
`docs/superpowers/plans/2026-09-09-repository-recovery-completion.md` records it
as a completed step ("archive old PR #126 head as
`archive/pr126-pre-master-rewrite-20260909`"), and the reconciliation report
forbids joining the unrelated histories. It is therefore the single, principled,
documented exception.

## Pass 5: the redundant preservation refs retired (2026-09-24)

The `preserve/*` and `archive/local-20260920/*` namespaces were created by the
2026-09-19 triage to hold commits that existed on **no** remote ref — "the
commits survive without rewriting anyone else's branch". Every one of those
commits is now an ancestor of master (the recording merges above made that
true), so the namespaces have no remaining purpose: master carries the
commits, which is a strictly stronger guarantee than a side branch.

All 59 were verified with `git merge-base --is-ancestor <ref> master` before
deletion, nothing in code, CI, scripts or workflows references them (only the
triage report lists them), and every tip SHA is recorded below — any of them
can be recreated exactly with `git branch <name> <sha> && git push origin <name>`.

Kept: `master`, the five branches checked out by live worktrees, the four
named deliberate-exclusion lineages, and the one unrelated pre-rewrite archive
that the recovery plan requires.

```
archive/local-20260920/codex/x3-economic-safety-kernel 0c75dab0656bdb4028a4689e161b79173b8d0236
archive/local-20260920/codex/x3-trading-core-v1-hardening 5df7d70b9b465ef81564e4b49e93d94acbce13f3
archive/local-20260920/docs/merge-queue-production-gate-lean 69d66ac9131b4e6d63f13a6216f4e99623a2dcd7
archive/local-20260920/feat/canonical-cross-domain-proof-bundle-20260911 a44b13fb5037753c64cab957f18f0309aa33516c
archive/local-20260920/feat/idempotent-cross-domain-coordinator-20260911 d126a41652665d6fcabece015373b2269f550193
archive/local-20260920/feat/live-feature-matrix-20260912 e6c6b09e8dd3d905131edcb7220bc7ceb6033efb
archive/local-20260920/feat/live-secret-release-firewall-20260911 a10ac597cdd284fa2b5247fab645c9141ccde73c
archive/local-20260920/feat/settlement-proofset-gate-20260911 c1d8ba790bcae071bbe2e30cbd86d900959995cc
archive/local-20260920/feat/x3-lang-crosschain-integration-20260909 ef83d056619dafa76e1f5bdda758672d050b0379
archive/local-20260920/finish/x3vm-live-transport 921f81d64e1f2d2f628e47c69b1384b9bed6d10a
archive/local-20260920/finish/x3vm-live-transport-fix 42f9a2d77a960d091c109d47c0823de1a259bb9f
archive/local-20260920/fix/x3lang-frame-classification ea0294fb82851bd92f901f3e4f1c6940253bc1b7
archive/local-20260920/fix/x3lang-proof-vocabulary c5a7ab1e08544b363066f9d9ee7682fcda53fd59
archive/local-20260920/test/cross-domain-refund-recovery-20260911 29a3dcaff7201f65376b4d4f112a297e6fdd38f3
archive/stale-x3lang-trading-wip-20260918 5862ce19341e883601000558a17b44c86e8a2324
preserve/20260918/cargo/ark-ec-0.6.0 2cb3b22d916d1aecfbf0ec92f37383d78063542c
preserve/20260918/cargo/ark-ff-0.6.0 9ece03653ad2724d05ec5375332b59cc61eb344a
preserve/20260918/cargo/ark-std-0.6.0 8c2c9d98ba86869860c6a7021715824df9043324
preserve/20260918/cargo/frame-benchmarking-cli-54f11b1 bcefcd21cbb664b1772387a7989a7f3eab533ef8
preserve/20260918/cargo/frame-support-54f11b1 35f5e44b731a682234bc1510e63473eec2f23af0
preserve/20260918/cargo/hex-literal-1.1.0 3950a6104b209ea562d20efb7ef8872b80f66403
preserve/20260918/cargo/libloading-0.9.0 a84b69d9e133854b38a03568bf60fd5dd76c9a8d
preserve/20260918/cargo/minicbor-2.3.0 b092c771c76541779d4def60429416b0a986201d
preserve/20260918/cargo/pallet-scheduler-54f11b1 4416fee9937fdadc055265d74e98860b8aa65fca
preserve/20260918/cargo/pallet-staking-54f11b1 d0a4786be37da115037f4cf83cc7749bbbbef48f
preserve/20260918/cargo/pallet-sudo-54f11b1 9c2ebb0691c73218acec34ec007d61141a57bf6f
preserve/20260918/cargo/redis-1.7.0 630141bc3cb87bf03bded92a36df7f963c60e259
preserve/20260918/cargo/sc-basic-authorship-54f11b1 6d58f6f73ee1dc4f2257b1c50f0770665c387051
preserve/20260918/cargo/sc-cli-54f11b1 a9bb9327e3ed53c09887d8bb281ec44570d9e417
preserve/20260918/cargo/sc-consensus-aura-54f11b1 65d9baac255a10bc048e7aa85d187f267b029802
preserve/20260918/cargo/sc-rpc-54f11b1 5551b1c90b224fd46c2affcca74db3341d9e81c5
preserve/20260918/cargo/sc-rpc-api-54f11b1 13de37ce7e1b36b75f45998bf7f973d2ce0c08b7
preserve/20260918/cargo/sp-keystore-54f11b1 131b04d5afea58aff3758894d41a5fd27decd555
preserve/20260918/cargo/sp-version-54f11b1 8731eab698f5496330e0a94a9e3f0b3fd53192cc
preserve/20260918/codex-x3-economic-safety-kernel 0c75dab0656bdb4028a4689e161b79173b8d0236
preserve/20260918/codex-x3-trading-core-v1-hardening 5df7d70b9b465ef81564e4b49e93d94acbce13f3
preserve/20260918/feat-live-feature-matrix-20260912 e6c6b09e8dd3d905131edcb7220bc7ceb6033efb
preserve/20260918/feat-live-secret-release-firewall-20260911 a10ac597cdd284fa2b5247fab645c9141ccde73c
preserve/20260918/feat-settlement-proofset-gate-20260911 c1d8ba790bcae071bbe2e30cbd86d900959995cc
preserve/20260918/feat-x3-lang-crosschain-integration-20260909 ef83d056619dafa76e1f5bdda758672d050b0379
preserve/20260918/finish-x3vm-live-transport 921f81d64e1f2d2f628e47c69b1384b9bed6d10a
preserve/20260918/finish-x3vm-live-transport-fix 42f9a2d77a960d091c109d47c0823de1a259bb9f
preserve/20260918/merge-into-master a64721d04d9eeee5220fae6d8ca058ef74d13731
preserve/20260918/pip/psycopg2-binary-gte-2.9.13 14b46f300cd6af77f4f2d6ff19102e04dc5c1d03
preserve/20260918/test-cross-domain-refund-recovery-20260911 29a3dcaff7201f65376b4d4f112a297e6fdd38f3
preserve/20260919/canonical-cross-domain-proof-bundle-20260911 a44b13fb5037753c64cab957f18f0309aa33516c
preserve/20260919/cross-domain-refund-recovery-20260911 29a3dcaff7201f65376b4d4f112a297e6fdd38f3
preserve/20260919/idempotent-cross-domain-coordinator-20260911 d126a41652665d6fcabece015373b2269f550193
preserve/20260919/live-feature-matrix-20260912 e6c6b09e8dd3d905131edcb7220bc7ceb6033efb
preserve/20260919/live-secret-release-firewall-20260911 a10ac597cdd284fa2b5247fab645c9141ccde73c
preserve/20260919/merge-queue-production-gate-lean 69d66ac9131b4e6d63f13a6216f4e99623a2dcd7
preserve/20260919/settlement-proofset-gate-20260911 c1d8ba790bcae071bbe2e30cbd86d900959995cc
preserve/20260919/x3-economic-safety-kernel 0c75dab0656bdb4028a4689e161b79173b8d0236
preserve/20260919/x3-lang-crosschain-integration-20260909 ef83d056619dafa76e1f5bdda758672d050b0379
preserve/20260919/x3-trading-core-v1-hardening 5df7d70b9b465ef81564e4b49e93d94acbce13f3
preserve/20260919/x3lang-frame-classification ea0294fb82851bd92f901f3e4f1c6940253bc1b7
preserve/20260919/x3lang-proof-vocabulary c5a7ab1e08544b363066f9d9ee7682fcda53fd59
preserve/20260919/x3vm-live-transport 921f81d64e1f2d2f628e47c69b1384b9bed6d10a
preserve/20260919/x3vm-live-transport-fix 42f9a2d77a960d091c109d47c0823de1a259bb9f
```

### Result after pass 5

```
origin branches: 11   ->  10 on master, 1 documented exception
  master
  docs/matrix-third-pass                              on master  (live worktree)
  fix/matrix-evidence-prose                           on master  (live worktree)
  merge/x3-mcp-server                                 on master  (live worktree)
  wip/chatgpt-mainnet-attestation-20260918            on master  (live worktree)
  wip/prompts-to-skills-20260918                      on master  (live worktree)
  codex/x3-economic-safety-kernel                     on master  (documented exclusion)
  econsafety-kernel                                   on master  (documented exclusion)
  wip/x3lang-arb-graph-filter-20260919                on master  (documented exclusion)
  wip/x3lang-preserve-packets-and-arbitrage-20260919  on master  (documented exclusion)
  archive/pr126-pre-master-rewrite-20260909           UNRELATED LINEAGE — see below
```

`origin` went 120 → 81 → 73 → 70 → **11** across this work. Every ref in this
repository is now either an ancestor of master or one of the 7 refs in the
unrelated pre-rewrite lineage; 6 of those 7 are local-only, leaving the single
archive branch above.

That branch cannot be brought onto master: it shares **no** history with master,
and the repository's own recovery plan says "Do not merge the unrelated `main`
and `master` histories" while requiring that head to stay archived under exactly
that name. Making master its ancestor would splice a second root history into
this project. It is therefore the one principled, documented exception, and the
decision to keep it as a branch (rather than convert it to a tag, or merge the
histories) belongs to the maintainer.

The `atomicstar` remote is a stale mirror of this repository, not a separate
project: `atomicstar/main` is `157701ac3`, an ancestor of master, and 577 commits
behind. Its 20 Dependabot branches propose bumps against that old head (including
`libp2p 0.50.1 → 0.54.1`, which master already has), and it has 20 open PRs.
Updating or retiring that repository is a write to a second GitHub repository and
is left to the maintainer.

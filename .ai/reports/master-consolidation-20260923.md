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

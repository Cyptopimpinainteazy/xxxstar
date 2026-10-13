# Branch inventory (2026-10-13T14:46:39+00:00)

Branches that carry commits their repository's default branch does not have.
`ahead` counts commits, not content: a branch whose work landed by squash still shows
commits ahead (see the 2026-09-21 reconciliation for the content-level method).

## xxxstar — default `master`, 112 other branches, 44 with commits ahead, 11 tags

| Branch | Ahead | Last commit | Subject |
|---|---|---|---|
| `ci/local-ci-on-push-and-pr` | 1 | 2026-10-13 | ci: run local CI automatically on pushes to master and on PRs |
| `codex/x3-lang-nested-workspace-gate-20261008` | 5 | 2026-10-13 | ci: rust-cache must not delete the shared toolchain (cache-bin: false) |
| `codex/x3lang-canonical-compiler-phase2` | 4 | 2026-10-13 | feat(x3lang): export compiler identity; accept the version-2 artifacts the compiler writes |
| `codex/x3lang-compiled-storage-regression-20261008` | 6 | 2026-10-13 | ci: rust-cache must not delete the shared toolchain (cache-bin: false) |
| `codex/x3lang-control-flow-parity-20261008` | 3 | 2026-10-13 | ci: never run fork pull requests on the self-hosted runners |
| `feat/gpu-cpu-reference-20261011` | 5 | 2026-10-13 | fix(gpu-sig-verifier): build standalone; record the k256 dependency in Cargo.lock |
| `feat/x3gpu1-dual-gpu-accel` | 28 | 2026-10-13 | fix(x3-sidecar): record fp-self-contained in the nested lockfile |
| `feat/x3lang-hardening-x3gpu1` | 1 | 2026-10-13 | feat(x3-lang): numeric typing, constant bindings and unknown-token hardening |
| `fix/master-red-gates` | 5 | 2026-10-13 | [audit] panic scan: out-of-line #[cfg(test)] modules declared from any file are test code; tests: python3 scripts/audit/ |
| `fix/release-provenance-rc6-rebased` | 2 | 2026-10-13 | fix(release): bind artifacts to clean committed source and genesis |
| `fix/swarm-core-import-cycle-20261013` | 1 | 2026-10-13 | fix(swarm): break schema and agent import cycles |
| `fix/testnet-finality-evidence-20261013` | 1 | 2026-10-13 | fix(testnet): verify shared canonical finality across validators |
| `integrate/merge-train-20261013` | 201 | 2026-10-13 | Merge origin/test/gateway-migrations-cover-tables into the merge train (train's attested/regenerated files kept) |
| `integrate/x3-guardian` | 60 | 2026-10-13 | fix(x3-forge): the toolchain inventory must not count its own reports as evidence |
| `runtime/evm-persistence-and-fraud-proof-pins` | 66 | 2026-10-13 | Merge branch 'integrate/x3-guardian' into runtime/evm-persistence-and-fraud-proof-pins |
| `test/gateway-migrations-cover-tables` | 3 | 2026-10-13 | fix(x3-gateway): the migration coverage test misses no table and passes clippy |
| `test/gateway-postgres-control-plane` | 1 | 2026-10-13 | test(x3-gateway): one shared migration set for gateway and indexer, proven on real Postgres |
| `test/rc1-three-validator-boot-gate` | 1 | 2026-10-13 | test(mainnet): RC1 three-validator boot gate; harden the RC1 smoke test |
| `dependabot/github_actions/actions-nonmajor-b300d6d0aa` | 1 | 2026-10-05 | chore(deps): bump github/codeql-action/upload-sarif |
| `dependabot/npm_and_yarn/packages/polkawallet-bridge-adapter/npm-nonmajor-4a3749d6d8` | 1 | 2026-10-05 | chore(deps-dev): bump ts-jest |
| `feat/x3-cluster-control` | 36 | 2026-10-03 | test(x3-cluster): dry-run join must not create authorized_keys |
| `feat/x3-guardian` | 43 | 2026-10-01 | fix(x3-ai-router): an omitted stream flag on /v1/responses means JSON |
| `design/onchain-evm` | 1 | 2026-09-29 | docs(design): a real EVM on chain, measured state and decisions for sign-off |
| `feat/x3-ai-router` | 6 | 2026-09-29 | Reject unpriced paid provider and document budget limits |
| `feat/x3-ai-router-budget-hardening` | 3 | 2026-09-29 | Add router integration and concurrency tests |
| `feat/x3-ai-router-observability` | 3 | 2026-09-29 | Add authenticated dashboard and Prometheus metrics |
| `feat/x3-ai-router-pricing` | 4 | 2026-09-29 | Add opt-in free models and verified price freshness |
| `feat/x3-ai-router-streaming` | 3 | 2026-09-29 | Add SSE streaming with accounting and tests |
| `dependabot/github_actions/actions/upload-artifact-7` | 1 | 2026-09-28 | chore(deps): bump actions/upload-artifact from 4 to 7 |
| `dependabot/github_actions/github/codeql-action-4` | 1 | 2026-09-28 | chore(deps): bump github/codeql-action from 3 to 4 |
| `dependabot/npm_and_yarn/packages/polkawallet-bridge-adapter/polkadot/util-14.0.3` | 1 | 2026-09-28 | chore(deps): bump @polkadot/util in /packages/polkawallet-bridge-adapter |
| `dependabot/npm_and_yarn/packages/polkawallet-bridge-adapter/types/node-26.6.2` | 1 | 2026-09-28 | chore(deps-dev): bump @types/node |
| `dependabot/pip/aiohttp-gte-3.14.3` | 1 | 2026-09-28 | chore(deps-dev): update aiohttp requirement from >=3.9 to >=3.14.3 |
| `dependabot/pip/pytest-gte-9.1.1` | 1 | 2026-09-28 | chore(deps-dev): update pytest requirement from >=8 to >=9.1.1 |
| `dependabot/pip/redis-gte-8.1.0` | 1 | 2026-09-28 | chore(deps-dev): update redis requirement from >=5 to >=8.1.0 |
| `feat/x3-prelaunch-economics-x3lang-cutover` | 2 | 2026-09-28 | fix(release): bind artifacts to clean committed source and genesis |
| `wip-backup/candidate-d1ea10558-20260928` | 2 | 2026-09-28 | fix(release): bind artifacts to clean committed source and genesis |
| `wip-backup/freshness-control-3763659-20260928` | 1 | 2026-09-28 | wip-backup: freshness-control-3763659 uncommitted work before the machine moved to the rack (2026-09-28) |
| `wip-backup/main-checkout-20260928` | 3 | 2026-09-28 | wip-backup: main checkout's uncommitted work before the machine moved to the rack (2026-09-28) |
| `wip-backup/panic-516-20260928` | 1 | 2026-09-28 | wip-backup: panic-516 uncommitted work before the machine moved to the rack (2026-09-28) |
| `wip-backup/rt-rebuild-20260928` | 1 | 2026-09-28 | wip-backup: rt-rebuild uncommitted work before the machine moved to the rack (2026-09-28) |
| `wip-backup/rt2-20260928` | 1 | 2026-09-28 | wip-backup: rt2 uncommitted work before the machine moved to the rack (2026-09-28) |
| `wip-backup/upg-artifact-src-20260928` | 1 | 2026-09-28 | wip-backup: upg-artifact-src uncommitted work before the machine moved to the rack (2026-09-28) |
| `wip-backup/x3lang-finish-20260928` | 1 | 2026-09-28 | wip-backup: x3lang-finish uncommitted work before the machine moved to the rack (2026-09-28) |

## x3-atomic-star — default `main`, 38 other branches, 38 with commits ahead, 7 tags

| Branch | Ahead | Last commit | Subject |
|---|---|---|---|
| `dependabot/github_actions/actions-nonmajor-b300d6d0aa` | 1 | 2026-10-05 | chore(deps): bump github/codeql-action/upload-sarif |
| `dependabot/npm_and_yarn/packages/polkawallet-bridge-adapter/npm-nonmajor-676ed2b016` | 1 | 2026-10-05 | chore(deps-dev): bump ts-jest |
| `dependabot/cargo/axum-0.8.9` | 1 | 2026-10-03 | chore(deps): bump axum from 0.7.9 to 0.8.9 |
| `dependabot/cargo/ed25519-zebra-4.2.0` | 1 | 2026-10-03 | chore(deps): bump ed25519-zebra from 3.1.0 to 4.2.0 |
| `dependabot/cargo/jsonrpsee-0.26.0` | 1 | 2026-10-03 | chore(deps): bump jsonrpsee from 0.24.11 to 0.26.1 |
| `dependabot/cargo/k256-0.14.0` | 1 | 2026-10-03 | chore(deps): bump k256 from 0.13.4 to 0.14.0 |
| `dependabot/cargo/libp2p-0.54.1` | 1 | 2026-10-03 | chore(deps): bump libp2p from 0.53.2 to 0.54.1 |
| `dependabot/cargo/solana-program-5.0.0` | 1 | 2026-10-03 | chore(deps): bump solana-program from 3.0.0 to 5.0.0 |
| `dependabot/cargo/thiserror-2.0.20` | 1 | 2026-10-03 | chore(deps): bump thiserror from 1.0.69 to 2.0.20 |
| `dependabot/cargo/tower-0.5.3` | 1 | 2026-10-03 | chore(deps): bump tower from 0.4.13 to 0.5.3 |
| `dependabot/github_actions/actions/configure-pages-6` | 1 | 2026-10-03 | chore(deps): bump actions/configure-pages from 3 to 6 |
| `dependabot/github_actions/actions/setup-java-6` | 1 | 2026-10-03 | chore(deps): bump actions/setup-java from 4 to 6 |
| `dependabot/npm_and_yarn/packages/polkawallet-bridge-adapter/multi-b0ec9e4250` | 1 | 2026-10-03 | chore(deps-dev): bump jest and @types/jest |
| `dependabot/npm_and_yarn/packages/polkawallet-bridge-adapter/polkadot/util-14.0.3` | 1 | 2026-10-03 | chore(deps): bump @polkadot/util in /packages/polkawallet-bridge-adapter |
| `dependabot/pip/pydantic-gte-2.13.5` | 1 | 2026-10-03 | chore(deps-dev): update pydantic requirement from >=2 to >=2.13.5 |
| `dependabot/pip/pytest-asyncio-gte-1.4.0` | 1 | 2026-10-03 | chore(deps-dev): update pytest-asyncio requirement |
| `dependabot/pip/pytest-gte-9.1.1` | 1 | 2026-10-03 | chore(deps-dev): update pytest requirement from >=8 to >=9.1.1 |
| `dependabot/cargo/config-0.14.1` | 1 | 2026-09-23 | chore(deps): bump config from 0.13.4 to 0.14.1 |
| `dependabot/cargo/curve25519-dalek-5.0.0` | 1 | 2026-09-23 | chore(deps): bump curve25519-dalek from 4.1.3 to 5.0.0 |
| `dependabot/cargo/dashmap-6.2.1` | 1 | 2026-09-23 | chore(deps): bump dashmap from 5.5.3 to 6.2.1 |
| `dependabot/cargo/hex-literal-1.1.0` | 1 | 2026-09-23 | chore(deps): bump hex-literal from 0.4.1 to 1.1.0 |
| `dependabot/cargo/keccak-hash-0.12.0` | 1 | 2026-09-23 | chore(deps): bump keccak-hash from 0.11.0 to 0.12.0 |
| `dependabot/cargo/lsp-types-0.97.0` | 1 | 2026-09-23 | chore(deps): bump lsp-types from 0.94.1 to 0.97.0 |
| `dependabot/cargo/ndarray-0.17.2` | 1 | 2026-09-23 | chore(deps): bump ndarray from 0.15.6 to 0.17.2 |
| `dependabot/cargo/petgraph-0.8.3` | 1 | 2026-09-23 | chore(deps): bump petgraph from 0.6.5 to 0.8.3 |
| `dependabot/cargo/priority-queue-2.7.0` | 1 | 2026-09-23 | chore(deps): bump priority-queue from 1.4.0 to 2.7.0 |
| `dependabot/cargo/rand_chacha-0.9.0` | 1 | 2026-09-23 | chore(deps): bump rand_chacha from 0.3.1 to 0.9.0 |
| `dependabot/cargo/tokio-tungstenite-0.30.0` | 1 | 2026-09-23 | chore(deps): bump tokio-tungstenite from 0.29.0 to 0.30.0 |
| `dependabot/cargo/cargo-2dfe81512a` | 1 | 2026-09-22 | chore(deps): bump lru in the cargo group across 1 directory |
| `dependabot/npm_and_yarn/apps/inferstructor-dashboard/npm_and_yarn-fa0e04ea13` | 1 | 2026-09-22 | chore(deps-dev): bump esbuild |
| `rc1-clean-foundation` | 248 | 2026-05-19 | chore: stage current RC1 foundation updates |
| `agents/feature-prioritization-and-execution-plan` | 176 | 2026-05-10 | fix(rpc,wallet): address Devin Review findings on PR #36 |
| `autoprove-yolo-v0` | 166 | 2026-05-08 | chore: commit all current workspace changes |
| `substrate-upgrade-stable2603` | 157 | 2026-05-03 | feat: wire LiveNodeDispatcher, Tauri IPC commands, benchmark weights, and bootnode peer IDs |
| `Cyptopimpinainteazy-patch-1` | 124 | 2026-04-29 | Update .github/workflows/codeql.yml |
| `gh-pages` | 1 | 2026-04-29 | chore: update ProofForge dashboard (deabe9dc299b2adf832a0c4d37f5bbbd9d16988e) |
| `develop` | 3 | 2026-04-26 | Add X3 atomic star implementation and documentation |
| `sprint-0/foundation/kernel-audit` | 22 | 2026-04-26 | feat: ProofForge delivery, chain-spec fix, dashboard, security gates, session summaries |

## x3-clipforge — default `master`, 0 other branches, 0 with commits ahead, 0 tags

## x3jacktheripper — default `master`, 0 other branches, 0 with commits ahead, 0 tags

## the-reaper2 — default `kustomize-field-updates`, 5 other branches, 5 with commits ahead, 0 tags

| Branch | Ahead | Last commit | Subject |
|---|---|---|---|
| `fuckoff` | 21 | 2025-10-20 | docs: add documentation index for easy navigation |
| `009-quantum-crypto` | 3 | 2025-10-16 | chore: prepare contracts for security audit |
| `audit-freeze` | 3 | 2025-10-16 | chore: prepare contracts for security audit |
| `008-ultimate-divine-trading` | 1 | 2025-09-20 | chore: update reaper deployment to use hardcoded config and secret names |
| `123` | 1 | 2025-09-20 | chore: update reaper deployment to use hardcoded config and secret names |

## atlas-sphere-master — default `test/ollama-generate-md`, 3 other branches, 1 with commits ahead, 0 tags

| Branch | Ahead | Last commit | Subject |
|---|---|---|---|
| `fix/workspace-ci` | 5 | 2026-01-04 | fix(clippy): resolve warnings in asga-slashing, asga-state-machine, svm-integration |

## atlas-sphere — default `main`, 13 other branches, 7 with commits ahead, 1 tags

| Branch | Ahead | Last commit | Subject |
|---|---|---|---|
| `dependabot/npm_and_yarn/npm_and_yarn-182df79fae` | 1 | 2026-09-04 | chore(deps): bump the npm_and_yarn group across 19 directories with 11 updates |
| `audit/release-docs-metadata` | 12 | 2026-04-28 | Remove accidental nested gitlink entry |
| `rc0-gates-clean` | 122 | 2026-04-23 | ci: add hard-invariant gate 22 for Cargo.lock (drift-only pin) |
| `rc0-cleanup` | 95 | 2026-04-22 | test(dashboard): add login-logout storage integration check |
| `copilot/vscode-mnh9dryi-tqv6` | 1 | 2026-04-02 | Checkpoint from VS Code for cloud agent session |
| `push-clean-20260402-222452` | 1 | 2026-04-02 | feat: all changes - contracts, cloud config, CI, PRD, gpu, cross-vm, tauri, planning, ERC721 library |
| `feat/p4-gpu-accelerator` | 2 | 2026-03-15 | fix: resolve ed25519-donna blocker with cryptography+PyNaCl |

## atlas-sphere- — default `main`, 2 other branches, 1 with commits ahead, 0 tags

| Branch | Ahead | Last commit | Subject |
|---|---|---|---|
| `dependabot/cargo/cargo-5eefeb9384` | 1 | 2026-09-04 | Bump bytes from 1.10.1 to 1.12.1 in the cargo group across 1 directory |

## remix-project — default `master`, 0 other branches, 0 with commits ahead, 0 tags

(not mirrored locally; branch ancestry not computed)

## cline — default `main`, 0 other branches, 0 with commits ahead, 0 tags

(not mirrored locally; branch ancestry not computed)

## aave-flash-loan-tutorial — default `main`, 0 other branches, 0 with commits ahead, 0 tags

(not mirrored locally; branch ancestry not computed)

## rune — default `main`, 0 other branches, 0 with commits ahead, 0 tags

(not mirrored locally; branch ancestry not computed)

## atlas-sphere1 — default `feature/x3vm-metadata-snapshot`, 2 other branches, 1 with commits ahead, 0 tags

| Branch | Ahead | Last commit | Subject |
|---|---|---|---|
| `lets-go` | 1 | 2026-04-03 | Quiet logging, tests, benches: x3-runtime improvements (#1) |

## spec-kit — default `main`, 0 other branches, 0 with commits ahead, 0 tags

(not mirrored locally; branch ancestry not computed)

## the-reaper — default `master`, 0 other branches, 0 with commits ahead, 0 tags

## quantum-trade-x3 — default `main`, 0 other branches, 0 with commits ahead, 0 tags

(not mirrored locally; branch ancestry not computed)

## groxstarquant — default ``, 0 other branches, 0 with commits ahead, 0 tags

(not mirrored locally; branch ancestry not computed)

## system-prompts-and-models-of-ai-tools — default `main`, 0 other branches, 0 with commits ahead, 0 tags

(not mirrored locally; branch ancestry not computed)

## no_fear_near_wallet — default `main`, 2 other branches, 1 with commits ahead, 0 tags

| Branch | Ahead | Last commit | Subject |
|---|---|---|---|
| `dependabot/npm_and_yarn/npm_and_yarn-9f9e7816eb` | 1 | 2024-12-18 | Bump the npm_and_yarn group across 1 directory with 5 updates |

## makeswift-nextjs-starter — default `main`, 0 other branches, 0 with commits ahead, 0 tags

(not mirrored locally; branch ancestry not computed)

## x3starkryptoex — default `main`, 0 other branches, 0 with commits ahead, 0 tags

(not mirrored locally; branch ancestry not computed)

## blockchaindocs — default `main`, 0 other branches, 0 with commits ahead, 0 tags

(not mirrored locally; branch ancestry not computed)

## ML_CRYPTO_HACKER — default `master`, 0 other branches, 0 with commits ahead, 0 tags

(not mirrored locally; branch ancestry not computed)

## DEXTOOLSweb3modal — default `master`, 0 other branches, 0 with commits ahead, 0 tags

(not mirrored locally; branch ancestry not computed)


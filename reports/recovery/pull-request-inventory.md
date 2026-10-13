# Pull request inventory (2026-10-13T14:46:39+00:00)

## xxxstar: 29 open, 405 merged, 150 closed without merge

### Open

| # | Branch | Created | Title |
|---|---|---|---|
| 601 | `integrate/merge-train-20261013` | 2026-10-13 | merge train 2026-10-13: 22 PRs and recovered agent work, integrated and fully gated |
| 600 | `test/rc1-three-validator-boot-gate` | 2026-10-13 | test(mainnet): RC1 three-validator boot gate; harden the RC1 smoke test |
| 599 | `runtime/evm-persistence-and-fraud-proof-pins` | 2026-10-13 | feat(evm): persist WASM EVM state, execute as the authenticated caller; pin fraud-proof mutants |
| 598 | `test/gateway-postgres-control-plane` | 2026-10-13 | test(x3-gateway): shared gateway/indexer migration set, proven on real Postgres |
| 597 | `integrate/x3-guardian` | 2026-10-13 | feat(x3-guardian): guardian pallets, forge intelligence and router work, integrated with master |
| 596 | `feat/x3lang-hardening-x3gpu1` | 2026-10-13 | feat(x3-lang): numeric typing, constant bindings and unknown-token hardening |
| 595 | `test/gateway-migrations-cover-tables` | 2026-10-13 | test(x3-gateway): guard that every queried table has a migration |
| 594 | `fix/swarm-core-import-cycle-20261013` | 2026-10-13 | fix(swarm): allow Reaper and schema imports before the agent core |
| 593 | `fix/release-provenance-rc6-rebased` | 2026-10-13 | fix(release): bind RC6 artifacts to clean committed source; fail certification on failed or skipped  |
| 592 | `fix/testnet-finality-evidence-20261013` | 2026-10-13 | fix(testnet): reject divergent or inconsistent finalized evidence |
| 591 | `ci/local-ci-on-push-and-pr` | 2026-10-13 | ci: run local CI automatically on pushes to master and on PRs |
| 590 | `feat/gpu-cpu-reference-20261011` | 2026-10-11 | GPU: add fail-closed secp256k1 CPU reference verifier |
| 589 | `fix/master-red-gates` | 2026-10-09 | ci: make master's three red gates pass (test-cheat, fake-code, npm audit) |
| 588 | `codex/x3lang-canonical-compiler-phase2` | 2026-10-09 | test(x3lang): begin canonical compiler bridge with identity contract |
| 587 | `codex/x3lang-control-flow-parity-20261008` | 2026-10-08 | test(x3lang): verify branches, loops and calls across three engines |
| 586 | `codex/x3lang-compiled-storage-regression-20261008` | 2026-10-08 | test(x3lang): verify compiled storage across kernel submissions |
| 585 | `codex/x3-lang-nested-workspace-gate-20261008` | 2026-10-08 | ci(x3-lang): verify nested compiler and VM workspace |
| 581 | `feat/x3-cluster-control` | 2026-10-03 | x3-cluster: distributed job runner + first physical pipeline (stacked on #578) |
| 578 | `feat/x3gpu1-dual-gpu-accel` | 2026-10-03 | Dual-GPU AI router, cluster tooling, and GPU hashing + secp256k1 verification |
| 555 | `design/onchain-evm` | 2026-09-29 | design: a real EVM on chain (for sign-off) |
| 529 | `dependabot/github_actions/github/codeql-action-4` | 2026-09-28 | chore(deps): bump github/codeql-action from 3 to 4 |
| 528 | `dependabot/github_actions/actions/upload-artifact-7` | 2026-09-28 | chore(deps): bump actions/upload-artifact from 4 to 7 |
| 527 | `dependabot/github_actions/actions-nonmajor-b300d6d0aa` | 2026-09-28 | chore(deps): bump github/codeql-action/upload-sarif from 4.38.0 to 4.38.2 in the actions-nonmajor gr |
| 526 | `dependabot/pip/redis-gte-8.1.0` | 2026-09-28 | chore(deps-dev): update redis requirement from >=5 to >=8.1.0 |
| 525 | `dependabot/pip/pytest-gte-9.1.1` | 2026-09-28 | chore(deps-dev): update pytest requirement from >=8 to >=9.1.1 |
| 524 | `dependabot/pip/aiohttp-gte-3.14.3` | 2026-09-28 | chore(deps-dev): update aiohttp requirement from >=3.9 to >=3.14.3 |
| 523 | `dependabot/npm_and_yarn/packages/polkawallet-bridge-adapter/polkadot/util-14.0.3` | 2026-09-28 | chore(deps): bump @polkadot/util from 13.5.9 to 14.0.3 in /packages/polkawallet-bridge-adapter |
| 522 | `dependabot/npm_and_yarn/packages/polkawallet-bridge-adapter/types/node-26.6.2` | 2026-09-28 | chore(deps-dev): bump @types/node from 20.19.33 to 26.6.2 in /packages/polkawallet-bridge-adapter |
| 521 | `dependabot/npm_and_yarn/packages/polkawallet-bridge-adapter/npm-nonmajor-4a3749d6d8` | 2026-09-28 | chore(deps-dev): bump ts-jest from 29.4.6 to 29.4.14 in /packages/polkawallet-bridge-adapter in the  |

### Closed without merge (possible abandoned work)

| # | Branch | Created | Title |
|---|---|---|---|
| 538 | `dependabot/npm_and_yarn/infra/blockchain-tps/npm_and_yarn-fbf58325b6` | 2026-09-29 | chore(deps): bump the npm_and_yarn group across 2 directories with 1 update |
| 512 | `dependabot/npm_and_yarn/apps/explorer/npm_and_yarn-5964df529b` | 2026-09-25 | chore(deps): bump the npm_and_yarn group across 2 directories with 10 updates |
| 511 | `dependabot/npm_and_yarn/apps/explorer/npm_and_yarn-e8840f269f` | 2026-09-25 | chore(deps): bump the npm_and_yarn group across 2 directories with 12 updates |
| 510 | `dependabot/npm_and_yarn/apps/explorer/npm_and_yarn-5964df529b` | 2026-09-25 | chore(deps): bump the npm_and_yarn group across 2 directories with 10 updates |
| 502 | `docs/public-testnet-alpha-execution-plan` | 2026-09-24 | docs: add Public Testnet Alpha execution plan |
| 498 | `dependabot/cargo/lru-0.16.4` | 2026-09-24 | chore(deps): bump lru from 0.12.5 to 0.16.4 |
| 497 | `dependabot/cargo/curve25519-dalek-5.0.0` | 2026-09-24 | chore(deps): bump curve25519-dalek from 4.1.3 to 5.0.0 |
| 409 | `dependabot/github_actions/actions/checkout-7` | 2026-09-21 | chore(deps): bump actions/checkout from 3 to 7 |
| 408 | `dependabot/github_actions/actions/configure-pages-6` | 2026-09-21 | chore(deps): bump actions/configure-pages from 3 to 6 |
| 407 | `dependabot/github_actions/actions-nonmajor-c5de02d4ff` | 2026-09-21 | chore(deps): bump github/codeql-action/upload-sarif from 4.38.0 to 4.38.1 in the actions-nonmajor gr |
| 388 | `integration/reconcile-20260920` | 2026-09-20 | Integration: consolidate five remaining branch tips for verification |
| 369 | `fix/foundry-real-evm-deploy` | 2026-09-20 | fix(foundry): real EVM deployment via ethers, not simulation (#110 item 1) |
| 342 | `fix/agent-guard-bip39-allow` | 2026-09-19 | fix(agent-guard): stop flagging the bip39 2.x constructor (master's only red gate) |
| 335 | `docs/merge-queue-production-gate-lean` | 2026-09-19 | docs(merge-queue): fix/production-gate-prerequisites is leaning archival |
| 310 | `fix/workspace-membership-batch-10` | 2026-09-19 | workspace batch 10: excluded crates get a real gate — and three of them were broken (26 -> 9 in the  |
| 309 | `fix/workspace-membership-batch-9` | 2026-09-19 | workspace burn-down batch 9: x3-indexer + x3-treasury (19 -> 15 in the gap) |
| 308 | `fix/workspace-membership-batch-8` | 2026-09-19 | workspace burn-down batch 8: apotheosis-tx, dream-mining, swarm-media, voice-to-x3 (24 -> 17 in the  |
| 299 | `batch/20260918T2015Z` | 2026-09-18 | batch 6: SVM native HTLC custody + X3VM recovery matrix + x3-lang intent bridge (25 remote branches  |
| 288 | `fix/private-mempool-real-shamir-threshold` | 2026-09-18 | fix(private-mempool): real Shamir/Lagrange threshold decryption instead of XOR |
| 286 | `dependabot/cargo/cargo-5686e80d00` | 2026-09-18 | chore(deps): bump lru from 0.12.5 to 0.16.3 in the cargo group across 1 directory |
| 258 | `dependabot/cargo/thiserror-2.0.20` | 2026-09-18 | chore(deps): bump thiserror from 1.0.69 to 2.0.20 |
| 257 | `dependabot/cargo/jsonschema-0.56.0` | 2026-09-18 | chore(deps): bump jsonschema from 0.18.3 to 0.56.0 |
| 256 | `dependabot/cargo/actix-cors-0.7.2` | 2026-09-18 | build(deps): bump actix-cors from 0.6.5 to 0.7.2 |
| 255 | `dependabot/cargo/trybuild-1.0.121` | 2026-09-18 | build(deps): bump trybuild from 1.0.120 to 1.0.121 |
| 254 | `dependabot/cargo/actix-rt-2.15.0` | 2026-09-18 | build(deps): bump actix-rt from 2.13.0 to 2.15.0 |
| 253 | `dependabot/cargo/primitive-types-0.13.1` | 2026-09-18 | build(deps): bump primitive-types from 0.12.2 to 0.13.1 |
| 252 | `dependabot/cargo/clap-4.6.7` | 2026-09-18 | build(deps): bump clap from 4.6.6 to 4.6.7 |
| 251 | `dependabot/cargo/solana-sdk-5.0.0` | 2026-09-18 | build(deps): bump solana-sdk from 3.0.0 to 5.0.0 |
| 250 | `dependabot/cargo/keccak-hash-0.11.0` | 2026-09-18 | build(deps): bump keccak-hash from 0.1.2 to 0.11.0 |
| 248 | `dependabot/cargo/smallvec-1.16.1` | 2026-09-18 | chore(deps): bump smallvec from 1.16.0 to 1.16.1 |
| 247 | `dependabot/cargo/k256-0.14.0` | 2026-09-18 | build(deps): bump k256 from 0.13.4 to 0.14.0 |
| 246 | `dependabot/cargo/toml-1.1.5spec-1.1.0` | 2026-09-18 | chore(deps): bump toml from 0.5.11 to 1.1.5+spec-1.1.0 |
| 245 | `dependabot/cargo/ureq-3.4.2` | 2026-09-18 | build(deps): bump ureq from 2.12.1 to 3.4.2 |
| 244 | `dependabot/cargo/solana-program-test-3.0.14` | 2026-09-18 | build(deps): bump solana-program-test from 3.0.0 to 3.0.14 |
| 242 | `dependabot/cargo/syn-3.0.5` | 2026-09-18 | chore(deps): bump syn from 2.0.119 to 3.0.5 |
| 241 | `dependabot/cargo/pollster-1.0.1` | 2026-09-18 | build(deps): bump pollster from 0.3.0 to 1.0.1 |
| 240 | `dependabot/cargo/solana-program-5.0.0` | 2026-09-18 | build(deps): bump solana-program from 3.0.0 to 5.0.0 |
| 239 | `dependabot/cargo/jsonrpsee-0.26.0` | 2026-09-18 | chore(deps): bump jsonrpsee from 0.22.5 to 0.24.11 |
| 238 | `dependabot/cargo/reqwest-0.12.28` | 2026-09-18 | build(deps): bump reqwest from 0.11.27 to 0.12.28 |
| 237 | `dependabot/cargo/indicatif-0.18.6` | 2026-09-18 | build(deps): bump indicatif from 0.17.11 to 0.18.6 |
| 160 | `feat/x3vm-durable-recovery-20260911` | 2026-09-11 | feat(x3vm): persist recovery snapshots atomically |
| 155 | `dependabot/cargo/ark-ec-0.6.0` | 2026-09-10 | chore(deps): bump ark-ec from 0.5.0 to 0.6.0 |
| 154 | `dependabot/cargo/sc-cli-54f11b1` | 2026-09-10 | build(deps): bump sc-cli from `e1f0f7a` to `54f11b1` |
| 152 | `dependabot/cargo/pallet-staking-54f11b1` | 2026-09-10 | chore(deps): bump pallet-staking from `e1f0f7a` to `54f11b1` |
| 151 | `dependabot/cargo/sp-version-54f11b1` | 2026-09-10 | chore(deps): bump sp-version from `e1f0f7a` to `54f11b1` |
| 149 | `dependabot/cargo/sp-keystore-54f11b1` | 2026-09-10 | chore(deps): bump sp-keystore from `e1f0f7a` to `54f11b1` |
| 148 | `dependabot/cargo/frame-benchmarking-cli-54f11b1` | 2026-09-10 | build(deps): bump frame-benchmarking-cli from `e1f0f7a` to `54f11b1` |
| 147 | `dependabot/cargo/rand-0.10.2` | 2026-09-10 | chore(deps): bump rand from 0.8.8 to 0.10.2 |
| 146 | `dependabot/cargo/pallet-sudo-54f11b1` | 2026-09-10 | chore(deps): bump pallet-sudo from `e1f0f7a` to `54f11b1` |
| 143 | `dependabot/cargo/pallet-scheduler-54f11b1` | 2026-09-10 | chore(deps): bump pallet-scheduler from `e1f0f7a` to `54f11b1` |
| 142 | `dependabot/cargo/sc-rpc-api-54f11b1` | 2026-09-10 | chore(deps): bump sc-rpc-api from `e1f0f7a` to `54f11b1` |
| 140 | `dependabot/cargo/sc-rpc-54f11b1` | 2026-09-10 | chore(deps): bump sc-rpc from `e1f0f7a` to `54f11b1` |
| 137 | `dependabot/cargo/sc-basic-authorship-54f11b1` | 2026-09-10 | build(deps): bump sc-basic-authorship from `e1f0f7a` to `54f11b1` |
| 136 | `dependabot/cargo/sc-consensus-aura-54f11b1` | 2026-09-10 | chore(deps): bump sc-consensus-aura from `e1f0f7a` to `54f11b1` |
| 135 | `codex/x3-trading-core-v1-hardening` | 2026-09-10 | fix(x3-lang): harden Trading Core v1 end to end |
| 128 | `ci/master-lineage-gates-20260908` | 2026-09-09 | ci: recover required gates on the master lineage |
| 127 | `dependabot/npm_and_yarn/apps/explorer/npm_and_yarn-4ba7f0e3fa` | 2026-09-08 | build(deps): bump the npm_and_yarn group across 4 directories with 11 updates |
| 126 | `ci/master-lineage-gates-20260908` | 2026-09-08 | ci: move required gates to the master lineage |
| 125 | `docs/grant-readiness-truth-20260908` | 2026-09-08 | docs: make grant and cross-chain readiness evidence-based |
| 122 | `dependabot/npm_and_yarn/apps/dashboard/npm_and_yarn-e628127086` | 2026-09-08 | chore(deps): bump the npm_and_yarn group across 8 directories with 9 updates |
| 121 | `dependabot/npm_and_yarn/apps/dashboard/npm_and_yarn-5b880f561c` | 2026-09-08 | chore(deps): bump the npm_and_yarn group across 11 directories with 11 updates |
| 120 | `dependabot/npm_and_yarn/apps/dashboard/npm_and_yarn-86bd1cad1e` | 2026-09-08 | chore(deps): bump the npm_and_yarn group across 22 directories with 15 updates |
| 119 | `dependabot/npm_and_yarn/apps/dashboard/npm_and_yarn-42e7ba4f26` | 2026-09-08 | chore(deps): bump the npm_and_yarn group across 21 directories with 12 updates |
| 117 | `dependabot/npm_and_yarn/packages/polkawallet-plugin/npm_and_yarn-24621e6fc4` | 2026-09-06 | build(deps): bump the npm_and_yarn group across 22 directories with 12 updates |
| 116 | `dependabot/npm_and_yarn/tests/wallet-integration/ip-address-10.7.0` | 2026-09-06 | build(deps): bump ip-address from 10.1.1 to 10.7.0 in /tests/wallet-integration |
| 115 | `dependabot/npm_and_yarn/apps/dashboard/npm_and_yarn-fe7e21b3fd` | 2026-09-06 | build(deps): bump the npm_and_yarn group across 23 directories with 15 updates |
| 109 | `docs/x3-deployment-runbook` | 2026-08-31 | docs: add gated X3 deployment runbook |
| 106 | `dependabot/npm_and_yarn/apps/dex/npm_and_yarn-258531b2b4` | 2026-07-27 | Bump the npm_and_yarn group across 11 directories with 6 updates |
| 105 | `dependabot/npm_and_yarn/npm_and_yarn-e1683985aa` | 2026-07-26 | Bump tar from 7.5.16 to 7.5.22 in the npm_and_yarn group across 1 directory |
| 104 | `dependabot/npm_and_yarn/apps/dex/npm_and_yarn-2419128bac` | 2026-07-21 | Bump the npm_and_yarn group across 14 directories with 12 updates |
| 103 | `dependabot/npm_and_yarn/apps/dex/npm_and_yarn-6100183905` | 2026-07-20 | Bump the npm_and_yarn group across 15 directories with 12 updates |
| 102 | `dependabot/npm_and_yarn/apps/validators/multi-6b284e118a` | 2026-07-20 | build(deps): bump the npm_and_yarn group across 1 directory with 2 updates |
| 101 | `dependabot/cargo/apps/inferstructor-dashboard/src-tauri/cargo-082059d156` | 2026-07-15 | Bump serde_with from 3.16.1 to 3.21.0 in /apps/inferstructor-dashboard/src-tauri in the cargo group  |
| 100 | `dependabot/npm_and_yarn/apps/dex/npm_and_yarn-4150d50b09` | 2026-07-10 | Bump the npm_and_yarn group across 16 directories with 11 updates |
| 99 | `dependabot/npm_and_yarn/apps/validators/multi-3ef1a22f08` | 2026-07-10 | Bump esbuild and vite in /apps/validators |
| 98 | `dependabot/npm_and_yarn/apps/dex/npm_and_yarn-2ac8b83ff7` | 2026-07-10 | Bump the npm_and_yarn group across 14 directories with 11 updates |
| 97 | `dependabot/npm_and_yarn/apps/explorer/npm_and_yarn-81cae43b49` | 2026-07-10 | Bump the npm_and_yarn group across 14 directories with 11 updates |
| 96 | `dependabot/npm_and_yarn/morphllm/opencode-morph-plugin-2.0.16` | 2026-06-29 | Bump @morphllm/opencode-morph-plugin from 2.0.15 to 2.0.16 |
| 95 | `dependabot/npm_and_yarn/onnxruntime-node-1.27.0` | 2026-06-22 | Bump onnxruntime-node from 1.20.1 to 1.27.0 |
| 94 | `dependabot/npm_and_yarn/apps/inferstructor-dashboard/npm_and_yarn-7db63aa9d7` | 2026-06-21 | Bump the npm_and_yarn group across 12 directories with 12 updates |

## x3-atomic-star: 30 open, 5 merged, 112 closed without merge

### Open

| # | Branch | Created | Title |
|---|---|---|---|
| 163 | `dependabot/github_actions/actions/setup-java-6` | 2026-10-03 | chore(deps): bump actions/setup-java from 4 to 6 |
| 162 | `dependabot/github_actions/actions/configure-pages-6` | 2026-10-03 | chore(deps): bump actions/configure-pages from 3 to 6 |
| 161 | `dependabot/github_actions/actions-nonmajor-b300d6d0aa` | 2026-10-03 | chore(deps): bump github/codeql-action/upload-sarif from 4.38.0 to 4.38.2 in the actions-nonmajor gr |
| 160 | `dependabot/npm_and_yarn/packages/polkawallet-bridge-adapter/polkadot/util-14.0.3` | 2026-10-03 | chore(deps): bump @polkadot/util from 13.5.9 to 14.0.3 in /packages/polkawallet-bridge-adapter |
| 159 | `dependabot/npm_and_yarn/packages/polkawallet-bridge-adapter/multi-b0ec9e4250` | 2026-10-03 | chore(deps-dev): bump jest and @types/jest in /packages/polkawallet-bridge-adapter |
| 158 | `dependabot/npm_and_yarn/packages/polkawallet-bridge-adapter/npm-nonmajor-676ed2b016` | 2026-10-03 | chore(deps-dev): bump ts-jest from 29.4.6 to 29.4.14 in /packages/polkawallet-bridge-adapter in the  |
| 157 | `dependabot/pip/pytest-gte-9.1.1` | 2026-10-03 | chore(deps-dev): update pytest requirement from >=8 to >=9.1.1 |
| 156 | `dependabot/pip/pydantic-gte-2.13.5` | 2026-10-03 | chore(deps-dev): update pydantic requirement from >=2 to >=2.13.5 |
| 155 | `dependabot/pip/pytest-asyncio-gte-1.4.0` | 2026-10-03 | chore(deps-dev): update pytest-asyncio requirement from >=0.24 to >=1.4.0 |
| 154 | `dependabot/cargo/thiserror-2.0.20` | 2026-09-24 | chore(deps): bump thiserror from 1.0.69 to 2.0.20 |
| 153 | `dependabot/cargo/rand_chacha-0.9.0` | 2026-09-23 | chore(deps): bump rand_chacha from 0.3.1 to 0.9.0 |
| 152 | `dependabot/cargo/tokio-tungstenite-0.30.0` | 2026-09-23 | chore(deps): bump tokio-tungstenite from 0.29.0 to 0.30.0 |
| 151 | `dependabot/cargo/libp2p-0.54.1` | 2026-09-23 | chore(deps): bump libp2p from 0.53.2 to 0.54.1 |
| 150 | `dependabot/cargo/curve25519-dalek-5.0.0` | 2026-09-23 | chore(deps): bump curve25519-dalek from 4.1.3 to 5.0.0 |
| 149 | `dependabot/cargo/jsonrpsee-0.26.0` | 2026-09-23 | chore(deps): bump jsonrpsee from 0.24.11 to 0.26.1 |
| 148 | `dependabot/cargo/solana-program-5.0.0` | 2026-09-23 | chore(deps): bump solana-program from 3.0.0 to 5.0.0 |
| 147 | `dependabot/cargo/config-0.14.1` | 2026-09-23 | chore(deps): bump config from 0.13.4 to 0.14.1 |
| 146 | `dependabot/cargo/ndarray-0.17.2` | 2026-09-23 | chore(deps): bump ndarray from 0.15.6 to 0.17.2 |
| 145 | `dependabot/cargo/dashmap-6.2.1` | 2026-09-23 | chore(deps): bump dashmap from 5.5.3 to 6.2.1 |
| 144 | `dependabot/cargo/keccak-hash-0.12.0` | 2026-09-23 | chore(deps): bump keccak-hash from 0.11.0 to 0.12.0 |
| 143 | `dependabot/cargo/axum-0.8.9` | 2026-09-23 | chore(deps): bump axum from 0.7.9 to 0.8.9 |
| 142 | `dependabot/cargo/k256-0.14.0` | 2026-09-23 | chore(deps): bump k256 from 0.13.4 to 0.14.0 |
| 141 | `dependabot/cargo/tower-0.5.3` | 2026-09-23 | chore(deps): bump tower from 0.4.13 to 0.5.3 |
| 140 | `dependabot/cargo/petgraph-0.8.3` | 2026-09-23 | chore(deps): bump petgraph from 0.6.5 to 0.8.3 |
| 139 | `dependabot/cargo/hex-literal-1.1.0` | 2026-09-23 | chore(deps): bump hex-literal from 0.4.1 to 1.1.0 |
| 138 | `dependabot/cargo/ed25519-zebra-4.2.0` | 2026-09-23 | chore(deps): bump ed25519-zebra from 3.1.0 to 4.2.0 |
| 137 | `dependabot/cargo/lsp-types-0.97.0` | 2026-09-23 | chore(deps): bump lsp-types from 0.94.1 to 0.97.0 |
| 136 | `dependabot/cargo/priority-queue-2.7.0` | 2026-09-23 | chore(deps): bump priority-queue from 1.4.0 to 2.7.0 |
| 135 | `dependabot/cargo/cargo-2dfe81512a` | 2026-09-22 | chore(deps): bump lru from 0.12.5 to 0.16.4 in the cargo group across 1 directory |
| 134 | `dependabot/npm_and_yarn/apps/inferstructor-dashboard/npm_and_yarn-fa0e04ea13` | 2026-09-22 | chore(deps-dev): bump esbuild from 0.27.7 to 0.28.2 in /apps/inferstructor-dashboard in the npm_and_ |

### Closed without merge (possible abandoned work)

| # | Branch | Created | Title |
|---|---|---|---|
| 133 | `dependabot/npm_and_yarn/jsdom-30.0.1` | 2026-07-29 | chore(deps-dev): bump jsdom from 28.1.0 to 30.0.1 |
| 132 | `dependabot/npm_and_yarn/jsdom-30.0.0` | 2026-07-27 | chore(deps-dev): bump jsdom from 28.1.0 to 30.0.0 |
| 131 | `dependabot/npm_and_yarn/apps/dex/npm_and_yarn-09e75fd66e` | 2026-07-27 | chore(deps): bump the npm_and_yarn group across 12 directories with 10 updates |
| 130 | `dependabot/npm_and_yarn/apps/dex/npm_and_yarn-0768511ec6` | 2026-07-25 | chore(deps): bump the npm_and_yarn group across 16 directories with 9 updates |
| 129 | `dependabot/npm_and_yarn/apps/dex/npm_and_yarn-e7a8ee1500` | 2026-07-24 | chore(deps): bump the npm_and_yarn group across 17 directories with 8 updates |
| 128 | `dependabot/npm_and_yarn/grammy-1.45.1` | 2026-07-21 | chore(deps): bump grammy from 1.42.0 to 1.45.1 |
| 127 | `dependabot/npm_and_yarn/vitest-4.1.10` | 2026-07-21 | chore(deps-dev): bump vitest from 3.2.4 to 4.1.10 |
| 126 | `dependabot/npm_and_yarn/apps/dex/npm_and_yarn-fcd432bb64` | 2026-07-10 | chore(deps): bump the npm_and_yarn group across 18 directories with 8 updates |
| 125 | `dependabot/npm_and_yarn/apps/dex/npm_and_yarn-0dc7f89dcb` | 2026-07-09 | chore(deps): bump the npm_and_yarn group across 18 directories with 9 updates |
| 124 | `dependabot/npm_and_yarn/apps/inferstructor-dashboard/npm_and_yarn-6b9e7dbf33` | 2026-06-30 | chore(deps): bump the npm_and_yarn group across 17 directories with 10 updates |
| 123 | `dependabot/npm_and_yarn/apps/validators/babel/core-7.29.7` | 2026-06-30 | chore(deps-dev): bump @babel/core from 7.29.0 to 7.29.7 in /apps/validators |
| 122 | `dependabot/npm_and_yarn/apps/inferstructor-dashboard/npm_and_yarn-583864071e` | 2026-06-30 | chore(deps): bump the npm_and_yarn group across 17 directories with 10 updates |
| 121 | `dependabot/npm_and_yarn/apps/inferstructor-dashboard/npm_and_yarn-1f5d1af33f` | 2026-06-30 | chore(deps): bump the npm_and_yarn group across 16 directories with 9 updates |
| 120 | `dependabot/npm_and_yarn/x3fronend/multi-dccbb322fd` | 2026-06-29 | chore(deps): bump esbuild and vite in /x3fronend |
| 119 | `dependabot/npm_and_yarn/apps/x3-intelligence/multi-dccbb322fd` | 2026-06-29 | chore(deps): bump esbuild and vite in /apps/x3-intelligence |
| 118 | `dependabot/npm_and_yarn/apps/validators/multi-dccbb322fd` | 2026-06-29 | chore(deps): bump esbuild and vite in /apps/validators |
| 117 | `dependabot/npm_and_yarn/apps/dex/npm_and_yarn-53af40124b` | 2026-06-29 | chore(deps): bump the npm_and_yarn group across 18 directories with 10 updates |
| 116 | `dependabot/npm_and_yarn/apps/inferstructor-dashboard/npm_and_yarn-293e115860` | 2026-06-29 | chore(deps): bump the npm_and_yarn group across 17 directories with 10 updates |
| 115 | `dependabot/npm_and_yarn/apps/inferstructor-dashboard/npm_and_yarn-969c6d6135` | 2026-06-24 | chore(deps): bump the npm_and_yarn group across 15 directories with 9 updates |
| 114 | `dependabot/npm_and_yarn/apps/inferstructor-dashboard/npm_and_yarn-d90d111059` | 2026-06-24 | chore(deps): bump the npm_and_yarn group across 14 directories with 9 updates |
| 113 | `dependabot/npm_and_yarn/apps/inferstructor-dashboard/npm_and_yarn-277950c4a2` | 2026-06-23 | chore(deps): bump the npm_and_yarn group across 13 directories with 9 updates |
| 112 | `dependabot/npm_and_yarn/apps/inferstructor-dashboard/npm_and_yarn-414ecaa735` | 2026-06-22 | chore(deps): bump the npm_and_yarn group across 13 directories with 9 updates |
| 111 | `dependabot/npm_and_yarn/apps/inferstructor-dashboard/npm_and_yarn-72e1f257d0` | 2026-06-22 | chore(deps): bump the npm_and_yarn group across 14 directories with 9 updates |
| 110 | `dependabot/npm_and_yarn/apps/inferstructor-dashboard/npm_and_yarn-6482ff0194` | 2026-06-22 | chore(deps): bump the npm_and_yarn group across 14 directories with 11 updates |
| 109 | `dependabot/npm_and_yarn/apps/inferstructor-dashboard/npm_and_yarn-32269744cc` | 2026-06-21 | chore(deps): bump the npm_and_yarn group across 17 directories with 11 updates |
| 108 | `dependabot/npm_and_yarn/apps/inferstructor-dashboard/npm_and_yarn-e4f3ab45bd` | 2026-06-20 | chore(deps): bump the npm_and_yarn group across 19 directories with 11 updates |
| 107 | `dependabot/npm_and_yarn/apps/inferstructor-dashboard/npm_and_yarn-7fd1717c2e` | 2026-06-20 | chore(deps): bump the npm_and_yarn group across 18 directories with 10 updates |
| 106 | `dependabot/npm_and_yarn/apps/inferstructor-dashboard/npm_and_yarn-6fc2aeb537` | 2026-06-20 | chore(deps): bump the npm_and_yarn group across 19 directories with 10 updates |
| 105 | `dependabot/npm_and_yarn/apps/inferstructor-dashboard/npm_and_yarn-5f47658d2f` | 2026-06-19 | chore(deps): bump the npm_and_yarn group across 19 directories with 9 updates |
| 104 | `dependabot/npm_and_yarn/apps/inferstructor-dashboard/npm_and_yarn-9548be56ff` | 2026-06-18 | chore(deps): bump the npm_and_yarn group across 16 directories with 8 updates |
| 103 | `dependabot/npm_and_yarn/apps/inferstructor-dashboard/npm_and_yarn-ba3969c9da` | 2026-06-17 | chore(deps): bump the npm_and_yarn group across 16 directories with 8 updates |
| 102 | `dependabot/npm_and_yarn/apps/inferstructor-dashboard/npm_and_yarn-8a1dcb95e4` | 2026-06-17 | chore(deps): bump the npm_and_yarn group across 16 directories with 8 updates |
| 101 | `dependabot/npm_and_yarn/x3fronend/multi-f57e1e291f` | 2026-06-16 | chore(deps): bump esbuild and vite in /x3fronend |
| 100 | `dependabot/npm_and_yarn/apps/x3-intelligence/multi-f57e1e291f` | 2026-06-16 | chore(deps): bump esbuild and vite in /apps/x3-intelligence |
| 99 | `dependabot/npm_and_yarn/apps/inferstructor-dashboard/npm_and_yarn-21a1560aef` | 2026-06-16 | chore(deps): bump the npm_and_yarn group across 17 directories with 9 updates |
| 98 | `dependabot/npm_and_yarn/vitest-4.1.9` | 2026-06-15 | chore(deps-dev): bump vitest from 3.2.4 to 4.1.9 |
| 97 | `dependabot/npm_and_yarn/grammy-1.44.0` | 2026-06-15 | chore(deps): bump grammy from 1.42.0 to 1.44.0 |
| 96 | `dependabot/npm_and_yarn/apps/inferstructor-dashboard/npm_and_yarn-921c3ff1dd` | 2026-06-12 | chore(deps): bump the npm_and_yarn group across 17 directories with 8 updates |
| 95 | `dependabot/npm_and_yarn/apps/inferstructor-dashboard/npm_and_yarn-da7cc5e484` | 2026-06-12 | chore(deps): bump the npm_and_yarn group across 17 directories with 7 updates |
| 94 | `dependabot/npm_and_yarn/apps/inferstructor-dashboard/npm_and_yarn-6f9cfede9c` | 2026-06-10 | chore(deps): bump the npm_and_yarn group across 17 directories with 7 updates |
| 93 | `dependabot/npm_and_yarn/apps/dex/npm_and_yarn-70dbf55e3b` | 2026-06-10 | chore(deps): bump the npm_and_yarn group across 18 directories with 7 updates |
| 92 | `dependabot/npm_and_yarn/apps/dex/npm_and_yarn-db40f5214c` | 2026-06-09 | chore(deps): bump the npm_and_yarn group across 18 directories with 7 updates |
| 91 | `dependabot/npm_and_yarn/apps/dex/npm_and_yarn-f3f719bb2b` | 2026-06-08 | chore(deps): bump the npm_and_yarn group across 18 directories with 8 updates |
| 90 | `dependabot/npm_and_yarn/apps/dex/npm_and_yarn-de89f07c2a` | 2026-06-05 | chore(deps): bump the npm_and_yarn group across 19 directories with 8 updates |
| 89 | `dependabot/npm_and_yarn/apps/dex/npm_and_yarn-5d2cb24160` | 2026-06-04 | chore(deps): bump the npm_and_yarn group across 19 directories with 8 updates |
| 88 | `dependabot/npm_and_yarn/apps/dex/npm_and_yarn-6a369ada2b` | 2026-06-03 | chore(deps): bump the npm_and_yarn group across 18 directories with 8 updates |
| 87 | `dependabot/npm_and_yarn/vitest-4.1.8` | 2026-06-02 | chore(deps-dev): bump vitest from 3.2.4 to 4.1.8 |
| 86 | `dependabot/npm_and_yarn/apps/dex/npm_and_yarn-351ff5771d` | 2026-06-01 | chore(deps): bump the npm_and_yarn group across 18 directories with 7 updates |
| 85 | `dependabot/npm_and_yarn/npm_and_yarn-3ac77625be` | 2026-06-01 | chore(deps-dev): bump vitest from 3.2.4 to 4.1.0 in the npm_and_yarn group across 1 directory |
| 84 | `dependabot/npm_and_yarn/apps/dex/npm_and_yarn-86a30dcdeb` | 2026-05-29 | chore(deps): bump the npm_and_yarn group across 18 directories with 6 updates |
| 83 | `dependabot/cargo/cargo-ef593d95cd` | 2026-05-28 | chore(deps): bump the cargo group across 1 directory with 2 updates |
| 82 | `dependabot/cargo/cargo-2dfe81512a` | 2026-05-28 | chore(deps): bump lru from 0.12.5 to 0.16.4 in the cargo group across 1 directory |
| 81 | `dependabot/npm_and_yarn/apps/dex/npm_and_yarn-6bbee76053` | 2026-05-28 | chore(deps): bump the npm_and_yarn group across 17 directories with 6 updates |
| 80 | `dependabot/npm_and_yarn/apps/dex/npm_and_yarn-df09594930` | 2026-05-23 | chore(deps): bump the npm_and_yarn group across 17 directories with 6 updates |
| 79 | `dependabot/npm_and_yarn/apps/dex/npm_and_yarn-58209faf9e` | 2026-05-22 | chore(deps): bump the npm_and_yarn group across 13 directories with 5 updates |
| 78 | `dependabot/npm_and_yarn/apps/dex/npm_and_yarn-1ab62ea4f2` | 2026-05-22 | chore(deps): bump the npm_and_yarn group across 13 directories with 5 updates |
| 77 | `dependabot/npm_and_yarn/vitest-4.1.7` | 2026-05-20 | chore(deps-dev): bump vitest from 3.2.4 to 4.1.7 |
| 76 | `dependabot/cargo/sp-consensus-36341e5` | 2026-05-20 | chore(deps): bump sp-consensus from `30b9588` to `36341e5` |
| 75 | `dependabot/cargo/frame-benchmarking-cli-36341e5` | 2026-05-20 | chore(deps): bump frame-benchmarking-cli from `30b9588` to `36341e5` |
| 74 | `dependabot/cargo/serde_json-1.0.149` | 2026-05-20 | chore(deps): bump serde_json from 1.0.143 to 1.0.149 |
| 73 | `dependabot/cargo/indicatif-0.18.4` | 2026-05-20 | chore(deps): bump indicatif from 0.17.11 to 0.18.4 |
| 72 | `dependabot/cargo/sp-api-36341e5` | 2026-05-20 | chore(deps): bump sp-api from `30b9588` to `36341e5` |
| 71 | `dependabot/cargo/tokio-cron-scheduler-0.15.1` | 2026-05-20 | chore(deps): bump tokio-cron-scheduler from 0.9.4 to 0.15.1 |
| 70 | `dependabot/cargo/sc-consensus-aura-36341e5` | 2026-05-20 | chore(deps): bump sc-consensus-aura from `30b9588` to `36341e5` |
| 69 | `dependabot/cargo/pallet-balances-36341e5` | 2026-05-20 | chore(deps): bump pallet-balances from `30b9588` to `36341e5` |
| 68 | `dependabot/cargo/toml-0.9.6` | 2026-05-20 | chore(deps): bump toml from 0.5.11 to 0.9.6 |
| 67 | `dependabot/cargo/sc-service-36341e5` | 2026-05-20 | chore(deps): bump sc-service from `30b9588` to `36341e5` |
| 66 | `dependabot/npm_and_yarn/vitest-4.1.6` | 2026-05-20 | chore(deps-dev): bump vitest from 3.2.4 to 4.1.6 |
| 65 | `dependabot/npm_and_yarn/grammy-1.43.0` | 2026-05-20 | chore(deps): bump grammy from 1.42.0 to 1.43.0 |
| 64 | `dependabot/pip/requests-gte-2.34.2` | 2026-05-20 | chore(deps): update requests requirement from >=2.28 to >=2.34.2 |
| 63 | `dependabot/npm_and_yarn/apps/dex/npm_and_yarn-9b17faf10c` | 2026-05-19 | chore(deps): bump the npm_and_yarn group across 12 directories with 5 updates |
| 62 | `dependabot/cargo/tokio-tungstenite-0.29.0` | 2026-05-09 | chore(deps): bump tokio-tungstenite from 0.20.1 to 0.29.0 |
| 61 | `dependabot/cargo/solana-program-4.0.0` | 2026-05-09 | chore(deps): bump solana-program from 3.0.0 to 4.0.0 |
| 60 | `dependabot/cargo/multihash-0.19.5` | 2026-05-09 | chore(deps): bump multihash from 0.18.1 to 0.19.5 |
| 59 | `dependabot/cargo/deadpool-postgres-0.14.1` | 2026-05-09 | chore(deps): bump deadpool-postgres from 0.12.1 to 0.14.1 |
| 58 | `dependabot/cargo/redis-1.2.1` | 2026-05-09 | chore(deps): bump redis from 0.32.7 to 1.2.1 |
| 57 | `dependabot/cargo/sc-transaction-pool-30b9588` | 2026-05-09 | chore(deps): bump sc-transaction-pool from `3009eb2` to `30b9588` |
| 56 | `dependabot/cargo/frame-system-30b9588` | 2026-05-09 | chore(deps): bump frame-system from `3009eb2` to `30b9588` |
| 55 | `dependabot/cargo/sp-keystore-30b9588` | 2026-05-09 | chore(deps): bump sp-keystore from `3009eb2` to `30b9588` |
| 54 | `dependabot/cargo/prost-0.14.3` | 2026-05-09 | chore(deps): bump prost from 0.12.6 to 0.14.3 |

## the-reaper2: 0 open, 1 merged, 0 closed without merge

## atlas-sphere: 2 open, 70 merged, 10 closed without merge

### Open

| # | Branch | Created | Title |
|---|---|---|---|
| 1274 | `dependabot/npm_and_yarn/npm_and_yarn-182df79fae` | 2026-09-04 | chore(deps): bump the npm_and_yarn group across 19 directories with 11 updates |
| 1273 | `rc0-gates-clean` | 2026-04-24 | rc0: hard-invariant gates 11-22 + wave consolidation (target-* stripped) |

### Closed without merge (possible abandoned work)

| # | Branch | Created | Title |
|---|---|---|---|
| 1270 | `chore/unify-curve25519` | 2026-02-13 | chore: investigate unifying curve25519-dalek for Solana/SVM support |
| 1269 | `feat/svm-counter-ci-docs` | 2026-02-13 | test(svm): add SVM counter program, isolated integration test + CI workflow + docs |
| 1267 | `feat/arbitrage-bonding` | 2026-02-08 | feat: Bonding/Collateral (storage-backed) |
| 1265 | `opt/yolo-20260206T175624` | 2026-02-06 | opt: YOLO deterministic orchestration + bench infrastructure |
| 1246 | `chore/roadmap-issue-templates-ci` | 2026-02-06 | chore(roadmap): add issue templates + x3-invariants CI stub |
| 1242 | `copilot/vscode-mis3dhsd-fgwi` | 2026-02-05 | Copilot/vscode mis3dhsd fgwi |
| 105 | `feat/x3-parser-golden-more` | 2026-01-31 | test(golden): add more parser fixtures & golden JSON |
| 104 | `feat/x3-parser-badge` | 2026-01-31 | ci: add parser-specific workflow and README badge |
| 103 | `md-audit/quick-fixes-1` | 2026-01-31 | docs(md): quick wins — keystore helper & .env.example |
| 102 | `feat/x3-parser-ci-golden` | 2026-01-31 | x3-parser: add CI test step and golden AST regression test |

## atlas-sphere-: 1 open, 1 merged, 0 closed without merge

### Open

| # | Branch | Created | Title |
|---|---|---|---|
| 2 | `dependabot/cargo/cargo-5eefeb9384` | 2026-09-04 | Bump bytes from 1.10.1 to 1.12.1 in the cargo group across 1 directory |

## remix-project: 7 open, 0 merged, 2 closed without merge

### Open

| # | Branch | Created | Title |
|---|---|---|---|
| 9 | `dependabot/npm_and_yarn/npm_and_yarn-53736346bd` | 2026-06-05 | Bump the npm_and_yarn group across 3 directories with 9 updates |
| 7 | `dependabot/npm_and_yarn/npm_and_yarn-5d31296a0d` | 2026-06-05 | Bump the npm_and_yarn group across 3 directories with 10 updates |
| 6 | `dependabot/npm_and_yarn/npm_and_yarn-ccec421cae` | 2026-06-05 | Bump the npm_and_yarn group across 3 directories with 11 updates |
| 4 | `dependabot/npm_and_yarn/npm_and_yarn-c7c8e5f2b8` | 2026-06-04 | Bump the npm_and_yarn group across 3 directories with 11 updates |
| 3 | `dependabot/npm_and_yarn/npm_and_yarn-da943d71ff` | 2026-05-30 | Bump the npm_and_yarn group across 3 directories with 12 updates |
| 2 | `dependabot/npm_and_yarn/npm_and_yarn-a509eb9483` | 2026-05-30 | Bump the npm_and_yarn group across 3 directories with 13 updates |
| 1 | `dependabot/npm_and_yarn/npm_and_yarn-91d6c4d4ee` | 2026-05-30 | Bump the npm_and_yarn group across 3 directories with 14 updates |

### Closed without merge (possible abandoned work)

| # | Branch | Created | Title |
|---|---|---|---|
| 8 | `dependabot/npm_and_yarn/npm_and_yarn-ccad07e843` | 2026-06-05 | Bump the npm_and_yarn group across 2 directories with 8 updates |
| 5 | `dependabot/npm_and_yarn/npm_and_yarn-5d31296a0d` | 2026-06-05 | Bump the npm_and_yarn group across 3 directories with 10 updates |

## cline: 2 open, 0 merged, 1 closed without merge

### Open

| # | Branch | Created | Title |
|---|---|---|---|
| 3 | `dependabot/npm_and_yarn/npm_and_yarn-80606371f4` | 2026-05-23 | Bump the npm_and_yarn group across 5 directories with 20 updates |
| 1 | `dependabot/npm_and_yarn/webview-ui/npm_and_yarn-f4b82a7c86` | 2026-05-10 | Bump the npm_and_yarn group across 3 directories with 21 updates |

### Closed without merge (possible abandoned work)

| # | Branch | Created | Title |
|---|---|---|---|
| 2 | `dependabot/npm_and_yarn/npm_and_yarn-f8cbfeef88` | 2026-05-22 | Bump the npm_and_yarn group across 4 directories with 19 updates |

## aave-flash-loan-tutorial: 1 open, 0 merged, 0 closed without merge

### Open

| # | Branch | Created | Title |
|---|---|---|---|
| 1 | `dependabot/npm_and_yarn/aave-flash-loan-v3/npm_and_yarn-5cee0a129d` | 2026-05-10 | Bump the npm_and_yarn group across 2 directories with 17 updates |

## rune: 1 open, 0 merged, 0 closed without merge

### Open

| # | Branch | Created | Title |
|---|---|---|---|
| 1 | `dependabot/npm_and_yarn/github-action/npm_and_yarn-260a72923e` | 2026-05-10 | Bump the npm_and_yarn group across 1 directory with 5 updates |

## atlas-sphere1: 0 open, 1 merged, 0 closed without merge

## spec-kit: 0 open, 1 merged, 0 closed without merge

## no_fear_near_wallet: 1 open, 0 merged, 1 closed without merge

### Open

| # | Branch | Created | Title |
|---|---|---|---|
| 2 | `dependabot/npm_and_yarn/npm_and_yarn-9f9e7816eb` | 2024-12-18 | Bump the npm_and_yarn group across 1 directory with 5 updates |

### Closed without merge (possible abandoned work)

| # | Branch | Created | Title |
|---|---|---|---|
| 1 | `dependabot/npm_and_yarn/npm_and_yarn-e088a6feb8` | 2024-12-16 | Bump the npm_and_yarn group across 1 directory with 5 updates |


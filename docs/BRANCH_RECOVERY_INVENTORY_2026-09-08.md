# Branch Recovery Inventory

**Recorded:** 2026-09-08
**Canonical candidate:** `master`
**Archive:** `archive/main-pre-reconciliation-20260908`

| Branch | Tip | Relation | Decision |
|---|---|---|---|
| `archive/main-pre-reconciliation-20260908` | `5d9deab1138f` | archive | KEEP_ARCHIVE |
| `chore/dependency-upgrades` | `8f75a8cef63d` | contained_in_master | DELETE_CANDIDATE |
| `ci/gpu-soak-062409646` | `0624096466f7` | unique_or_unrelated | HOLD_FOR_COMPONENT_REVIEW |
| `ci/master-lineage-gates-20260908` | `e409b9aed819` | open_pr | KEEP_PENDING_REVIEW |
| `codex/surgical-recovery` | `3a8a152931ee` | unique_or_unrelated | HOLD_FOR_COMPONENT_REVIEW |
| `dependabot/cargo/actix-cors-0.7.1` | `188d9fd3fb62` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `dependabot/cargo/apps/inferstructor-dashboard/src-tauri/cargo-082059d156` | `73ab26b01eb9` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `dependabot/cargo/axum-0.8.9` | `c6d10f0417da` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `dependabot/cargo/bincode-3.0.0` | `e53c579a6e62` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `dependabot/cargo/colored-3.1.1` | `d6fd01e54691` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `dependabot/cargo/config-0.14.1` | `baae1b263e52` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `dependabot/cargo/criterion-0.8.2` | `ed07fd591c55` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `dependabot/cargo/dashmap-6.2.1` | `d59f587f4434` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `dependabot/cargo/deadpool-postgres-0.14.1` | `a8c2ee1c9ff2` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `dependabot/cargo/env_logger-0.11.10` | `a53d0a9a9842` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `dependabot/cargo/minicbor-2.2.2` | `4ee4105eae6f` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `dependabot/cargo/pollster-0.4.0` | `e6b769612369` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `dependabot/cargo/rand-0.10.1` | `d0cc94f00b50` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `dependabot/cargo/redis-1.2.3` | `18d5d897ff6e` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `dependabot/cargo/rlp-0.6.1` | `c12a4da7347a` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `dependabot/cargo/secp256k1-0.30.0` | `c98c8a9a9b46` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `dependabot/cargo/serde_json-1.0.150` | `836d3c328e93` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `dependabot/cargo/sqlx-0.9.0` | `61def65c9885` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `dependabot/cargo/tokio-tungstenite-0.29.0` | `3c6db14582c4` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `dependabot/cargo/toml-1.1.2spec-1.1.0` | `fd00370ed47c` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `dependabot/cargo/wgpu-29.0.3` | `9c7a4ea46374` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `dependabot/npm_and_yarn/apps/dashboard/npm_and_yarn-fe7e21b3fd` | `069a16c8e9ee` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `dependabot/npm_and_yarn/apps/dex/npm_and_yarn-2419128bac` | `7b93e467f6f4` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `dependabot/npm_and_yarn/apps/dex/npm_and_yarn-258531b2b4` | `fc18410084a5` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `dependabot/npm_and_yarn/apps/dex/npm_and_yarn-4150d50b09` | `405e9c6de4c2` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `dependabot/npm_and_yarn/apps/dex/npm_and_yarn-6100183905` | `eeca08765a59` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `dependabot/npm_and_yarn/apps/dex/npm_and_yarn-b3dd0e608d` | `f41c293a534d` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `dependabot/npm_and_yarn/apps/explorer/npm_and_yarn-613b65474c` | `e5b902f64cdf` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `dependabot/npm_and_yarn/apps/inferstructor-dashboard/npm_and_yarn-7db63aa9d7` | `216420620122` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `dependabot/npm_and_yarn/apps/validators/multi-6b284e118a` | `bfdb0e4a8384` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `dependabot/pip/dot-kilo/worktrees/luminous-pecorino/contracts/botchain-tri-vm-genesis/pip-43dfb2fea1` | `661aacd610c0` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `dependabot/pip/psycopg2-binary-gte-2.9.12` | `90bc99c96bad` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `dependabot/pip/requests-gte-2.34.2` | `be67797e1b8c` | obsolete_dependency_branch | DELETE_CANDIDATE_REJECTED |
| `design/x3-funding-os-2026-09-03` | `0b031a7b860d` | reviewed_planning_only | HOLD_REJECTED |
| `docs/grant-readiness-truth-20260908` | `e6c38baf3198` | open_pr | KEEP_PENDING_REVIEW |
| `docs/x3-deployment-runbook` | `9f4700fe5180` | selectively_ported_to_PR_126 | HOLD_UNTIL_PR_126_REVIEW |
| `docs/x3lang-1-0-readiness-design` | `7e04dbc8b45d` | unique_or_unrelated | HOLD_FOR_COMPONENT_REVIEW |
| `feat/x3lang-core-hardening` | `2f4e66abee7a` | unique_or_unrelated | HOLD_FOR_COMPONENT_REVIEW |
| `fix-claude-tx1-cn1-audit` | `df1901a0fe60` | contained_in_master | DELETE_CANDIDATE |
| `fix-x3lang-python` | `3d78fcf7b428` | contained_in_master | DELETE_CANDIDATE |
| `fix/foundry-production-bolts` | `305730c9a7a8` | unique_or_unrelated | HOLD_FOR_COMPONENT_REVIEW |
| `fix/sidecar-router-e2e-gate-20260523` | `09390b075c22` | unique_or_unrelated | HOLD_FOR_COMPONENT_REVIEW |
| `fix/sidecar-signer-gate-20260523` | `7be92a5bb0f4` | unique_or_unrelated | HOLD_FOR_COMPONENT_REVIEW |
| `fix/x3-lang-production-gate` | `f8fcd5b77308` | unique_or_unrelated | HOLD_FOR_COMPONENT_REVIEW |
| `fresh_main` | `15d15badc28a` | unique_or_unrelated | HOLD_FOR_COMPONENT_REVIEW |
| `main` | `5d9deab1138f` | archived_unrelated_history | DELETE_AFTER_CONFIRMATION |
| `master` | `7ddf0f27fa92` | canonical | KEEP |
| `recovery/git-corruption-safe-main-20260524` | `7b86953c50c5` | unique_or_unrelated | HOLD_FOR_COMPONENT_REVIEW |
| `recovery/git-corruption-salvage-20260524-031437` | `07eee09a11ad` | unique_or_unrelated | HOLD_FOR_COMPONENT_REVIEW |
| `recovery/integration-20260520` | `ef5577068812` | unique_or_unrelated | HOLD_FOR_COMPONENT_REVIEW |
| `security/apps-explorer-deps` | `8d3c25f12f60` | unique_or_unrelated | HOLD_FOR_COMPONENT_REVIEW |
| `t5/fix-annotations-20260522-1458` | `ebce3fa6cc5c` | unique_or_unrelated | HOLD_FOR_COMPONENT_REVIEW |
| `t5/fix-t5-blockers-v2` | `a6f4c55d056c` | unique_or_unrelated | HOLD_FOR_COMPONENT_REVIEW |
| `wip/recover-local-changes-20260523` | `9041c7da64be` | unique_or_unrelated | HOLD_FOR_COMPONENT_REVIEW |
| `your-task-branch` | `b19bcef0a5e1` | unique_or_unrelated | HOLD_FOR_COMPONENT_REVIEW |

## Decision rules

- `DELETE_CANDIDATE`: the branch tip is reachable from `master`; deletion removes only the branch name.
- `DELETE_CANDIDATE_REJECTED`: stale automated dependency work is rejected in favor of current `master` dependency state.
- `HOLD`: do not delete until the component-level comparison is closed or a durable archive ref is created.
- No deletion may include `master`, either open-PR branch, or the archive branch.

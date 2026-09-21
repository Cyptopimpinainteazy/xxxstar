# Task Ledger — Repo Consolidation 2026-09-17

| ID | Item | Type | Priority | Status |
|---|---|---|---|---|
| TL-001 | 6 local-only branches + 3 WIP snapshots not durable on `origin` | FIXED — pushed 2026-09-17 | P0 | CLOSED |
| TL-002 | `feat/idempotent-cross-domain-coordinator-20260911` local lineage not on origin | FIXED — pushed as `-pre-rebase-20260917`; all 8 subjects already on live branch | P1 | CLOSED |
| TL-003 | `feat/canonical-cross-domain-proof-bundle-20260911` had 3 pre-rebase commits not on origin | FIXED — pushed as `-pre-rebase-20260917` | P1 | CLOSED |
| TL-004 | ~45 open PRs / ~50 origin branches not merged into `master` — needs triage decision | DEFERRED | P1 | OPEN |
| TL-005 | Main working tree mixes real edits with artifact churn; needs split before committing | FIXABLE_NOW | P2 | OPEN |
| TL-006 | Recovered USB clone dirty content (244 files) | CAPTURED as `wip/consolidation-20260917/recovered-usb-clone`; assessed as mostly stale delta vs master | P2 | CLOSED |
| TL-007 | Several local branches sit on stale bases (2–8 commits behind `master`) and will conflict on merge | DEFERRED | P2 | OPEN |
| TL-008 | Standalone x3-lang prototypes absent from this repo | PRESERVED as `wip/consolidation-20260917/x3-lang-prototype-20260621` (22 files) | P3 | CLOSED |
| TL-009 | Duplicate snapshot artifacts on disk (`xxxstar-main.zip` 505MB, `Recovered-from-USB` copy) | DEFERRED — safe to delete now that content is preserved remotely | P4 | OPEN |
| TL-010 | Local `master` is 3 commits behind `origin/master` | FIXABLE_NOW | P2 | OPEN |

## Local-only work inventory (tips exist nowhere on origin)

| Branch | Ahead of master | Last commit |
|---|---|---|
| `fix/svm-htlc-native-custody` | 34 | svm: enforce native HTLC custody |
| `pr-181-check` | 12 | fix(ci): repair formal gate runtime environment |
| `test/cross-domain-recovery-matrix-20260911` | 10 | Avoid redundant production audit test run |
| `feat/idempotent-cross-domain-coordinator-20260911` | 8 | test(coordinator): persist secret claim ownership |
| `feat/canonical-cross-domain-proof-bundle-20260911` | 3 | feat(atomic): expose canonical cross-domain proof |
| `fix/svm-htlc-native-custody-master` | 2 | ci(svm): run live lifecycle gate on master |
| `agents/setup-instructions-request` | 1 | Saving uncommitted changes before archiving session |
| `wip/consolidation-20260917/main` | 2 | snapshot of main working tree |
| `wip/consolidation-20260917/chatgpt-mainnet` | 1 | snapshot of chatgpt worktree |
| `wip/consolidation-20260917/pasted-text-processing` | 36 | snapshot of prompts→skills conversion |

Already landed or fully superseded (no rescue needed): `pr132-work`, `codex/chatgpt-mainnet-work`, `codex/x3-trading-core-v1`, `chore/reqwest-0.12-rust-highs`, `ci/x3-local-runner-smoke`.

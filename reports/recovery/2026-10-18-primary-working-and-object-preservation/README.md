# Primary worktree and Git-object preservation

Host: x3star1. Source: `/home/lojak/Desktop/xxxstar-main`, HEAD `1beb644603a0f1b6de8bb5882e699fdc99bb2e58`. Captured 2026-10-19 UTC / 2026-10-18 America/Denver. Live master ref was checked and remained `70423054e2d9560a212ab763d4f265f9cdc448d9`.

Implemented and proved:

- All 8 unstaged modified and 122 nonignored untracked entries were privately archived: 130 files, 86,135,995 bytes. Independent extraction matched bytes, SHA256 hashes and modes. An isolated Git repository reproduced the exact binary unstaged patch and passed full fsck.
- Every enumerated primary Git object was packed: 154,915 objects, including 4,682 commits, in a 2,549,433,452-byte pack. Independent import reproduced the exact object inventory, 1,272 refs and 668 raw reflog files. All 699 previously audited primary unreferenced commits are present. Full fsck and the separate unreachable-object fsck both exited 0.
- Source HEAD, index, refs, status, captured worktree bytes and captured reflog bytes stayed unchanged. Original repositories received no ref updates, repairs or integration.

Private archives:

- `/home/lojak/x3-recovery-sources/preserved/primary-working-20261019T024612Z`
- `/home/lojak/x3-recovery-sources/preserved/primary-objects-20261019T024742Z`

Both archives passed SHA256 manifest checks and permission checks. Raw source, configuration, patches, indexes, packs and reflogs stay private. [Working proof](WORKING_RESTORE_RESULT.json) and [object proof](OBJECT_RESTORE_RESULT.json) record archive manifest hashes and exact scope. The earlier [58-file staged archive](../2026-10-18-primary-staged-preservation/README.md) remains independently verified.

## Reconciliation findings

[Exact blob comparison](blob-comparison.json) covers all 130 captured worktree entries against source HEAD, pinned master, refreshed PR #623 and captured PR #562. Path absence or byte difference alone does not establish unique functionality.

The five local untracked gateway migrations already exist byte-for-byte on pinned master and PR #623. The local orchestra evidence-bundle ID guard also exists on master; its only local difference is spaces inside a log message. These items need no separate feature port.

The untracked migration-coverage test is an older version of master's test. An isolated probe proves the local test misses table names ending a SQL literal, while master's version detects them. Retain the stronger target test. Separately, the existing recovered static schema guard failed against the old HEAD's incomplete schema and passed against recovered schema (1 test). These are static Rust tests, not executed SQL or gateway runtime proof. See [static schema verification](gateway-static-verification.json), [regression probe](gateway-guard-regression.json) and [classification](RECOVERY_CLASSIFICATION.json).

The historical router backup parses and has the same 108 function/class names as source HEAD. This does not prove semantic equivalence. No historical router code was executed. Other configuration and infrastructure candidates remain unverified.

## Recovery queue and handoff

The [130-item scoped queue](RECOVERY_QUEUE.json) records exact paths, blob IDs, SHA256 hashes, expected value, risks, recommended actions and validation. Seven patch files are P1 review candidates; 24 implementation/infrastructure files are P2 candidates; 92 configuration/evidence/generated entries need P3 classification; seven already-present or superseded entries are retained as P4. These are recovery priorities, not bug severity ratings.

Remaining blockers: ignored source and nested repositories, other linked-worktree indexes/files, copied worktrees with broken pointers, other object stores, unavailable hosts and off-host preservation. The current index has zero gitlink entries despite a historical `.gitmodules` file; this does not establish absence of nested repositories. Local archives share the source host and do not prove disaster resilience. No workspace build, live-node test, live PostgreSQL test or feature integration was performed.

Files changed: this new metadata package and the parent evidence index in the separate publication clone; private archives and isolated verification repositories outside original checkouts.

Commands run: read-only source status/ref/index/object capture; `git pack-objects --stdout`; local `git bundle verify` and bundle import into a new repository; `git index-pack --stdin`; `git update-ref --stdin` and `git symbolic-ref` only inside the new verification repository; `git cat-file --batch-all-objects`; `git fsck --full --no-reflogs --no-progress`; `git fsck --full --no-reflogs --unreachable --no-progress`; SHA256 and byte comparisons; standalone `rustc --test` for isolated static probes; Python AST parsing without imports. Archive command exits were 0. The two expected negative static probes exited 101; their positive comparisons exited 0.

Proof result: PASS for the stated preservation and static comparisons. Runtime and integration tests: NOT RUN. Completion percent: no overall percentage assigned; infrastructure coverage and broader recovery remain partial.

Next 10 tasks:

1. Preserve full jacktheripper baseline history and remaining working bytes without applying staged deletions.
2. Preserve copied worktree source behind the 15 broken Git pointers.
3. Preserve unreferenced objects in the remaining object stores.
4. Preserve each other linked-worktree index and dirty/untracked files independently.
5. Inspect ignored source with bounded scans and keep secret material out of reports.
6. Review the seven retained local patch files against their real bases.
7. Compare historical router backup function bodies and behavior with current canonical code.
8. Reconcile the staged compiler/runtime changes with PR #623 without reverting newer tests.
9. Reproduce canonical trading integration failures in an isolated checkout.
10. Restore authorized remote audit coverage and record any still-unavailable hosts explicitly.

The original [master audit](../2026-10-18-lost-work-audit/MASTER_RECOVERY_REPORT.md) remains a dated forensic snapshot. This checkpoint resolves the primary worktree and primary object-store preservation portions, not the entire recovery mission.

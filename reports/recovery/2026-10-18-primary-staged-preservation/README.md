# Primary staged compiler/runtime preservation

Captured on x3star1 at 2026-10-19T02:41:40Z (2026-10-18 in America/Denver).

Implemented: a private, independent archive of the 58 staged compiler/runtime file versions in `/home/lojak/Desktop/xxxstar-main`, the exact raw index, a binary patch, staged Git objects and the full history reachable from source HEAD `1beb644603a0f1b6de8bb5882e699fdc99bb2e58`.

Archive: `/home/lojak/x3-recovery-sources/preserved/primary-staged-20261019T024140Z`.
Verification repository: `/home/lojak/x3-recovery-sources/preserved/verification-20261019T024140Z/restored.git`.
The archive has mode 0700; its files and directories deny group/world access. Its 220 checksummed artifacts total 274,708,485 bytes. The source payload, raw index, patch, bundle and command logs stay private; this publication contains verification metadata only.

Proof result: **PASS**. A new bare repository restored the baseline bundle and staged object pack without source alternates or shallow history. All 22,690 index entries, the cached binary patch and all 58 staged blob versions matched. `git fsck --full --no-reflogs --no-progress` exited 0. The reconstructed staged tree is `0f3afa6bcd62ebe3b83a22a0a5c0190b41619898`. Archive checksums and permissions passed. Source HEAD, index, refs, status and the 58 working file versions were unchanged.

Commands run: `git pack-objects --stdout`, `git bundle create`, `git bundle verify`, local bundle `git fetch` into a new bare repository, `git index-pack --stdin`, `git ls-files --stage -z`, `git diff --cached --binary --full-index`, `git write-tree`, `git cat-file blob`, `git fsck --full --no-reflogs --no-progress`, and SHA256/permission/source-state comparisons. Every recorded command completed with exit 0; see [verification](VERIFICATION.json) and [result](RESULT.json).

Not tested: compiler/runtime behavior, live-node interoperability or mainnet readiness. This step changed no production code. The preserved versions have not been integrated into master or PR #623.

Remaining blockers: other unstaged/untracked files, submodule working trees, history outside the captured HEAD and staged objects, copied worktrees and unavailable hosts still need independent preservation or investigation. This is a local copy on the same host, not an off-host backup. The original [audit queue](../2026-10-18-lost-work-audit/RECOVERY_QUEUE.md) remains a dated proposal; this checkpoint completes only its primary staged-source preservation portion. See the [staged comparison](../2026-10-18-primary-staged-followup/README.md) before any integration.

Files changed: this proof package and the parent evidence index in the separate publication clone; new private preservation artifacts outside all original repositories.

Next 10 tasks, in order:

1. Preserve primary unstaged gateway/router files with exact byte hashes.
2. Preserve primary untracked source files separately from secrets and generated data.
3. Inventory and preserve dirty submodule working trees independently.
4. Preserve unreferenced commit object closures before any cleanup.
5. Preserve source bytes behind the 15 broken copied-worktree pointers.
6. Preserve and diagnose the jacktheripper index without repairing the original.
7. Compare duplicate finality fixes with current master and relevant PR contents.
8. Reconcile staged trading behavior with refreshed PR #623 without overwriting newer tests.
9. Reproduce canonical trading stack failures in an isolated checkout.
10. Restore authorized remote audit coverage and record each unavailable host explicitly.

Completion percent: not assigned to the overall recovery mission; it remains partial. This preservation scope passed all listed checks. Integration, broader preservation and host coverage are incomplete.

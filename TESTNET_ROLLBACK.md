# TESTNET_ROLLBACK.md

Rollback/undo procedures for changes made during this audit. The repo is now a git repo
(baseline commit `091dbe3`). General rollback for any code change:

```bash
cd /home/lojak/Desktop/xxxstar-main
git status                    # review what changed
git diff                      # review diffs
# Revert an uncommitted change to a single file:
git checkout -- <path>
# Revert everything to the baseline snapshot:
git reset --hard 091dbe3
```

A `.pre-edit-snapshot/` directory pre-existed in the tree (retained as-is; do not delete).

Procedures per subsystem will be recorded here as they are created/exercised (Phase 8).

# Workstream A — burn down the recorded production panic/unwrap sites

You are a lane of the X3 engineering team. Repo: `/home/lojak/Desktop/xxxstar-main` (branch
`feat/x3-prelaunch-economics-x3lang-cutoff`… check it; master is pushed). Read
`AGENTS.md` first, then this brief.

## Goal

`docs/reports/panic-unwrap-baseline.json` records **520** production panic/unwrap sites (measured
2026-09-27 with the current scanner). That number is 43 higher than the commit the previous baseline
was taken at, and the growth is concentrated:

| file | +sites |
| --- | --- |
| `crates/gpu-swarm/src/admin.rs` | +10 |
| `node/src/chain_spec.rs` | +8 |
| `crates/gpu-swarm/src/crown/scrapyard.rs` | +7 |
| `crates/x3-order-window/src/lib.rs` | +3 |
| `crates/x3-accel/src/lib.rs` | +3 |
| `crates/gpu-swarm/src/{crown/auditor.rs,jobs/mev_discovery.rs,performance/memory_pooling.rs}` | +2 each |
| plus ~7 files at +1 |

Bring the count **down**, worst-first. A site may only keep its `unwrap`/`expect`/`panic!` if failure
is genuinely impossible and that invariant is written down in a comment next to it — AGENTS.md §20.
Otherwise propagate the error (`?`, explicit `match`, a named error variant) or return a
fail-closed default. Where a function's signature cannot express the failure yet, change the
signature; do not add `unwrap_or_default()` that turns a failure into a silent success for something
security-relevant.

## Measure, don't guess

```bash
python3 scripts/audit/panic_unwrap_scan.py > /tmp/panic.json   # 'counts' + 'findings.production'
python3 -c "import json;d=json.load(open('/tmp/panic.json'));print(d['counts'])"
bash scripts/mainnet/panic_unwrap_audit.sh                      # ratchet check vs the baseline
python3 scripts/audit/panic_unwrap_self_test.py                 # scanner self-test
```

`bash scripts/mainnet/panic_unwrap_audit.sh` will FAIL while the live count is above the baseline —
that is expected and is what you are fixing. **Do not run `--update-baseline`**; the baseline moves
only when you have landed the reduction and I (the primary agent) agree.

## Rules

- Do not weaken, delete, `#[ignore]`, or loosen a test. Break the behaviour on purpose, watch the
  test go red, restore, watch it go green, and record that in your evidence.
- Keep each commit focused on one or two files. Run the narrowest tests for what you touched
  (`cargo test -p <crate>`), then a broader one if the change is not obviously local.
- `cargo clippy -p <crate> --all-targets -- -D warnings` must be clean for what you touch.
- **Commit only your own paths. Do not `git add -A`. Do not push** — message me (`/root`) with the
  commit hash, the measured before/after count, and the commands you ran. I will push.
- If a site is in a file another lane is editing (check `git status`), leave it and note it.

## Deliverable

A commit (or a few) that lowers the production count, with the per-file delta in the commit message,
plus a short report to me: files, commands, before/after counts, what is left.

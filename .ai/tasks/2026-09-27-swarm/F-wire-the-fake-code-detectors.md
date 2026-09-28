# Workstream F — the mandated fake-code detectors have never run

You are a lane of the X3 engineering team. Repo: `/home/lojak/Desktop/xxxstar-main`. Read
`AGENTS.md` first (its "Required Proof" section mandates the fake-code scan), then this brief.

## The finding (verified by another lane on 2026-09-27)

* `scripts/x3-detect-stubs.sh` and `scripts/x3-detect-test-cheats.sh` — the detectors `AGENTS.md`
  names under "Forbidden" — are **invoked by nothing**, and they **time out**: exit 124 at 240 s.
  They walk build output such as `x3fronend/out/_next/…`, so the scan has never actually run in this
  repository. A mandated check that cannot complete is worse than no check, because the requirement
  reads as satisfied.
* `scripts/proof/verify_receipts.py` is not executable (exit 126).

Reproduce all three before changing anything, and record the exact commands and outputs.

## Your files

`scripts/x3-detect-stubs.sh`, `scripts/x3-detect-test-cheats.sh`, `scripts/proof/verify_receipts.py`,
the new baseline/ratchet files you add under `docs/reports/` or `scripts/`, and **one** gate block in
`scripts/local-ci.sh`. Do not touch `pallets/**`, `runtime/**`, `crates/**`, `feature-matrix/**`,
`FEATURE_REGISTRY.toml`, `docs/audit/**` or `audit-artifacts/**` — other lanes own those.

## The work

1. **Bound the walk.** Exclude build output and vendored trees (`target/`, `*/out/`, `node_modules/`,
   `.next/`, `dist/`, `.wt-*`, `*.tar.gz`), and make the scan finish in seconds, not minutes. Measure
   the before/after wall time; the number goes in the commit message.
2. **A ratchet, not a wall of red.** These detectors will fire on years of legitimate `TODO`s and on
   `#[cfg(test)]` mocks that are entirely allowed. Follow the pattern this repository already uses for
   panics (`scripts/audit/panic_unwrap_scan.py` + `docs/reports/panic-unwrap-baseline.json`): record a
   baseline of the current findings with a hash of each finding's identity, and fail only when the
   count grows. Never regenerate a baseline to hide a regression — if you must re-baseline, say what
   grew and why in the commit message.
3. **Cheat detection must be precise, not a grep for the word "mock".** A finding is a *cheat* when a
   test asserts a constant, is a no-op body, or mocks the thing under test in the production path.
   Say in the script's header which shapes it detects and which it deliberately does not, so nobody
   reads silence as coverage.
4. **Wire them in**: add `fake-code scan` and `test-cheat scan` gates (names may follow the file's own
   convention) to the fast set in `scripts/local-ci.sh`, next to the existing panic ratchet entry.
   They must be *useful on the current tree*: either green because the tree is at the baseline, or red
   with a named, small list that you fix. A gate that is red on master and stays red is not a gate.
5. `verify_receipts.py`: set the executable bit and add whatever it needs to run (a `--help` that
   works is enough to prove it is invokable; if it cannot run, say so and propose the fix instead of
   guessing).

## Rules

* Do not weaken, delete or `#[ignore]` an existing test. Do not edit the repository's *code* to make a
  detector pass — the detectors are about the tree as it is; report what they find and fix only what
  is a genuine defect you can prove.
* Break-it-first: inject one real stub (an empty `fn` in a scratch copy of a file the detector scans),
  watch the gate go red naming it, restore byte-identically, watch it go green.
* Python 3.10 / bash-compatible, no new third-party dependencies, no network.
* **Never `git add -A`.** Stage only your own paths, by explicit path. **Do not push** — report to
  `/root` with: commit hash, file list, exact commands and results, the baseline counts, the wall-time
  before/after, and the break-it-first evidence.

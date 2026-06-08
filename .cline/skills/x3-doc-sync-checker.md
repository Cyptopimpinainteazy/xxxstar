# Skill: X3 Doc Sync Checker

## Purpose
Compare docs against source and mark outdated claims. Documentation that lies is worse than no documentation.

## Use When
- After implementation changes.
- When docs reference specific files, commands, or APIs that may have changed.
- Before claiming documentation is complete.

## Inputs To Inspect
- `docs/` — all documentation.
- `README.md` — project readme.
- `SECURITY.md` — security docs.
- `docs/reports/` — status reports.
- Source files referenced by docs.

## Checks To Perform
- Do named files in docs still exist at the stated path?
- Do commands in docs still work?
- Do example code snippets compile/run?
- Do API references match current signatures?
- Do status numbers (percentages, counts) match reality?

## Proof To Require
- List of broken doc references.
- Commands verified as working.
- Outdated numbers flagged.

## Output Format
- Docs reviewed: <count>
- Broken references: [list]
- Outdated claims: [list]
- Verified accurate: <count>
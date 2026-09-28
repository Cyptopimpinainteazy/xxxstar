# Workstream E — make the repo scanner an execution-support agent (P6)

You are a lane of the X3 engineering team. Repo: `/home/lojak/Desktop/xxxstar-main`. Read
`AGENTS.md` first, then this brief.

## What the row says today

`FEATURE_REGISTRY.toml` → `[repo_scanner_agent]`: `readiness_score = 25`, `crate_or_service =
scripts/swarm/swarm_scan.sh`, `mode = "LIVE_TESTNET"`, one required test
(`swarm_scan_generates_report`), blockers `"Dev-ops tooling — not part of blockchain runtime"` and
`"Single test; no CI gate"`.

It is a reporting script. The release prompt wants an agent whose output is *actionable*:

```text
severity
exact file
exact symbol
why it matters
suggested fix
test required
release gate affected
```

and "where safe, the agent should create a patch rather than merely record the issue".

## Your files

`scripts/swarm/**`, plus **one** new gate line in `scripts/local-ci.sh` and its test. Do not touch
`pallets/**`, `runtime/**`, `crates/**`, `feature-matrix/**`, `FEATURE_REGISTRY.toml`,
`audit-artifacts/**`, or `docs/audit/**`: other lanes own those right now, a `make mainnet-check`
is running against this tree, and a runtime-graph edit invalidates the attestation it just recorded.

## The work

1. **Findings, not prose.** The scanner must emit a machine-readable report (JSON) *and* a readable
   summary, each finding carrying the seven fields above. A finding without a file and a line/symbol
   is not a finding.
2. **The classes the release prompt names**, each detected from the repository rather than from a
   hand-maintained list where that is possible:

   ```text
   TODO/FIXME in production paths
   dead modules / unreachable crates
   half-wired pallets (storage or calls nothing reaches)
   missing runtime APIs the pallets declare
   uncovered extrinsics (no test names them)
   missing or zero WeightInfo (the `()` implementation in a runtime)
   panic/unwrap growth against docs/reports/panic-unwrap-baseline.json
   stale docs / stale feature registry entries
   stale release evidence
   unmerged implementation branches
   ```

3. **Determinism.** Same tree, same report — order findings by (severity, path, line). No embedded
   timestamps inside the compared payload, no absolute host paths in the findings.
4. **A real test**, not a constant assertion: the existing `swarm_scan_generates_report` should now
   assert the report's *shape* (every finding has the seven fields; severities are from the fixed
   set; JSON and Markdown agree on the finding count) against a **fixture tree** you construct inside
   the test, so it proves the scanner parses rather than that a string exists. Add cases for two
   deliberate defects in the fixture (one TODO in a production path, one un-gated crate) and assert
   they are found with the right severity, plus one clean file that must produce nothing.
5. **A CI gate** so the scanner is no longer outside CI: add `test swarm-scan` (or the naming the
   file already uses) to `scripts/local-ci.sh` in the fast set — follow the shape of neighbouring
   entries and keep the runtime short (a fixture run, not a full-repo crawl).
6. **Patch generation, only where it is safe.** If you generate patches, they must be written under
   `.ai/` and never applied automatically. Say so in the output header. Do not patch the repository
   yourself.

## Rules

* Do not weaken, delete or `#[ignore]` an existing test. No fake green: if a class cannot be detected
  honestly with reasonable effort, say so in the report's `unimplemented_checks` list rather than
  emitting a finding you cannot justify.
* Break-it-first: introduce one of the fixture defects into a scratch copy of the *scanner's input*,
  watch the specific test go red, restore, watch it go green. Record the two outputs.
* Python 3.10 compatible, no new third-party dependencies. Shell out to `rg` if available and fall
  back to a pure-Python walk otherwise (this box's toolchain has been wiped before mid-session).
* **Never `git add -A`.** Stage only your own paths. **Do not push.** Report to `/root`
  with: commit hash, file list, exact commands and results, the finding classes implemented and the
  ones you deliberately left in `unimplemented_checks`, and the break-it-first evidence.

# X3 Atomic Star — Mainnet Readiness Audit (2026-09-05, commit `fbd4613b`)

This directory is a complete, evidence-based mainnet-readiness audit of the X3 Atomic Star repository at commit `fbd4613bd8769ac7422278fae441af1b302a1c88` (branch `master`).

## Contents

| File | Purpose |
|---|---|
| `booklet.pdf` | The full 61-page rendered report — start here. |
| `source/` | Maintainable Typst source for the booklet (see Regeneration below). |
| `findings.json` | Canonical machine-readable **defect register**: 33 findings, one record per defect, with severity/file/line/evidence/recommendation. |
| `feature-matrix.csv` | Canonical machine-readable **capability-completeness register**: 67 rows, one per feature explicitly evaluated across the audit's seven domains. This is a *different taxonomy* from `findings.json` — features describe capabilities and their status; findings describe defects. A feature can be VERIFIED overall while still having an associated finding (e.g. F056/HIGH-02). |
| `executive-summary.md` | Standalone 1-2 page summary suitable for grant/sponsor review, without needing the full PDF. |
| `manifest.json` | Every generated file in this directory with its purpose, generation timestamp, the audited commit SHA, and a SHA-256 checksum. |
| `diagrams/` | Present for standalone reusable chart/diagram assets; the booklet's three data charts and three architecture diagrams are generated inline from `source/charts.typ` and `source/chapters/03-architecture-overview.typ` — see note below if you want them as separate files. |
| `evidence/` | Reserved for raw command-output snapshots; the evidence actually cited in this audit is summarized in Appendix B of the booklet with exact commands and exit codes (the raw session logs are session-local and not durable, hence the summary-in-appendix approach rather than copying ephemeral logs here). |

## How This Was Produced

Seven independent domain investigations (consensus/networking, transaction lifecycle, state/storage, cryptography/keys, contracts/VM/cross-chain, tokenomics, APIs/ops/proof-gates/performance) each produced a feature table with 8 columns (Feature, Claimed, Actual implementation, Wiring, Tests, Status, Evidence quality, Missing work). Those seven tables were consolidated into `feature-matrix.csv`. Findings were consolidated separately into `findings.json`, each with an exact `file:line` citation, evidence quality label, failure scenario, and recommendation. The booklet (`source/booklet.typ`) reads both files directly at build time via Typst's `json()`/`csv()` functions — chart data, severity counts, and the full findings/feature tables in the booklet are never hand-retyped, eliminating drift between the machine-readable artifacts and the prose.

## Regenerating `booklet.pdf`

Prerequisites:
- [Typst](https://typst.app) 0.12.0 or later (`typst --version` to check).
- The `cetz` 0.3.1 package, either already cached locally (`~/.cache/typst/packages/preview/cetz/0.3.1`) or reachable via the `@preview` package registry over the network at compile time.

Command (run from this directory):

```bash
typst compile --root . source/booklet.typ booklet.pdf
```

`--root .` is required so that `source/data.typ`'s relative reads of `../findings.json` and `../feature-matrix.csv` resolve correctly.

**Note on charts:** all charts and diagrams are hand-drawn using `cetz`'s core drawing primitives (`draw.rect`, `draw.line`, `draw.content`) rather than `cetz.plot` / `cetz-plot`. As of this writing the cached `cetz` 0.3.1 package's plotting submodule is a stub that panics on use (`cetz-plot` was split into its own, separately-versioned package and is not bundled), so this booklet deliberately avoids it for offline-reproducibility reasons — the build should not depend on a plotting library that isn't actually usable.

**Note on `//` inside content:** Typst treats `//` as a line-comment start in markup mode. Any literal `//` (e.g. a Substrate dev key like `//Alice`) must either be wrapped in backtick raw spans (`` `//Alice` `` — raw spans are lexed literally and are safe) or have its slashes escaped (`\/\/Alice`) if written as plain markup text. Both patterns are used correctly in the current source; if you add new content mentioning a `//`-prefixed value, follow the same rule.

**Note on `~` for approximation:** Typst reserves the bare `~` character for a non-breaking space, not a literal tilde. Any approximate figure (e.g. "approximately 140 crates") must either spell out "approximately"/"roughly" or escape the tilde as `\~140`. The current source uses the escaped form; do not reintroduce a bare `~` before a number.

## Regenerating `feature-matrix.csv`

The CSV was built programmatically (not hand-typed) from the seven domain investigation `.md` files' feature tables, using a small Python script that writes via the `csv` module so quoting/escaping is guaranteed correct. Those source `.md` files lived in a session-local scratchpad directory during the original audit and are not durable — `feature-matrix.csv` in this directory is the durable artifact; treat it as canonical going forward rather than trying to regenerate it from the (now-gone) source files.

## Two Taxonomies, Not One

`findings.json` (defect-oriented, "what is wrong") and `feature-matrix.csv` (capability-oriented, "what exists and its status") intentionally use different granularity and different status vocabularies in places. Do not attempt to merge them into a single table — cross-reference by finding ID (e.g. `HIGH-02`) where a feature row's "Missing work" column names one.

## Do Not Confuse With Sibling Directories

Two other directories exist under `audit-artifacts/mainnet-readiness/` that are **not** part of this audit and must not be copied from or overwritten:
- `2026-09-05-6a24d8cf-audit/` — a prior, complete audit for an **older commit** (`6a24d8cf`).
- `fbd4613b/` (no date prefix) — a separate, independently-produced audit for the **same commit** by a different authoring process. Its own `README.md`/`executive-summary.md` should be treated as independent commentary, not a supplement to this one — where the two disagree, re-check the underlying source yourself rather than assuming either is correct.

## A Note on Environment Stability

During the original audit session, this repository was observed under continuous, autonomous modification by a separate agent system on the host machine (committing as `openclaw-agent`), including live changes to runtime and consensus source files. Every finding in this audit cites the exact commit `fbd4613bd8769ac7422278fae441af1b302a1c88`; before acting on any `file:line` citation, confirm your checkout's `git log -1` matches that commit, or re-verify the citation against current `HEAD` if it has moved on.

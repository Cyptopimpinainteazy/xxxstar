# X3 Atomic Star — Mainnet Readiness Audit (2026-09-06, commit `fbd4613b`)

This directory is a self-contained, independent audit deliverable produced by Claude (Anthropic) in a single read-only session against commit `fbd4613bd8769ac7422278fae441af1b302a1c88` of the X3 Atomic Star repository. It does not overwrite or depend on any other directory under `audit-artifacts/mainnet-readiness/` (several prior audits already exist there from other sessions/agents — they are separate, untouched artifacts).

## Contents

| File | Purpose |
|---|---|
| `X3-ROAD-TO-MAINNET.pdf` | The full rendered booklet (18 pages): executive brief, architecture, feature scorecard, findings by severity, fake-completeness report, security notes, launch-gate checklist, prioritized recovery plan, final truth statement, appendices. |
| `source/report.typ` | Maintainable Typst source for the PDF above. |
| `findings.json` | Machine-readable findings register (19 entries) plus the exact verification commands run this session and their results. |
| `feature-matrix.csv` | Feature-by-feature completeness matrix (48 rows) backing the scorecard in the PDF. |
| `EXECUTIVE_SUMMARY.md` | Standalone summary suitable for grant/sponsor/partner conversations. |
| `diagrams/*.svg` | The three figures embedded in the PDF, as standalone reusable SVG assets. |
| `manifest.json` | File list with SHA-256 checksums and provenance. |
| `checksums.txt` | Plain `sha256sum`-format checksum file for `sha256sum -c` verification. |

## Regenerating the PDF

Requires [Typst](https://typst.app) (this was built with Typst 0.12.0):

```bash
cd audit-artifacts/mainnet-readiness/2026-09-06-fbd4613b-claude
typst compile --root . source/report.typ X3-ROAD-TO-MAINNET.pdf
```

The `--root .` flag is required so Typst can resolve the `../diagrams/*.svg` image references relative to this directory.

To re-render pages to PNG for visual review (as was done to verify this deliverable before finalizing it):

```bash
typst compile --root . --format png --ppi 110 source/report.typ /tmp/page-{p}.png
```

## Verifying checksums

```bash
sha256sum -c checksums.txt
```

## Scope note

This audit is intentionally scoped to what could be independently verified — by direct source reading (file:line cited throughout) and safe local command execution (`cargo check`, `cargo audit`, `forge test`, targeted `grep`/`git log` sweeps) — in a single session. It does not include: booting a multi-node network, moving funds, deploying contracts, sustained load/chaos testing, or a full line-by-line review of all 140 workspace crates. Every claim in the PDF carries an evidence-quality tag (Confirmed by execution / Confirmed by static inspection / Claimed by documentation / Inferred / Not verified / BLOCKED) so a reader can tell what was actually checked from what was not. See "Scope, Limitations, and How to Read This Report" (page 3 of the PDF) for full detail.

## Re-auditing after fixes

The findings in `findings.json` each include a `verification_after_fix` field describing exactly how to confirm a fix. Re-running the "verification_commands_run" list in that file against a later commit is a reasonable way to spot-check whether the repository's status has materially changed.

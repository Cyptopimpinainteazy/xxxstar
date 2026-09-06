# X3 Atomic Star: The Road to Mainnet

Read-only audit by **Codex / AI-assisted analysis**, 5 September 2026.

**Public testnet: NO-GO. Mainnet: NO-GO. Evidence readiness: 20/100.**
The 127-page manual records 29 findings: 3 Critical, 18 High, 7 Medium and 1 Low. Its 64 scoped capabilities comprise 2 VERIFIED, 18 IMPLEMENTED BUT UNVERIFIED, 29 PARTIAL, 3 BLOCKED, 4 DISCONNECTED, 5 PLACEHOLDER and 3 MISSING. This is an evidence score, not percent code written or a formal independent security certification.

## Deliverables

| File or directory | Purpose |
|---|---|
| [X3-ROAD-TO-MAINNET.pdf](X3-ROAD-TO-MAINNET.pdf) | Final illustrated, bookmarked field manual; 127 A4 pages |
| [source.md](source.md) | Complete readable manuscript |
| [report-source.json](report-source.json) | Canonical structured rendering source |
| [executive-summary.md](executive-summary.md) | Standalone summary for engineers, sponsors and grant reviewers |
| [findings.json](findings.json) | Detailed evidence, scenarios, fixes and acceptance requirements for all 29 findings |
| [feature-completeness.csv](feature-completeness.csv) | Detailed 64-capability matrix |
| [scorecard.json](scorecard.json) | Explicit indicators, weights, calculations and uncertainty |
| [recovery-plan.json](recovery-plan.json) | Phased engineering backlog; proposed acceptance tasks, not completed fixes |
| [launch-gates.json](launch-gates.json) | Six proposed objective launch gates and nonwaivable safety criteria |
| [benchmark-results.csv](benchmark-results.csv) | Blank results template; no fabricated throughput measurements |
| [provenance.json](provenance.json) | Branch, commit, working-tree qualification and exclusions |
| [figure-register.json](figure-register.json), [assets/](assets/) | 26 separately reusable SVG and vector PDF figures |
| [pdf-navigation.json](pdf-navigation.json) | Final chapter page numbers and table register |
| [evidence/verification-ledger.json](evidence/verification-ledger.json), [evidence/](evidence/) | Commands, exit codes, logs, inventories, source hashes and advisory evidence |
| [audit-harness/](audit-harness/) | Narrow RPC test harness and adversarial proof rejection tests against actual repository code |
| [visual-review/](visual-review/) | Contact sheets covering all 127 final pages |
| [artifact-validation.json](artifact-validation.json) | Package consistency, source preservation and PDF validation results |
| [manifest.json](manifest.json), [manifest.sha256](manifest.sha256) | Per-file purposes, UTC creation times, base commit and SHA-256; detached checksum of manifest |
| [author_report.py](author_report.py), [visuals.py](visuals.py), [render_report.py](render_report.py) | Maintainable manuscript/figure/PDF generation |
| [validate_package.py](validate_package.py), [make_manifest.py](make_manifest.py) | Validation and integrity manifest generation |

The manifest lists every shipped file individually. No production source was changed; all pre-existing changes were preserved.

## Audited source and reproducibility

Base commit: `6a24d8cf38f2522ddf9ae0b47011fd59a9984208`, branch `master`. No repository remote URL was available. The source tree contained extensive existing tracked and untracked changes. **Checking out the commit alone does not recreate the audited source.** Match the owner's preserved working tree against `evidence/source-hashes.json`, `provenance.json` lockfile hashes and `evidence/working-tree-before.txt` before reproducing code observations. The hash inventory covers 6,909 selected text files; it does not certify every binary, excluded vendor file or unrecorded file.

The booklet itself can be regenerated from this frozen package without compiling the blockchain. Re-running experiments needs the matching working tree, toolchain and offline dependencies. Audit commands ran in `/tmp/x3-audit-20260905/repo`; that temporary checkout is not part of the durable package. Sensitive environment/key files, build outputs and installed JavaScript dependencies were excluded. Keep regeneration output in a copy if preserving the delivered manifest.

## Regenerate the document

Tested document dependencies: system Python 3.10.12, ReportLab 3.6.8, Pillow 9.0.1 for contact sheets, Poppler `pdfinfo`/`pdftotext`/`pdftoppm`, and DejaVu Sans/Bold/Mono fonts under `/usr/share/fonts/truetype/dejavu/`. The PATH Python 3.14 environment lacked ReportLab; use `/usr/bin/python3` in this environment. This workflow performs no automatic installations.

Run from this artifact directory:

```bash
/usr/bin/python3 author_report.py
/usr/bin/python3 visuals.py
/usr/bin/python3 render_report.py
pdfinfo X3-ROAD-TO-MAINNET.pdf
/usr/bin/python3 validate_package.py
```

`author_report.py` rebuilds the manuscript and structured report from the frozen audit data. To make editorial changes, maintain that generator; editing `source.md` alone does not change the PDF. To render an already edited `report-source.json` directly, skip the authoring step. The renderer recomputes the table of contents and bookmarks. PDF metadata timestamps can differ between renders; regenerate checksums after any render. Do not reuse these findings as evidence for another commit.

Render pages for visual inspection:

```bash
mkdir -p /tmp/x3-report-preview
pdftoppm -scale-to 900 -png X3-ROAD-TO-MAINNET.pdf /tmp/x3-report-preview/page
```

Review all pages for layout, especially long paths, tables and figures. The delivered PDF underwent all-page raster/contact-sheet review and selected larger page review. Automated checks validate score arithmetic, counts, source hashes, page bounds, replacement glyphs, figure assets and chapter heading/TOC presence. Bookmarks/page labels also received visual review. The PDF is not tagged for assistive technology; the Markdown source is provided as an alternate reading format.

`validate_package.py` assumes this directory remains three levels below the repository root, as delivered. It validates source preservation against that repository and writes `artifact-validation.json` plus the post-audit Git status. For standalone PDF regeneration without the repository, render and inspect the PDF; source-preservation validation requires the original source tree.

## Reproduce bounded tests

The harnesses include or depend on actual source, with test-only assertions. No fake production adapters are introduced. From this artifact directory, with matching source and dependencies:

```bash
CARGO_NET_OFFLINE=true CARGO_TARGET_DIR=/tmp/x3-audit-rpc-target cargo test --locked --manifest-path audit-harness/rpc/Cargo.toml
CARGO_NET_OFFLINE=true CARGO_TARGET_DIR=/tmp/x3-audit-proof-target cargo test --locked --manifest-path audit-harness/proof/Cargo.toml
```

Observed results: RPC harness 4 passed. All 3 adversarial proof rejection tests failed because malformed payloads were accepted; that failure is evidence of H01, not a healthy release signal. The tests establish helper behavior, not an executed fund-loss exploit. Production gateway authorization and router selection limitations are documented in H01.

`evidence/verification-ledger.json` is the authoritative original command/exit-code record. Initial workspace attempts timed out; completed workspace check/test/clippy and release/testnet-feature builds failed with WASM E0152 duplicate core. The full Rust test suite therefore did not run. Selected Python DSL tests yielded 6 passes and 4 failures. `pnpm`, `python`, Forge and Anchor were unavailable; isolated npm test/build lacked Vitest/TypeScript. Migration collection was blocked by absent Alembic; database-mutating migration tests were not executed. No live node transaction lifecycle, multi-node finality, restore drill or sustained blockchain performance was proven.

The requested recursive fake-code scan returned exit 2 with 54,017 recorded candidate lines; traversal errors were not retained, so completeness is not claimed. The separate first-party candidate inventory contains 15,967 matches. Matches are candidates, not automatically defects. Only paths, line numbers and matched tokens were retained for that scan. Cached dependency auditing was not refreshed: the configured scan ignored 35 IDs; the unsuppressed scan found 53 advisory/package-version matches, not 53 confirmed exploitable node vulnerabilities.

## Integrity and sensitive-data handling

After all final checks and visual review:

```bash
/usr/bin/python3 make_manifest.py
sha256sum -c manifest.sha256
/usr/bin/python3 make_manifest.py --verify
```

`manifest.json` contains SHA-256, size, purpose, UTC filesystem creation time (or modification-time fallback where birth time is unavailable), and base commit for each generated file. It excludes itself and its detached hash to avoid a circular checksum. It excludes Python bytecode caches. The manifest hash is stored in `manifest.sha256`.

The input copy excluded sensitive key/environment files. The package records paths and safe diagnostics, not environment dumps or credential values. A high-confidence credential-pattern scan found no matches; this is a heuristic check, not a guarantee against every secret representation. No public-chain signing, transfers, deployments or production credential use occurred.

## Completion boundary

This deliverable completes the requested audit and completion blueprint within the recorded evidence scope. It does not fix the 29 findings, certify all code, prove blocked tests passed, or make X3 launch-ready. The most urgent engineering sequence is: restore the build; authenticate external headers and finality anchors; secure rollback provenance; replace structural proof acceptance; complete persistent VM execution and signed transaction submission; then demonstrate multi-node finality/recovery and enforce independent launch review. Full acceptance tasks and rollback requirements are in the field manual and recovery plan.

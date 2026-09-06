# Live X3 readiness

A local evidence workflow for the existing X3 audit. Task updates and verification runs regenerate HTML, five reusable SVG charts, CSV, Markdown and a PDF. The watcher also notices source changes, evidence expiration and altered logs. The original 127-page audit remains a historical baseline; the live report is its current completion/evidence supplement, not a rewritten claim that the original source findings still apply to changed code.

## Open the report

From the repository root:

```bash
./scripts/readiness/start-local.sh
```

Open **http://127.0.0.1:8765/live/**. Keep the terminal open; Ctrl-C stops the watcher and HTTP server. This binds only to loopback. Use `./scripts/readiness/start-local.sh 8766` for another port. It does not install a boot service or publish a website. Browser refresh is every 10 seconds; a new snapshot appears when evidence changes. Running `watch` never runs tests or executes commands from state automatically.

The initialized data store is `audit-artifacts/mainnet-readiness/live/`. `current.json` points to the latest complete snapshot. Older snapshots retain their data and hashes. CLI mutations also refresh immediately, without needing the watcher.

Required here: Python 3.10+, Git, ReportLab and DejaVu fonts. The tested renderer is system `/usr/bin/python3` with ReportLab 3.6.8. The default PATH Python lacks that package. No automatic installation is performed. Use `READINESS_PYTHON` to select another compatible interpreter for the startup script. `refresh --no-pdf` is an explicit HTML-only option; normal refresh fails if PDF rendering fails and preserves the previous published pointer.

## Update a task

```bash
/usr/bin/python3 scripts/readiness/workflow.py task FIX-C01 in_progress
/usr/bin/python3 scripts/readiness/workflow.py task FIX-C01 awaiting_verification
```

Even requesting `completed` cannot complete a task without fresh passing checks, a recorded acceptance review, and completed dependencies. Task progress and readiness are different measurements.

Register and run an actual verification command (arguments after `--command` are passed as an argument vector, without an implicit shell):

```bash
/usr/bin/python3 scripts/readiness/workflow.py check-add header-rejection --targets FIX-C01 FT32 --timeout 600 --command cargo test --locked -p pallet-cross-chain-validator --features std
/usr/bin/python3 scripts/readiness/workflow.py run header-rejection
```

Check registration does not execute the command. Only `run` executes it. Use trusted local test commands: this is a recorder, not a command sandbox or a detector of inadequate test coverage. A zero exit from an irrelevant command does not prove acceptance. Reviewers must inspect the actual cases, fixtures, scope and results. Do not put credentials in command arguments or review notes; those are retained as audit data.

The recorder uses a minimal environment, no inherited generic credential variables, `CARGO_NET_OFFLINE=true`, a disposable Cargo target under the data store, a timeout, and retained redacted logs. A failed command returns a failing CLI exit status but still records the failure and refreshes the report. Logs over 16 MiB are marked truncated and cannot earn credit. Source changes during execution invalidate the result.

After inspecting the acceptance requirement and all required test evidence, record a named review:

```bash
/usr/bin/python3 scripts/readiness/workflow.py review FIX-C01 closure --reviewer 'Your name / protocol security' --note 'Describe the rejection, no-write and canonical-positive cases actually reviewed' --checks header-rejection
```

All checks bound to a task are required for closure. If additional checks are registered, include all of them. A review is bound to the exact task acceptance contract, source fingerprint and receipt IDs. Editing acceptance, adding required checks, changing source, or rerunning a check requires a fresh review. The UI distinguishes requested progress from verified completion.

## Update feature readiness

Features have five independent 20-point criteria: implemented, wired, tested, executed, reproducible. A review of a task does not automatically establish every feature criterion. Bind an applicable check to the feature and review each criterion for which evidence exists:

```bash
/usr/bin/python3 scripts/readiness/workflow.py review FT32 tested --reviewer 'Your name' --note 'Describe the actual negative and positive test evidence' --checks header-rejection
```

Use `implemented`, `wired`, `executed`, or `reproducible` only when that separate claim was examined. Passing a compiler check cannot establish a finalized transaction or cryptographic correctness. Every review must reference fresh passing checks bound to that feature.

Subsystem scores use the original audit's explicit weights. A Critical finding caps the overall score at 20. The score can go down on regression or stale evidence. Closing all tracked findings produces **NOT ASSESSED**, never automatic GO: independent release-gate, custody, network and launch authorization work remains outside a numeric score.

## Historical evidence and freshness

Initialization verifies every original manifest entry and compares the 6,909 original recorded source hashes and recorded lockfile hashes. Original feature criteria can carry forward only if that source matches. Imported criteria remain visibly labeled historical evidence on unchanged source.

The live fingerprint includes Git-visible tracked and untracked files and executable bits. It excludes `.git`, target/node_modules, named vendor directories, audit artifacts, screenshots, bytecode/cache outputs, keystore directories, `.env*`, `.pem` and `.key` files. Missing tracked files and symlink targets affect the fingerprint; symlinks are not followed. Git-ignored files are outside the scope unless tracked. This is a defined source scope, not proof of unchanged external toolchains, network state or production secrets.

Freshness is deliberately conservative across the whole recorded tree: even a documentation or workflow edit can invalidate previous evidence. Baseline evidence and new run records expire after 30 days by default (`max_evidence_age_days`). New code changes require fresh execution and review, so readiness can temporarily drop while engineering progresses. The historical score remains visible separately. Generated live output does not invalidate itself.

Local review names are operator attestations, not authenticated digital signatures. Hash checks catch missing or edited logs and snapshots; someone with write access to both state and evidence can rewrite them. Independent reviews and external CI attestations are not claimed by this local implementation.

## Commands and data

```bash
/usr/bin/python3 scripts/readiness/workflow.py status
/usr/bin/python3 scripts/readiness/workflow.py refresh
/usr/bin/python3 scripts/readiness/workflow.py watch --interval 10
/usr/bin/python3 scripts/readiness/workflow.py verify audit-artifacts/mainnet-readiness/live/snapshots/ACTUAL-SNAPSHOT-ID
/usr/bin/python3 -m pytest scripts/readiness/tests -q
```

To create a **new** store without overwriting history:

```bash
/usr/bin/python3 scripts/readiness/workflow.py --store audit-artifacts/mainnet-readiness/live-new init --baseline audit-artifacts/mainnet-readiness/2026-09-05-6a24d8cf-audit
```

`init` refuses an existing state. Every store must be beneath repository `audit-artifacts/` so output stays outside source fingerprints.

- `state.json`: task definitions, dependencies, check definitions, reviews and receipt references; CLI is the recommended writer.
- `evidence/<id>.json` and `.log`: exact command, exit, timing, source fingerprint and retained log hashes.
- `snapshots/<id>/`: HTML, PDF, five SVG charts, feature CSV, Markdown, full evaluated JSON, state snapshot and SHA-256 manifest.
- `current.json`: pointer advanced only after all requested outputs succeed and validate.
- `verification/`: implementation test and operational validation evidence.

Keep the original finding acceptance requirements when changing task definitions. Deleting a finding is not closure. This tool supports the original scoped backlog; changes to its scope and scoring policy require human review. The built-in watch process serializes operations with a file lock and does not automatically prune evidence or snapshots.

## Validation scope

Automated tests use real temporary Git repositories and real subprocesses, including failures, timeouts, source changes, missing/tampered evidence, dependencies, acceptance changes, stale reviews and failed PDF publication. The PDF renderer retains a ReportLab 3.6.8 upstream deprecation warning under Python 3.10; rendering is tested. No production blockchain code is changed by this workflow.

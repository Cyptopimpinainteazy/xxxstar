# X3 CI / Actions Policy

The gate of record for this repository is **local CI**, not GitHub-hosted Actions.

## Canonical verification

- `scripts/local-ci.sh` runs the real gate suite with real exit codes.
- `.githooks/pre-push` invokes `scripts/local-ci.sh --pre-push`.
- Trusted Linux workflows use the self-hosted runner labels `[self-hosted, Linux, X64, x3]`.
- Heavy, release, security, deployment, desktop, and cross-domain workflows are retained for **manual dispatch**.
- Cross-domain reusable workflows may also retain `workflow_call` where another manual workflow composes them.

## Automatic GitHub Actions policy

Automatic `pull_request` / `push` hosted CI is intentionally disabled. GitHub-hosted jobs on this account terminate before executing any steps, so they produce red checks without verification and drown out real failures.

Do **not** move untrusted pull-request code onto a self-hosted runner. PR-time verification happens on the author's machine through the pre-push hook before the branch is pushed.

## Local gate modes

```bash
scripts/local-ci.sh
scripts/local-ci.sh --live
scripts/local-ci.sh --cross
scripts/local-ci.sh --variants
scripts/local-ci.sh --release
scripts/local-ci.sh --deep
scripts/local-ci.sh --all
```

See `docs/local-ci.md` for the full gate list, evidence format, and runner policy.

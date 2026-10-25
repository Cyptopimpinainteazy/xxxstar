# X3 CI / Actions Policy

The gate of record for this repository is **local CI**, not GitHub-hosted Actions.

## Canonical verification

- `scripts/local-ci.sh` runs the real gate suite with real exit codes.
- `.githooks/pre-push` invokes `scripts/local-ci.sh --pre-push`.
- Trusted Linux workflows use the self-hosted runner labels `[self-hosted, Linux, X64, x3]`.
- Heavy, release, security, deployment, desktop, and cross-domain workflows are retained for **manual dispatch**.
- Cross-domain reusable workflows may also retain `workflow_call` where another manual workflow composes them.

## Automatic GitHub Actions policy

Automatic GitHub-**hosted** CI is disabled because these jobs terminate before executing test steps. The trusted `local-ci.yml` workflow instead runs on self-hosted runners for master pushes and same-repository PRs; forks are excluded explicitly.

Draft PRs run a lightweight, non-signoff preflight. When a PR is marked ready for review, the exact-head full local CI suite runs and publishes the required `x3/local-ci` status. Pre-push focused checks on low-risk files cannot satisfy that requirement. Do **not** run untrusted fork code on self-hosted runners.

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

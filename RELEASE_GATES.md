# Release Gates

**Canonical source:** `FEATURE_REGISTRY.toml`
**Reviewed:** 2026-09-08
**Current readiness:** 51% average across 15 implemented registry entries
**Release decision:** Blocked

Readiness scores measure implementation, integration, tests, CI enforcement, security controls, and operational evidence. They do not measure lines of code.

## Gate commands

- `make guard`
- `make test`
- `make audit`
- `make mainnet-check`
- `make fresh-machine-check`

A command counts only when its complete output is tied to the exact reviewed commit.

## Current enforcement status

The repository contains workflows for build, lint, tests, security scanning, provenance, and release checks. On 2026-09-08, GitHub refused to start jobs because the account was locked due to a billing issue. Configured workflows are not passing evidence while that condition remains.

Draft PR #126 retargets workflow triggers from the disconnected `main` lineage to the active `master` lineage. It must not merge until GitHub Actions can run and the required jobs pass.

## Mainnet claim rule

Do not call X3 mainnet-ready or production-ready unless all of the following are true:

1. every required gate passes on the exact release commit;
2. every critical registry feature meets its approved threshold;
3. external audits and public staging criteria in `LAUNCH_SCOPE.md` are complete;
4. branch protection for `master` is verified and enforced;
5. external bridge paths have production quorum, finalized-root validation, key custody, and recovery evidence.

None of those conditions may be replaced by an old report, generated checklist, or expected command output.

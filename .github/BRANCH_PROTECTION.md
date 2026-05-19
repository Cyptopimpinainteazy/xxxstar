# GitHub Branch Protection Setup

> This document describes the branch protection rules to configure in GitHub.
> These are configured via GitHub's UI or GitHub CLI.

## Protected Branches

### 1. `main` Branch
**Purpose:** Production-ready releases only

```bash
# Via GitHub CLI:
gh repo rule create --include-default-rules --branch main \
  --require-code-review-count 2 \
  --require-approvals-from-code-owners \
  --require-status-checks \
  --require-branches-to-be-up-to-date \
  --dismiss-stale-reviews
```

**Rules:**
- ✅ Require 2 code review approvals
- ✅ Require CODEOWNERS approval
- ✅ Require all status checks to pass (format, lint, tests, security)
- ✅ Require branches to be up to date before merging
- ✅ Dismiss stale pull request approvals when new commits are pushed
- ✅ Require commit signatures
- ✅ Include administrators in restrictions

### 2. `develop` Branch
**Purpose:** Integration baseline for features

```bash
gh repo rule create --include-default-rules --branch develop \
  --require-code-review-count 1 \
  --require-status-checks \
  --require-branches-to-be-up-to-date \
  --dismiss-stale-reviews
```

**Rules:**
- ✅ Require 1 code review approval
- ✅ Require all status checks to pass
- ✅ Require branches to be up to date before merging
- ✅ Dismiss stale pull request approvals

### 3. `sprint-*` Branches (Pattern)
**Purpose:** Sprint-specific feature branches

```bash
gh repo rule create --include-default-rules --branch "sprint-*" \
  --require-code-review-count 1 \
  --require-status-checks \
  --require-branches-to-be-up-to-date
```

**Rules:**
- ✅ Require 1 code review approval
- ✅ Require all status checks to pass
- ✅ Manual merge (no auto-merge)

---

## Setting Up via GitHub Web UI

1. **Navigate to:** Repository → Settings → Branches
2. **Add Rule:**
   - Branch name pattern: `main` / `develop` / `sprint-*`
   - Check all boxes below:
     - ✅ Require a pull request before merging
     - ✅ Require approvals (count = 2 for main, 1 for develop)
     - ✅ Require review from Code Owners
     - ✅ Dismiss stale pull request approvals when new commits are pushed
     - ✅ Require status checks to pass before merging
     - ✅ Require branches to be up to date before merging
     - ✅ Require commit signatures
     - ✅ Include administrators

---

## Required Status Checks

All of these must pass:

- `Format (format)` — cargo fmt check
- `Lint (lint)` — cargo clippy check
- `Test (test) [lib]` — unit tests
- `Test (test) [integration]` — integration tests
- `Build Release (build-release)` — full release build
- `Security Audit (security-audit)` — cargo audit

---

## Enforcement

These rules apply to:
- ✅ All users (including repository administrators)
- ✅ All pull requests
- ✅ Force pushes are blocked

---

## Exceptions

To bypass these rules temporarily (if needed):

```bash
# Emergency bypass (only for critical hotfixes)
# GitHub > Settings > Branches > "Allow force pushes" > Select users
# (Not recommended; use hotfix branch instead)
```

---

## Verify Rules Applied

```bash
# Via GitHub CLI
gh repo rule list --branch main
gh repo rule list --branch develop
gh repo rule list --branch "sprint-*"
```

Expected output:
```
Branch pattern: main
  - Requires 2 approvals
  - Requires CODEOWNERS approval
  - Requires status checks
  - Requires up-to-date
  - Dismisses stale reviews
  - Requires commit signatures
```

---

## First-Time Setup Checklist

- [ ] `.github/CODEOWNERS` file created
- [ ] `.github/workflows/build.yml` configured
- [ ] Branch protection rules created for `main`
- [ ] Branch protection rules created for `develop`
- [ ] Branch protection rules created for `sprint-*` pattern
- [ ] All required status checks enabled
- [ ] Administrators restricted
- [ ] Test run: Create dummy PR and verify all checks pass

---

## Emergency Releases (If Needed)

For critical hotfixes that must bypass normal review:

1. Create `hotfix/critical-{issue-id}` branch
2. Push with `--force-if-includes` (requires explicit approval)
3. Only admins can override via:
   - Temporarily disable rule, merge, re-enable
   - Document reason in Slack #releases

**Avoid this at all costs.** Use normal PR process.

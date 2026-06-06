# Branch Merge Guide - Consolidating All Features into Main

This guide provides instructions for merging all feature branches into the `main` branch to create a unified codebase with all features and improvements.

## Current State

- **Main Branch**: `abf4ecd` - Contains basic infrastructure and Alembic lint fixes
- **Master Branch**: `eed18cf` - Alternative base branch (appears older)
- **Total Branches**: 28 branches with various features and improvements

## Recommended Merge Strategy

### Phase 1: Core Infrastructure (Priority: High)

These branches contain critical fixes and should be merged first:

1. **chore/alembic-idempotency-check** (`a19755e`)
   - Adds Alembic idempotency check (runs upgrade head twice)
   - Clean merge expected

2. **chore/alembic-idempotent-guard** (`dcf5f8d`)
   - Makes migrations idempotent by cleaning orphaned sequences
   - Asserts alembic.ini logger sections
   - Clean merge expected

3. **ci/codecov-action** (`a41e81f`)
   - Adds Codecov integration
   - May conflict with existing CI workflows

### Phase 2: Bug Fixes and Security (Priority: High)

4. **copilot/fix-security-bug-issues** (PR #9)
   - Fixes hardcoded secrets in workflows
   - Fixes PostgreSQL port mapping
   - Removes duplicate code
   - Fixes contract compilation issues
   - **MUST BE MERGED** for security

5. **copilot/fix-all-pull-requests** (PR #6)
   - Fixes command injection vulnerability in GitHub Actions
   - **MUST BE MERGED** for security

### Phase 3: Workflow and CI Improvements (Priority: Medium)

6. **chore/e2e-ci-improvements** (`6653e61`)
   - E2E test improvements

7. **copilot/fix-workflow-errors** (PR #7)
   - Adds package-lock.json
   - Updates Lighthouse CI action version

8. **copilot/fix-workflow-issues-apps/swarm-apps/dash-legacy-2-legacy-2board-legacy-2-legacy-2** (`bc9c085`)
   - Swarm apps/dash-legacy-2-legacy-2board workflow fixes

### Phase 4: Feature Branches (Priority: Medium)

9. **feature/apps/swarm-apps/dash-legacy-2-legacy-2board-legacy-2-legacy-2-e2e** (PR #3) - `23573a3`
   - TypeScript migration for UI components
   - Jest + ts-jest + Testing Library setup
   - Playwright e2e tests
   - **Large changeset** - review carefully

10. **feature/apps/dash-legacy-2-legacy-2board-mvp-final** (`65356ed`)
    - Dashboard MVP final version

11. **feature/apps/dash-legacy-2-legacy-2board-mvp-clean** (`23573a3`)
    - Dashboard MVP clean version (same SHA as apps/swarm-apps/dash-legacy-2-legacy-2board-legacy-2-legacy-2-e2e)

12. **feat/frontend/ui/cli-and-banner** (`0664823`)
    - CLI and banner UI features

13. **feat/async-reputation-pg-repo** (`758c527`)
    - Async reputation PostgreSQL repository
    - Adds Alembic import sanity check

14. **feature/sigill-aggregator-helper** (`d4819df`)
    - Signal aggregator helper

### Phase 5: TypeScript Cleanup (Priority: Low)

15. **chore/tsx-cleanup-2** (`99022f9`)
    - TypeScript/TSX cleanup

### Phase 6: Core Platform Features (Priority: High)

16. **feature/x3-kernel-task1** (PR #1) - `3e84ef4` / `f7e6744`
    - Atomic Trade Engine pallet implementation
    - AMM adapter interfaces
    - Trade graph pathfinding
    - **Large changeset** - contains Rust/Substrate code

### Phase 7: Testing and Production (Priority: Medium)

17. **staging/production-hardening** (`dc55cc4`)
    - Production hardening changes

18. **test/alembic-lint-fail** (`1301d12`)
    - Test branch for Alembic lint failures (may not need to merge)

### Phase 8: Copilot PRs (Review Required)

19. **copilot/sub-pr-1** (PR #2) - BMAD workflow documentation fixes
20. **copilot/sub-pr-3** (PR #4) - Contract compilation optimization  
21. **copilot/sub-pr-3-again** (PR #5) - Code review feedback
22. **copilot/set-up-copilot-instructions** (PR #55, draft)
23. **copilot/set-up-copilot-instructions-again**
24. **copilot/vscode-mis3dhsd-fgwi** - Base for some sub-PRs

### Skip These Branches

- **opt/yolo-20251209T114158** - Experimental/temporary branch
- **copilot/merge-all-feature-branches** - Current working branch

## Merge Commands

### Option A: Manual Sequential Merge (Recommended)

```bash
# Start from latest main
git checkout main
git pull origin main

# Phase 1: Infrastructure
git merge --no-ff chore/alembic-idempotency-check -m "Merge: Add Alembic idempotency check"
git merge --no-ff chore/alembic-idempotent-guard -m "Merge: Add Alembic idempotent guard"
git merge --no-ff ci/codecov-action -m "Merge: Add Codecov integration"

# Phase 2: Security Fixes
git merge --no-ff copilot/fix-security-bug-issues -m "Merge: Fix security vulnerabilities"
git merge --no-ff copilot/fix-all-pull-requests -m "Merge: Fix command injection vulnerability"

# Phase 3: CI/Workflow
git merge --no-ff chore/e2e-ci-improvements -m "Merge: E2E CI improvements"
git merge --no-ff copilot/fix-workflow-errors -m "Merge: Fix workflow errors"
git merge --no-ff copilot/fix-workflow-issues-apps/swarm-apps/dash-legacy-2-legacy-2board-legacy-2-legacy-2 -m "Merge: Fix swarm apps/dash-legacy-2-legacy-2board workflow"

# Phase 4: Features
git merge --no-ff feature/apps/swarm-apps/dash-legacy-2-legacy-2board-legacy-2-legacy-2-e2e -m "Merge: Swarm apps/dash-legacy-2-legacy-2board e2e tests"
git merge --no-ff feat/frontend/ui/cli-and-banner -m "Merge: CLI and banner UI"
git merge --no-ff feat/async-reputation-pg-repo -m "Merge: Async reputation repository"
git merge --no-ff feature/sigill-aggregator-helper -m "Merge: Signal aggregator"

# Phase 5: Cleanup
git merge --no-ff chore/tsx-cleanup-2 -m "Merge: TypeScript cleanup"

# Phase 6: Core Platform
git merge --no-ff feature/x3-kernel-task1 -m "Merge: Atomic Trade Engine"

# Phase 7: Production
git merge --no-ff staging/production-hardening -m "Merge: Production hardening"

# Push to main
git push origin main
```

### Option B: Create Unified Feature Branch (Alternative)

If you prefer to test everything together first:

```bash
# Create integration branch
git checkout -b integration/all-features main
git pull origin main

# Merge all feature branches
for branch in \
  chore/alembic-idempotency-check \
  chore/alembic-idempotent-guard \
  ci/codecov-action \
  copilot/fix-security-bug-issues \
  copilot/fix-all-pull-requests \
  chore/e2e-ci-improvements \
  copilot/fix-workflow-errors \
  feature/apps/swarm-apps/dash-legacy-2-legacy-2board-legacy-2-legacy-2-e2e \
  feat/frontend/ui/cli-and-banner \
  feat/async-reputation-pg-repo \
  chore/tsx-cleanup-2 \
  feature/x3-kernel-task1 \
  staging/production-hardening
do
  echo "Merging $branch..."
  git merge --no-ff "$branch" -m "Merge: $branch"
  if [ $? -ne 0 ]; then
    echo "Conflict in $branch - resolve manually"
    exit 1
  fi
done

# Test everything
# npm test
# cargo test
# ./scripts/run_e2e_tests.sh

# If tests pass, merge to main
git checkout main
git merge --no-ff integration/all-features
git push origin main
```

## Conflict Resolution

Common conflict areas to watch for:

1. **Package files**: `package.json`, `package-lock.json`, `Cargo.toml`
2. **CI workflows**: `.github/workflows/*.yml`
3. **Configuration**: `alembic.ini`, `tsconfig.json`
4. **Documentation**: `docs/root/README.md` files

### Resolving Conflicts

```bash
# When merge conflicts occur:
git status  # See conflicting files
git diff    # Review conflicts

# Edit conflicting files manually, then:
git add <resolved-files>
git commit -m "Resolve merge conflicts from <branch-name>"
```

## Testing Requirements

After each phase or major merge:

### Frontend (apps/swarm-apps/dash-legacy-2-legacy-2board-legacy-2-legacy-2)
```bash
cd apps/swarm-apps/dash-legacy-2-legacy-2board-legacy-2-legacy-2
npm install
npm run tsc --silent
npm test
npm run e2e:test
```

### Backend (Python/Alembic)
```bash
python -m pytest
# Check Alembic migrations
cd alembic && alembic upgrade head && alembic downgrade base
```

### Rust/Substrate (if applicable)
```bash
cargo test
cargo build --release
```

## Post-Merge Cleanup

After successfully merging to main:

```bash
# Delete merged branches (optional)
git branch -d chore/alembic-idempotency-check
git push origin --delete chore/alembic-idempotency-check

# Or delete all merged branches at once
git branch --merged main | grep -v "main" | xargs git branch -d
```

## Rollback Plan

If issues are discovered after merging:

```bash
# Find the commit before merges
git log --oneline -20

# Create rollback branch
git checkout -b rollback/pre-merge <commit-sha>

# Or revert specific merges
git revert -m 1 <merge-commit-sha>
```

## Notes

- Some branches may have identical SHAs (e.g., `feature/apps/dash-legacy-2-legacy-2board-mvp-clean` and `feature/apps/swarm-apps/dash-legacy-2-legacy-2board-legacy-2-legacy-2-e2e`) - these are duplicates
- Review all PR descriptions before merging to understand changes
- Consider merging PR #3 and PR #1 carefully as they contain large changesets
- The security fixes in PR #6 and PR #9 should be prioritized
- Test thoroughly between phases to catch integration issues early

## Validation Checklist

- [ ] All CI workflows pass
- [ ] No hardcoded secrets remain
- [ ] All tests pass (unit, integration, e2e)
- [ ] Documentation is up to date
- [ ] No duplicate code
- [ ] Security scan passes
- [ ] Application builds successfully
- [ ] Database migrations work correctly

# Quick Merge Execution Checklist

Use this checklist when executing the branch merge.

## Pre-Merge Preparation

- [ ] Backup current main branch
  ```bash
  git checkout main
  git pull origin main
  git branch backup/main-$(date +%Y%m%d)
  ```

- [ ] Create integration branch (recommended)
  ```bash
  git checkout -b integration/all-features main
  ```

- [ ] Review documentation
  - [ ] Read docs/reports/BRANCH_MERGE_GUIDE.md
  - [ ] Read docs/reports/FEATURE_ANALYSIS.md

- [ ] Ensure clean working directory
  ```bash
  git status  # Should show nothing to commit
  ```

## Phase 1: Critical Security Fixes ⚠️

**Priority**: IMMEDIATE - Do this first!

- [ ] Merge `copilot/fix-security-bug-issues` (PR #9)
  - Removes hardcoded password from CI workflow
  - Fixes PostgreSQL port mapping
  - Removes duplicate code
  
- [ ] Merge `copilot/fix-all-pull-requests` (PR #6)
  - Fixes command injection vulnerability
  
- [ ] Run security scan
  ```bash
  grep -r "password.*=" .github/workflows/
  # Should return no hardcoded passwords
  ```

## Phase 2: Database Infrastructure

- [ ] Merge `chore/alembic-idempotent-guard`
  - Makes migrations idempotent
  
- [ ] Merge `chore/alembic-idempotency-check`
  - Adds CI validation for idempotency
  
- [ ] Test Alembic migrations
  ```bash
  cd alembic
  alembic upgrade head
  alembic downgrade base  
  alembic upgrade head  # Should work twice without errors
  ```

## Phase 3: CI/CD Improvements

- [ ] Merge `ci/codecov-action`
  - Adds code coverage reporting
  
- [ ] Merge `copilot/fix-workflow-errors` (PR #7)
  - Fixes npm ci failure
  - Updates Lighthouse action
  
- [ ] Merge `chore/e2e-ci-improvements`
  - E2E test improvements
  
- [ ] Merge `copilot/fix-workflow-issues-apps/swarm-apps/dash-legacy-2-legacy-2board-legacy-2-legacy-2`
  - Additional apps/dash-legacy-2-legacy-2board workflow fixes

## Phase 4: Backend Features

- [ ] Merge `copilot/sub-pr-3` (PR #4)
  - Optimizes contract compilation
  
- [ ] Merge `feat/async-reputation-pg-repo`
  - Async reputation repository
  
- [ ] Merge `feature/sigill-aggregator-helper`
  - Signal aggregator utilities
  
- [ ] Run Python tests
  ```bash
  python -m pytest
  ```

## Phase 5: Frontend Features

- [ ] Merge `feature/apps/swarm-apps/dash-legacy-2-legacy-2board-legacy-2-legacy-2-e2e` (PR #3)
  - **Large changeset** - TypeScript migration
  - Adds testing infrastructure
  
- [ ] Merge `feat/frontend/ui/cli-and-banner`
  - CLI and banner improvements
  
- [ ] Merge `chore/tsx-cleanup-2`
  - TypeScript cleanup
  
- [ ] Run frontend tests
  ```bash
  cd apps/swarm-apps/dash-legacy-2-legacy-2board-legacy-2-legacy-2
  npm install
  npm run tsc --silent
  npm test
  npm run e2e:test
  ```

## Phase 6: Core Platform ⚡

- [ ] **IMPORTANT**: Rebase `feature/x3-kernel-task1` to main
  ```bash
  git checkout feature/x3-kernel-task1
  git rebase main
  # Resolve any conflicts
  git checkout integration/all-features  # or main
  ```
  
- [ ] Merge `feature/x3-kernel-task1`
  - **Large changeset** - Atomic Trade Engine pallet
  - Contains Rust/Substrate code
  
- [ ] Run Rust tests (if applicable)
  ```bash
  cargo test
  cargo build --release
  ```

## Phase 7: Production Hardening

- [ ] Merge `staging/production-hardening`
  - Production readiness improvements
  
- [ ] Final comprehensive test suite
  ```bash
  # Python
  python -m pytest
  
  # Frontend  
  cd apps/swarm-apps/dash-legacy-2-legacy-2board-legacy-2-legacy-2 && npm test
  
  # Rust (if applicable)
  cargo test
  ```

## Conflict Resolution Tracker

### Expected Conflicts

Track resolved conflicts here:

- [ ] `.github/workflows/ci-swarm.yml`
  - Conflicts between: _______________
  - Resolution: _______________
  
- [ ] `package.json` / `package-lock.json`
  - Conflicts between: _______________
  - Resolution: Run `npm install` to regenerate
  
- [ ] `tools/fund_allocations.py`
  - Conflicts between: _______________
  - Resolution: _______________
  
- [ ] `apps/swarm-apps/dash-legacy-2-legacy-2board-legacy-2-legacy-2/` files
  - Conflicts between: _______________
  - Resolution: _______________

### Actual Conflicts Encountered

Document any unexpected conflicts:

1. File: _______________
   - Branches: _______________
   - Resolution: _______________

2. File: _______________
   - Branches: _______________
   - Resolution: _______________

## Post-Merge Validation

- [ ] All CI workflows pass
  - Check GitHub Actions tab
  
- [ ] No hardcoded secrets remain
  ```bash
  grep -r "password" .github/workflows/
  grep -r "secret" .github/workflows/ | grep -v "secrets."
  ```
  
- [ ] All tests pass
  - [ ] Python: `python -m pytest`
  - [ ] Frontend: `cd apps/swarm-apps/dash-legacy-2-legacy-2board-legacy-2-legacy-2 && npm test`
  - [ ] E2E: `npm run e2e:test`
  - [ ] Rust: `cargo test` (if applicable)
  
- [ ] Application builds successfully
  - [ ] Frontend: `npm run build`
  - [ ] Backend: Python services start without errors
  - [ ] Rust: `cargo build --release` (if applicable)
  
- [ ] Database migrations work
  ```bash
  alembic upgrade head
  alembic downgrade base
  alembic upgrade head
  ```
  
- [ ] Documentation is up to date
  - [ ] README files reflect new features
  - [ ] API documentation updated (if applicable)

## Finalization

- [ ] Review git log
  ```bash
  git log --oneline --graph -20
  ```
  
- [ ] Create annotated tag for this major merge
  ```bash
  git tag -a v1.0.0-merged -m "All feature branches merged into main"
  ```
  
- [ ] Push to main (if using integration branch)
  ```bash
  git checkout main
  git merge --no-ff integration/all-features
  git push origin main
  git push origin --tags
  ```

- [ ] OR push integration branch (for review)
  ```bash
  git push origin integration/all-features
  # Create PR: integration/all-features → main
  ```

## Rollback Procedure (If Needed)

If critical issues are discovered after merge:

- [ ] Identify pre-merge commit SHA: _______________

- [ ] Create rollback branch
  ```bash
  git checkout -b rollback/pre-merge <commit-sha>
  git push origin rollback/pre-merge
  ```

- [ ] OR revert the merge
  ```bash
  git revert -m 1 <merge-commit-sha>
  git push origin main
  ```

## Cleanup (After Successful Merge)

- [ ] Delete merged local branches
  ```bash
  git branch --merged main | grep -v "main" | xargs git branch -d
  ```

- [ ] Optionally delete remote branches (⚠️ permanent)
  ```bash
  # List branches to delete first
  git branch -r --merged main
  
  # Delete individually:
  git push origin --delete <branch-name>
  ```

- [ ] Update project documentation with new features

- [ ] Notify team of completed merge

## Notes & Observations

Use this space to document any issues, learnings, or observations during the merge:

```
[Add notes here as you work through the merge]





```

## Estimated Time

- Setup & Preparation: 30 min
- Phase 1-2 (Security + DB): 1 hour
- Phase 3 (CI/CD): 45 min  
- Phase 4 (Backend): 1 hour
- Phase 5 (Frontend): 2 hours
- Phase 6 (Core Platform): 2 hours
- Phase 7 (Production): 1 hour
- Testing & Validation: 2 hours
- **Total**: 10-12 hours (can be done over multiple sessions)

---

**Good luck! 🚀**

# Workflow Fixes for x3-chain Repository

## Summary
This document describes the workflow failures identified in PRs #3, #7, and #8, and provides the necessary fixes.

## Failing Workflows

### 1. apps/swarm-apps/dash-legacy-2-legacy-2board-legacy-2-legacy-2-e2e.yml Workflow
**Run ID**: 20392288117  
**Error**: `npm ci` fails with "can only install packages with an existing package-lock.json"  
**Root Cause**: The `apps/swarm-apps/dash-legacy-2-legacy-2board-legacy-2-legacy-2/package-lock.json` file is missing from the repository  
**Fix**: Generate package-lock.json by running `npm install` in the apps/swarm-apps/dash-legacy-2-legacy-2board-legacy-2-legacy-2 directory

### 2. ci-swarm.yml Workflow  
**Run ID**: 20392288133  
**Error**: `Unable to resolve action treosh/lighthouse-ci-action@v6`  
**Root Cause**: Version v6 of the Lighthouse CI action doesn't exist; latest is v12  
**Fix**: Update line 73 in `.github/workflows/ci-swarm.yml` from `@v6` to `@v12`

## Required Changes

### File 1: `.github/workflows/ci-swarm.yml`
```yaml
# Line 73: Change from
- uses: treosh/lighthouse-ci-action@v6

# To
- uses: treosh/lighthouse-ci-action@v12
```

### File 2: `apps/swarm-apps/dash-legacy-2-legacy-2board-legacy-2-legacy-2/package-lock.json`
- Generate this file by running `npm install` in the `apps/swarm-apps/dash-legacy-2-legacy-2board-legacy-2-legacy-2/` directory
- Commit the generated file to the repository

### File 3: `.gitignore` (Recommended)
Add a .gitignore file to prevent committing unnecessary files:
```
# Node modules
node_modules/

# Build artifacts
dist/
build/

# Python
__pycache__/
*.py[cod]
*$py.class
*.so
.Python
env/
venv/
ENV/

# IDE
.vscode/
.idea/
*.swp
*.swo

# OS
.DS_Store
Thumbs.db
```

## Implementation Status

- **PR #7**: Has already implemented these fixes on the `copilot/fix-workflow-errors` branch targeting `feature/apps/swarm-apps/dash-legacy-2-legacy-2board-legacy-2-legacy-2-e2e`
- **PR #8**: Current PR, needs to determine approach for applying fixes to `main` branch
- **PR #3**: Original feature PR where workflows are failing

## Recommendation

Since PR #7 has already implemented the fixes:
1. Merge PR #7 into `feature/apps/swarm-apps/dash-legacy-2-legacy-2board-legacy-2-legacy-2-e2e` to fix the failing workflows there
2. When merging `feature/apps/swarm-apps/dash-legacy-2-legacy-2board-legacy-2-legacy-2-e2e` to `main`, the fixes will come along
3. Close PR #8 as a duplicate

OR

If workflows need to be added directly to `main`:
1. Create the workflow files on `main` with fixes already applied
2. Add the package-lock.json file
3. Add .gitignore file

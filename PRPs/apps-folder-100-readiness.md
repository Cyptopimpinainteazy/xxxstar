name: "Apps Folder 100% Readiness PRP"
description: |
  Bring all targeted apps to pass lint/typecheck/test/build gates with minimal, focused fixes.

## Goal
Drive all app-level quality gates to green for the apps in `apps/` so release readiness is consistent and measurable.

## Why
- Ensures app surfaces are shippable, not just core chain crates.
- Removes recurring CI and local dev failures.
- Converts vague “apps quality” into objective pass/fail gates.

## What
Fix quality/build/test/lint issues in:
- `apps/dex`
- `apps/wallet`
- `apps/x3-desktop`
- `apps/x3-intelligence`
- `apps/validators`
- `apps/inferstructor-dashboard`

### Success Criteria
- [ ] Each target app has passing `lint` (if script exists)
- [ ] Each target app has passing `typecheck`/`type-check` (if script exists)
- [ ] Each target app has passing `test` (if script exists, or explicitly pass-with-no-tests policy)
- [ ] Each target app has passing `build` (if script exists)
- [ ] Updated baseline report shows all gates green

## All Needed Context

### Documentation & References
```yaml
- file: .artifacts/apps-quality-baseline.txt
  why: Current pass/fail baseline and concrete error output

- file: apps/dex/package.json
  why: Verify lint/build script definitions and invocation behavior

- file: apps/wallet/package.json
  why: Jest + Next script behavior and test failure policy

- file: apps/x3-desktop/package.json
  why: lint/typecheck/test/build command surface

- file: apps/x3-desktop/src-tauri/.cargo/config.toml
  why: Desktop build/toolchain constraints that can affect build gates

- doc: https://nextjs.org/docs/app/building-your-application/configuring/eslint
  section: Next.js lint configuration and execution
  critical: Next projects should lint from app root with valid next config

- doc: https://eslint.org/docs/latest/use/configure/
  section: Configuration file resolution
  critical: ESLint failure can be pure config discovery issue

- doc: https://www.typescriptlang.org/tsconfig
  section: strictness and module/type resolution
  critical: typecheck failures often come from missing types / wrong imports
```

### Current Baseline Snapshot (from `.artifacts/apps-quality-baseline.txt`)
```text
dex:
- lint FAIL (next lint invocation/config issue)
- build FAIL (missing app/pages directory)

wallet:
- lint FAIL (next lint invocation/config issue)
- test FAIL (no tests found, exits 1)
- build FAIL (TS error: unused React import)

x3-desktop:
- lint FAIL (no ESLint config found)
- typecheck FAIL (numerous TS errors, tauri api/type mismatches)
- test FAIL
- build FAIL

x3-intelligence:
- lint FAIL
- test FAIL
- build FAIL

validators:
- lint FAIL
- type-check PASS
- build FAIL

inferstructor-dashboard:
- lint FAIL
- build PASS
```

### Desired Codebase Tree (new/updated files expected)
```bash
apps/*/package.json                        # script corrections where needed
apps/*/next.config.*                       # next lint/build root correctness
apps/*/eslint.config.* or .eslintrc.*      # lint config normalization
apps/*/tsconfig*.json                      # typecheck consistency
apps/wallet/src/components/...             # direct TS build fixes
apps/x3-desktop/src/components/...         # direct TS fixes
.artifacts/apps-quality-baseline-after.txt # post-fix baseline
```

### Known Gotchas
```text
- Running npm scripts from root with --prefix can produce misleading behavior for some Next scripts.
- Next.js warns about multiple lockfiles and root inference; set explicit root when needed.
- Some apps intentionally have no tests; if kept that way, set test script policy explicitly.
- x3-desktop currently has many TS errors that are not lint-only; fix in batches to avoid regressions.
```

## Implementation Blueprint

### Task 1: Normalize command execution + baseline harness
- Create a deterministic script to run each app gate from app directory (`cd apps/<name> && npm run <cmd>`)
- Capture output into `.artifacts/apps-quality-baseline-after.txt`
- Ensure script exits non-zero only after full matrix collected

### Task 2: Fix Next app lint/build invocation issues (`dex`, `wallet`)
- Verify `package.json` scripts call proper commands from app root
- Ensure Next project has valid `app/` or `pages/` structure and correct config pathing
- Replace deprecated `images.domains` with `images.remotePatterns` where practical
- Address lockfile-root warning with explicit `turbopack.root` if needed

### Task 3: Wallet test and TS build hygiene
- Decide policy:
  - add minimal smoke tests, OR
  - use `jest --passWithNoTests` if intentionally testless
- Fix current build failure in `src/components/crm/DorksSearchPanel.tsx` (`React` unused import)

### Task 4: x3-desktop lint/typecheck/build stabilization
- Add ESLint config file so `npm run lint` resolves config
- Resolve high-signal type errors first:
  - typo/import errors (`usEffect` -> `useEffect`)
  - missing Tauri API module imports/type package mismatch
  - invalid JSX intrinsic usage (`<value>`)
  - duplicate object key definitions
- Re-run `typecheck`, then `test`, then `build`

### Task 5: x3-intelligence, validators, inferstructor-dashboard targeted fixes
- Address lint failures first (config or code)
- Fix build blockers only (minimal diff)
- Preserve existing app behavior and routes

### Task 6: Final validation and readiness report
- Re-run full matrix and ensure all gates pass
- Write concise delta summary to progress log
- If any app remains red, document exact blocker and owner-action

## Validation Loop

### Level 1: App-by-app gates
```bash
cd apps/dex && npm run lint && npm run build
cd apps/wallet && npm run lint && npm run test && npm run build
cd apps/x3-desktop && npm run lint && npm run typecheck && npm run test && npm run build
cd apps/x3-intelligence && npm run lint && npm run test && npm run build
cd apps/validators && npm run lint && npm run type-check && npm run build
cd apps/inferstructor-dashboard && npm run lint && npm run build
```

### Level 2: Full matrix
```bash
bash /tmp/apps_quality_baseline_cd.sh
# Expected: all RESULT lines are PASS
```

### Level 3: Regression safety
```bash
cd /home/lojak/Desktop/x3-chain-master
npm run -s test --workspaces --if-present
```

## Final Validation Checklist
- [ ] All targeted app gates pass
- [ ] No ad-hoc environment-only hacks introduced
- [ ] Config changes documented in relevant app README/package comments
- [ ] Post-fix baseline file present and green

## Anti-Patterns to Avoid
- ❌ Don’t silence TypeScript globally to force green builds
- ❌ Don’t disable lint rules across the board
- ❌ Don’t “fix” tests by removing them
- ❌ Don’t change app UX/feature behavior outside readiness scope

## Confidence Score
8/10

Rationale: failures are concrete and reproducible; most are config + focused TS cleanup. Main risk is volume of `x3-desktop` type errors, but they are tractable in batches.

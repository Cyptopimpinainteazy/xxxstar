# Workflow Hook: Pre-Merge

Run before any merge to main.

This hook ensures the code is safe for others to build on.

## Required Output

```md
## Pre-Merge Checklist

### CI Matrix

All CI checks must pass:

| Check | Status | Link |
|-------|--------|------|
| Build (debug) | PASS / FAIL | <link> |
| Build (release) | PASS / FAIL | <link> |
| Tests (unit) | PASS / FAIL | <link> |
| Tests (integration) | PASS / FAIL | <link> |
| Tests (e2e) | PASS / FAIL | <link> |
| Clippy | PASS / FAIL | <link> |
| Format | PASS / FAIL | <link> |
| Docs | PASS / FAIL | <link> |

All green? YES / NO

If NO, wait for CI to pass before merging.

### Migration Compatibility

Storage migration needed?
- YES / NO

If YES:

- Old schema: <describe>
- New schema: <describe>
- Migration path: <describe>
- Rollback path: <describe>
- Data loss risk: NONE / LOW / MEDIUM / HIGH
- Tested on prod-like data? YES / NO

Migration safe? YES / NO / UNKNOWN

If UNKNOWN, test before merge.

### Parallel Merge Gate

Files touched:
- <file 1>
- <file 2>

Likely merge conflicts:
- <conflict 1>
- <conflict 2>

Shared interfaces changed?
- YES / NO

If YES:
- Changed interface: <name>
- Other agents affected: <names>
- Coordination: <required coordination>

Safe merge order:
1. <this PR>
2. <other PR>

Post-merge validation:
```txt
cargo test --workspace
cargo test --test integration
```

### Spec Drift Gate

Spec/source checked:
- `docs/FEATURE.md`
- `RC_PLAN.md`
- `DESIGN.md`

Implementation matches spec?
- YES / PARTIAL / NO

Spec outdated?
- YES / NO

Code outdated?
- YES / NO

If drift:
- What changed: <explain>
- Why it changed: <explain>
- Was it necessary: YES / NO
- Did it remain inside scope: YES / NO

Required doc updates:
- <update 1>

Required code updates:
- <update 1>

All docs updated? YES / NO

If NO, update before merge.

### Release Notes Draft

Changes summary:
- New features: <list>
- Bug fixes: <list>
- Breaking changes: <list>
- Deprecations: <list>
- Known issues: <list>

Release notes ready for release? YES / NO

### Mainnet Safety Gate

Mainnet-sensitive?
- YES / NO

If YES, classify:
- Risk area: <consensus / bridge / validator / wallet / DEX / treasury / runtime / RPC / config>
- Risk level: <LOW / MEDIUM / HIGH / CRITICAL>
- Allowed for mainnet now? <LOCAL ONLY / DEVNET / TESTNET / AUDIT / MAINNET CANDIDATE / MAINNET READY>

If not MAINNET READY, document why:
- <blocker 1>
- <blocker 2>

### Handoff Pack

**Current state:**
- What was completed
- What remains
- Known limitations

**Important files:**
- `src/file.rs` — <purpose>
- `tests/test.rs` — <purpose>

**Commands to run next:**
```txt
cargo test --workspace
cargo test --test integration
<next task command>
```

**Known blockers:**
- <blocker 1>
- <blocker 2>

**Next recommended task:**
- <task 1>
- <task 2>

**For reviewers:**
- Pay attention to: <area>
- Test this path: <command>
- Known limitations: <list>
```

## Approval Checklist

Before merging:

- [ ] All CI checks pass
- [ ] No merge conflicts
- [ ] Migration is safe (if applicable)
- [ ] Spec drift resolved
- [ ] Release notes drafted
- [ ] Mainnet safety classified
- [ ] Handoff pack is complete
- [ ] No P0/P1 blockers remain

If any box is unchecked, do not merge. Fix the issue first.

---

**Rule:** Do not merge if you would not want another team to pull this code and build on it immediately. If unsure, wait for review.

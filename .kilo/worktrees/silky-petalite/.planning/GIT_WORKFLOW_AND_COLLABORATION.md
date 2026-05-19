# 🔄 X3 V0.4 IMPLEMENTATION — GIT WORKFLOW & COLLABORATION

**Version:** 1.0  
**Effective:** Week 1 (Sprint 0)  
**Duration:** 20 weeks (until testnet launch)

---

## 📋 BRANCH STRATEGY

### Main Branches

| Branch | Purpose | Protection | Merge Gate |
|--------|---------|-----------|-----------|
| `main` | Production-ready testnet | 🔴 Protected | 2 approvals + all tests green |
| `develop` | Integration baseline | 🔴 Protected | 1 approval + all tests green |
| `sprint/*` | Active sprint work | 🟡 Suggested | Team lead approval |

### Feature Branches

**Naming Convention:** `{sprint}/{module}/{feature}`

**Examples:**
- `sprint-1/packet-standard/replay-protection`
- `sprint-2/x3-ixl/instruction-set`
- `sprint-5/gateway/base-adapter`
- `sprint-0/readiness/invariant-audit`

**Naming Rules:**
- [ ] Use lowercase, hyphens (not underscores)
- [ ] Include sprint number
- [ ] Include module name
- [ ] Descriptive feature suffix

### Release Branches

**Naming Convention:** `release/v0.4.0-{sprint}.{week}`

**Examples:**
- `release/v0.4.0-s1.2` (Sprint 1, Week 2)
- `release/v0.4.0-s5.14` (Sprint 5, Week 14)

---

## 🔀 WORKFLOW

### Phase 1: Create Feature Branch

```bash
# Update develop
git fetch origin
git checkout develop
git pull origin develop

# Create feature branch
git checkout -b sprint-1/packet-standard/replay-protection

# Confirm branch
git branch -a
```

### Phase 2: Make Changes

```bash
# Stage commits
git add crates/x3-packet-standard/src/replay.rs
git commit -m "feat: implement replay protection map for packet standard

- Add ReplayProtectionMap with domain/sender/sequence tracking
- Implement submit_packet() with idempotency guarantee
- Add prune_old_entries() for storage management
- Tests: 500 replay attempts rejected

Fixes #123 (issue number if applicable)
"

# Small, logical commits preferred
# Each commit should be independently buildable
```

### Phase 3: Local Testing

```bash
# Run tests before pushing
cargo test -p x3-packet-standard

# Check formatting
cargo fmt --all

# Run clippy
cargo clippy -p x3-packet-standard -- -D warnings

# Build full workspace
cargo build --release
```

### Phase 4: Push & Create PR

```bash
# Push feature branch
git push -u origin sprint-1/packet-standard/replay-protection

# Create Pull Request
# Use template (see below)
```

### Phase 5: PR Review & Merge

```bash
# After 2 approvals + tests passing:
git checkout develop
git pull origin develop
git merge --no-ff sprint-1/packet-standard/replay-protection
git push origin develop

# Delete feature branch
git push origin --delete sprint-1/packet-standard/replay-protection
git branch -d sprint-1/packet-standard/replay-protection
```

---

## 📝 PULL REQUEST TEMPLATE

```markdown
## 🎯 Purpose
Brief description of what this PR accomplishes.

## 📋 Checklist
- [ ] Code follows style guide
- [ ] All tests passing (`cargo test`)
- [ ] Documentation updated
- [ ] Benchmark results included (if applicable)
- [ ] No `TODO` comments left
- [ ] Ready for code review

## 🧪 Testing
Describe testing approach:
- [ ] Unit tests added: 5 test cases
- [ ] Integration tests added: 3 scenarios
- [ ] Fuzz testing: 1,000 random inputs
- [ ] Performance: Baseline vs. optimized

## 🔍 Review Notes
- Replay protection guarantee: idempotent re-submission
- Storage pruning: O(1) amortized
- Edge case: Same sequence, different sender → accepted

## 📊 Files Changed
- [ ] `crates/x3-packet-standard/src/replay.rs` (150 LOC)
- [ ] `tests/packet_standard_tests.rs` (200 LOC)

## 🎯 Related Issues
Fixes #123, Relates to #124

## 🔗 Sprint
- Sprint: 1
- Module: Packet Standard
- Task: Replay Protection
```

---

## 🏷️ COMMIT MESSAGE CONVENTION

**Format:** `{type}({scope}): {subject}`

**Types:**
- `feat:` — New feature
- `fix:` — Bug fix
- `refactor:` — Code reorganization
- `test:` — Test additions/improvements
- `docs:` — Documentation
- `perf:` — Performance improvement
- `chore:` — Dependency update, config change

**Scope:** Module/crate name (e.g., `packet-standard`, `x3-ixl`, `gateway`)

**Subject:** 
- Imperative mood ("add" not "adds")
- No period at end
- Max 50 characters

**Examples:**
```
feat(packet-standard): add replay protection map
fix(x3-ixl): resolve instruction ordering bug
test(gateway): add witness quorum tests
docs(liquidity-core): document launchpad flow
perf(parallel-executor): optimize access list builder
```

---

## 🔐 PROTECTION RULES

### For `main` Branch
```
- Require 2 approvals from CODEOWNERS
- All status checks must pass
- Require branches to be up to date
- Dismiss stale PR approvals
- Require code review before merge
- Require passing tests before merge
```

### For `develop` Branch
```
- Require 1 approval from CODEOWNERS
- All status checks must pass
- Require branches to be up to date
- Auto-merge on approval (optional, for speed)
```

### For `sprint/*` Branches
```
- Require sprint lead approval
- All tests must pass
- Manual merge (no auto-merge)
```

---

## 👥 CODE OWNERS

**File:** `.github/CODEOWNERS`

```
# Kernel & Foundation
pallets/x3-kernel/ @kernel-team @security-team
crates/x3-asset-kernel-types/ @kernel-team
crates/x3-readiness-report/ @kernel-team

# Packet Standard
crates/x3-packet-standard/ @protocol-team @security-team

# X3-IXL
crates/x3-ixl/ @vm-team @security-team

# LiquidityCore
crates/x3-liquidity-core/ @dex-team

# Gateway
crates/x3-external-liquidity-gateway/ @gateway-team

# Services
crates/x3-integrated-services/ @services-team

# Parallel Executor
crates/x3-parallel-executor/ @performance-team

# AppZone Factory
crates/x3-appzone-factory/ @developer-tools-team

# CI/CD
.github/ @devops-team
Dockerfile* @devops-team
```

---

## ✅ DEFINITION OF DONE (per Feature)

A feature is "done" when:

- [ ] Code written (all acceptance criteria met)
- [ ] Unit tests pass (>90% coverage)
- [ ] Integration tests pass
- [ ] Code reviewed (2 approvals for main, 1 for develop)
- [ ] Documentation updated
- [ ] Performance benchmarked (if applicable)
- [ ] Security audit passed (if applicable)
- [ ] Merged to `develop`
- [ ] No regressions in existing tests
- [ ] Deployment tested (if applicable)

---

## 🚀 RELEASE PROCESS

### Weekly Release Candidate (RC)

**Every Friday EOD:**

```bash
# Create release branch
git checkout develop
git pull origin develop
git checkout -b release/v0.4.0-s{sprint}.{week}

# Bump version in Cargo.toml files
# (major.minor.patch = 0.4.{sprint})

# Update CHANGELOG.md
git add .
git commit -m "chore: release v0.4.0-s{sprint}.{week} RC"
git push origin release/v0.4.0-s{sprint}.{week}

# Open PR against main
# Title: "chore(release): v0.4.0-s{sprint}.{week}"
```

**Review & Merge to Main:**
- [ ] All tests green
- [ ] Benchmarks stable
- [ ] Security audit passed
- [ ] 2 approvals from release team
- [ ] Merge to main
- [ ] Tag: `git tag v0.4.0-s{sprint}.{week}`

### Emergency Hotfix

**If critical bug found post-merge:**

```bash
# Create hotfix branch from main
git checkout main
git checkout -b hotfix/critical-bug-xyz

# Fix the bug
# Commit: "fix: critical bug in X"

# PR against main → get 2 approvals → merge
# Also cherry-pick to develop
git checkout develop
git cherry-pick {commit-hash}
```

---

## 📊 GITHUB ACTIONS CI/CD

**File:** `.github/workflows/build.yml`

```yaml
name: Build & Test

on:
  pull_request:
    branches: [main, develop, sprint-*]
  push:
    branches: [main, develop, sprint-*]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: 1.89.0
          override: true
      
      - name: cargo fmt
        run: cargo fmt --all -- --check
      
      - name: cargo clippy
        run: cargo clippy --all --all-targets -- -D warnings
      
      - name: cargo test
        run: cargo test --all
      
      - name: cargo test (release)
        run: cargo test --all --release
      
      - name: cargo build (release)
        run: cargo build --all --release
      
      - name: Code coverage
        run: cargo tarpaulin --out Html --output-dir coverage/
      
      - name: Upload coverage
        uses: codecov/codecov-action@v3
        with:
          files: ./coverage/index.html
```

---

## 🔔 REVIEW EXPECTATIONS

### For Reviewers

**Turnaround:** 24-48 hours target

**Questions to ask:**
- [ ] Does this implement the spec/issue correctly?
- [ ] Are there obvious bugs or security issues?
- [ ] Is the code testable?
- [ ] Does it follow our patterns and conventions?
- [ ] Is performance acceptable?
- [ ] Are there any missing edge cases?

**Approval means:**
- "I've reviewed this and it's good to merge."
- You're putting your name behind it.

### For Authors

**Before requesting review:**
- [ ] All tests passing locally
- [ ] Self-reviewed your own code
- [ ] No formatting issues
- [ ] Commit messages are clear

**During review:**
- [ ] Respond to all comments within 24 hours
- [ ] Re-request review after making changes
- [ ] Don't force-push after review starts (creates confusion)

---

## 📈 SPRINT BOARD INTEGRATION

**GitHub Issues → Sprint Board:**

Each PR must have:
- [ ] Sprint label (sprint-0, sprint-1, etc.)
- [ ] Module label (kernel, packet-standard, x3-ixl, etc.)
- [ ] Task linked to issue (if applicable)
- [ ] Assignee (person responsible)

**Example Issue Template:**

```markdown
## 🎯 Task: Implement Replay Protection

**Sprint:** Sprint 1  
**Module:** Packet Standard  
**Effort:** 4 days  
**Assignee:** @alice

### Description
Implement replay protection map for x3-packet-standard.

### Acceptance Criteria
- [ ] ReplayProtectionMap struct created
- [ ] 500 replay attempts rejected in fuzz test
- [ ] O(1) amortized storage ops
- [ ] Pruning removes expired entries
- [ ] 95%+ test coverage

### Related PRs
- [ ] (none yet)

### Definition of Done
- [ ] Code reviewed (2 approvals)
- [ ] All tests passing
- [ ] Merged to develop
```

---

## 🆘 CONFLICT RESOLUTION

### If Merge Conflicts Arise

```bash
# Update feature branch with latest develop
git fetch origin
git merge origin/develop

# Resolve conflicts
git add .
git commit -m "merge: resolve conflicts with develop"

# Push and request re-review
git push origin sprint-1/packet-standard/replay-protection
```

### Prevention
- Pull develop frequently
- Keep branches short-lived (max 3 days)
- Communicate with team on shared areas

---

## 📊 METRICS TO TRACK

**Per Sprint:**
- [ ] Lines of code added
- [ ] Test coverage %
- [ ] Build time (target: <5 min)
- [ ] Average PR review time (target: <24h)
- [ ] Bugs found in QA (target: <5 per sprint)

**Per Module:**
- [ ] Test pass rate (target: 100%)
- [ ] Code coverage (target: >90%)
- [ ] Performance regression (target: 0%)
- [ ] Security findings (target: 0 critical)

---

## 🎓 BEST PRACTICES

### ✅ DO:
- Write small, focused PRs (one feature per PR)
- Commit frequently with clear messages
- Pull develop before pushing
- Use feature branches for all work
- Request review early (even if "WIP")
- Add tests before code (TDD)
- Update docs as you go

### ❌ DON'T:
- Force-push after review starts
- Commit directly to develop/main
- Mix unrelated changes in one PR
- Leave `TODO` comments
- Skip tests to merge faster
- Rewrite history on shared branches
- Merge your own PRs without review

---

## 📞 COMMUNICATION

### Slack Channels

| Channel | Purpose |
|---------|---------|
| `#x3-dev` | General development discussion |
| `#sprint-1` | Sprint 1 specific work |
| `#pr-reviews` | PR review notifications |
| `#blockers` | Critical issues |
| `#releases` | Release coordination |

### Daily Standup
- **Time:** 10 AM UTC (adjustable)
- **Format:** 15 minutes
  - What I did yesterday
  - What I'm doing today
  - Any blockers

### Sprint Planning
- **Time:** Monday 9 AM UTC
- **Duration:** 2 hours
- **Attendees:** Full team

### Sprint Review
- **Time:** Friday 4 PM UTC
- **Duration:** 1 hour
- **Format:** Demo completed features, metrics review

---

## 🔐 SECURITY & COMPLIANCE

### Code Review Checklist

Before approving ANY PR, verify:
- [ ] No secrets in code (API keys, private keys, etc.)
- [ ] No unsafe code blocks (document if needed)
- [ ] Input validation present
- [ ] Error handling complete
- [ ] No `unwrap()` in production code
- [ ] Crypto code reviewed by security team
- [ ] No new dependencies without approval

### Dependency Management

```bash
# Before adding new dependency:
git add Cargo.toml
cargo tree --all-features  # Verify tree
cargo audit                 # Check vulnerabilities
# Get approval before committing
```

---

## ✨ FINAL CHECKLIST: READY FOR WEEK 1

Before Sprint 0 starts, ensure:

- [ ] `.github/CODEOWNERS` file created
- [ ] Branch protection rules configured
- [ ] GitHub Actions workflows enabled
- [ ] Team members added to appropriate labels
- [ ] Slack channels created for each sprint
- [ ] Sprint board/project set up
- [ ] Release process documented
- [ ] All team members trained on workflow


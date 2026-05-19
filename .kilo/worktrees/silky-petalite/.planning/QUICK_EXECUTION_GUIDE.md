# ⚡ QUICK START: V0.4 IMPLEMENTATION EXECUTION GUIDE

**For:** Team leads, project managers, sprint coordinators  
**When:** Start of each sprint  
**Update Frequency:** Weekly

---

## 🎯 THIS WEEK: SPRINT 0 (Foundation Audit)

**Week:** 1  
**Start Date:** Monday, April 28, 2025  
**End Date:** Friday, May 2, 2025  
**Team:** 2 engineers  
**Deliverable:** Kernel audit complete, readiness-report crate scaffolded

### Daily Targets

**Monday (Apr 28):**
- [ ] Team meeting: Sprint 0 kickoff
- [ ] Setup: Branch `sprint-0/foundation/kernel-audit` created
- [ ] Task: Start Phase 0.1 (Canonical Supply Invariant Audit)
- [ ] Slack: Daily standup @10 AM UTC

**Tuesday (Apr 29):**
- [ ] Phase 0.1 continued: Fuzz test harness added
- [ ] Target: 1,000 random operations tested

**Wednesday (Apr 30):**
- [ ] Phase 0.1 complete
- [ ] Start Phase 0.2 (Emergency Halt Path)
- [ ] Target: Emergency halt tested + documented

**Thursday (May 1):**
- [ ] Phase 0.3 & 0.4 in parallel (Mint/Burn audit + Balance reconciliation)
- [ ] Target: Both complete

**Friday (May 2):**
- [ ] Phase 0.5: Readiness crate scaffolding
- [ ] Sprint Review: Demo readiness collector
- [ ] Merge PRs to `develop`
- [ ] Tag: `v0.4.0-s0.1`

---

## 📊 WEEKLY CHECKLIST

- [ ] **Before Monday standup:**
  - Pull latest `develop`
  - Review sprint board
  - Note blockers from last week

- [ ] **Daily (10 AM UTC):**
  - What I did yesterday
  - What I'm doing today
  - Blockers?

- [ ] **Friday EOD:**
  - All PRs merged
  - Tests passing
  - Document completion
  - Update MASTER_STATUS.md

---

## 🚀 SPRINT KICKOFF TEMPLATE

**Use this for Sprint 0, 1, 2, ... 8:**

```
📍 SPRINT {N} KICKOFF: {Module Name}
🎯 Goal: {One sentence description}
📅 Duration: Weeks {X}-{Y}
👥 Team Size: {N} engineers
📦 Deliverable: {What ships at end}

DEPENDENCIES:
- [ ] {Previous sprint} complete
- [ ] {External dep} ready
- [ ] {Tooling} installed

TASKS:
- [ ] Phase {N}.1: {Task name} ({effort})
- [ ] Phase {N}.2: {Task name} ({effort})
...

DEFINITION OF DONE:
- [ ] All tests passing
- [ ] Code reviewed (2 approvals)
- [ ] Merged to develop
- [ ] Documentation updated
- [ ] Benchmarks recorded

SUCCESS CRITERIA:
- [ ] {Metric 1} ≥ {Target}
- [ ] {Metric 2} ≥ {Target}
- [ ] {Metric 3} ≥ {Target}

BLOCKERS:
(None identified; update as needed)

NEXT SPRINT:
Sprint {N+1}: {Module}
```

---

## 🔄 DAILY WORKFLOW (COPY & PASTE)

**Morning (8 AM UTC):**
```bash
# Update local branches
git fetch origin
git checkout develop
git pull origin develop

# Check sprint board
# (Open GitHub Projects tab)

# Review PRs awaiting review
# (Open PRs tab, filter "needs review")
```

**During day:**
```bash
# Create feature branch (if starting new task)
git checkout -b sprint-{N}/{module}/{feature-name}

# Make commits
git add .
git commit -m "type(scope): description"

# Run tests before pushing
cargo test -p {crate-name}
cargo fmt --all
cargo clippy --all -- -D warnings

# Push when ready
git push -u origin sprint-{N}/{module}/{feature-name}
```

**Afternoon (4 PM UTC):**
```bash
# Review others' PRs (quick check)
# Leave comment with suggestions

# Update Slack status
# "On track / Blocked / Complete"
```

**Friday (4 PM UTC):**
```bash
# Merge completed PRs
git checkout develop
git pull origin develop
git merge --no-ff sprint-{N}/{module}/{feature-name}
git push origin develop

# Delete feature branch
git push origin --delete sprint-{N}/{module}/{feature-name}

# Create release candidate
git checkout -b release/v0.4.0-s{N}.{week}
# (bump version in Cargo.toml)
git push origin release/v0.4.0-s{N}.{week}

# Open PR: "chore(release): v0.4.0-s{N}.{week}"
```

---

## 🎯 KEY METRICS TO TRACK

### Per Sprint (Report Friday)

```markdown
## Sprint {N} Completion Report

### Metrics
- Tests passing: 100% ✅
- Code coverage: 92% (target: >90%)
- Build time: 4m 32s (target: <5 min)
- Avg PR review time: 18 hours (target: <24 hours)
- Critical bugs found in QA: 0 ✅
- LOC added: 2,150

### Module Health
- Kernel: ✅ 100% test pass
- Gateway: ✅ Adapters 1-2 complete
- Services: 🟡 Oracle 50% done

### Blockers
- None

### Next Week
Sprint {N+1} starts Monday
```

---

## ⚠️ BLOCKER RESOLUTION

**If you hit a blocker:**

1. **Post immediately to Slack #blockers**
   - What task are you on?
   - What went wrong?
   - What do you need?

2. **Owner takes action within 1 hour**
   - Escalate to tech lead
   - Unblock or adjust task
   - Communicate back to team

3. **Track in Sprint Board**
   - Mark issue "blocked"
   - Add comment: "Blocked by {reason}"
   - Assign to unblocking person

4. **Avoid**: Working around the blocker (it'll haunt you later)

---

## 🔍 CODE REVIEW CHECKLIST (5 min per PR)

When reviewing a PR:

- [ ] **Correctness:** Does it do what the issue/PR description says?
- [ ] **Tests:** Are there tests? Do they make sense?
- [ ] **Security:** Any obvious security issues?
- [ ] **Style:** Follows conventions? Formatted?
- [ ] **Performance:** Any new dependencies or slow loops?
- [ ] **Docs:** Is it documented?

**Approval message:**
> Looks good! Tests pass, code is clean, ready to merge. ✅

**Request changes:**
> Small issues to fix: [1) ..., 2) ..., 3) ...]. Let's chat about [specific concern].

---

## 📈 PROGRESS VISUALIZATION

**Copy this to MASTER_STATUS.md, update weekly:**

```
# X3 V0.4 Implementation Progress

## Timeline (20 weeks)
```
Sprint 0: ████░░░░░░░░░░░░░░ Week 1      (5%)
Sprint 1: ░░░░████░░░░░░░░░░░ Weeks 2-3  (0%)
Sprint 2: ░░░░░░░░████░░░░░░░ Weeks 4-6  (0%)
Sprint 3: ░░░░░░░░░░░░████░░░ Weeks 7-8  (0%)
Sprint 4: ░░░░░░░░░░░░░░░░░░░ Weeks 9-10 (0%)
Sprint 5: ░░░░░░░░░░░░░░░░░░░ Weeks 11-14(0%)
Sprint 6: ░░░░░░░░░░░░░░░░░░░ Weeks 15-16(0%)
Sprint 7: ░░░░░░░░░░░░░░░░░░░ Weeks 17-19(0%)
Sprint 8: ░░░░░░░░░░░░░░░░░░░ Week 20    (0%)
```

## Module Status
| Module | Status | Tests | Coverage | Notes |
|--------|--------|-------|----------|-------|
| Kernel | 🟢 READY | 65/65 | 100% | No changes needed |
| Packet Standard | ⏳ IN PROGRESS | 0/50 | 0% | Sprint 1 |
| X3-IXL | ⏳ PLANNED | 0/100 | 0% | Sprint 2 |
| LiquidityCore | ⏳ PLANNED | 48/48 | 92% | Needs refactor |
| Universal Contracts | ⏳ PLANNED | 0/30 | 0% | Sprint 4 |
| Gateway | ⏳ PLANNED | 0/100 | 0% | Sprint 5 |
| Services | ⏳ PLANNED | 0/50 | 0% | Sprint 6 |
| Parallel Executor | ⏳ PLANNED | 0/80 | 0% | Sprint 7 |

## Blockers
None currently identified.

## Last Updated: Friday, May 2, 2025
```

---

## 🎬 RUNNING YOUR FIRST SPRINT (0)

### Pre-Week (Friday before)
```bash
# Create sprint branch
git checkout -b sprint-0/foundation/kernel-audit

# Create phase task files
mkdir -p tasks/sprint-0
touch tasks/sprint-0/{phase-1,phase-2,phase-3,phase-4,phase-5}.md

# Each phase file:
# - Acceptance criteria
# - Test cases
# - Deliverable
# - Estimated hours

# Push to tracking branch (optional)
git add tasks/
git commit -m "chore(sprint-0): add phase task definitions"
git push origin sprint-0/foundation/kernel-audit
```

### Monday Morning
```bash
# Team sync (all hands 10 AM UTC)
# Review sprint plan
# Assign tasks
# Confirm blockers

# Each engineer:
git checkout sprint-0/foundation/kernel-audit
# (or create personal branch off sprint-0 if working independently)

# Start work
# Run tests frequently: cargo test -p x3-kernel
```

### Daily (3 PM UTC Check-in)
```bash
# Quick sync: 15 min
# What's done, what's stuck?
# Move blockers to Slack #blockers

# Each engineer:
# - Commit progress
# - Push work-in-progress
# - Request early feedback if unsure
```

### Friday End-of-Sprint
```bash
# Final PR reviews (by 3 PM UTC)
# All tests must pass

# Merge to develop (by 4 PM UTC)
git checkout develop && git pull
git merge --no-ff sprint-0/foundation/kernel-audit
git push origin develop

# Create release candidate
git checkout -b release/v0.4.0-s0.1
# Update version: 0.4.0 in Cargo.toml files
git commit -m "chore: bump to v0.4.0-s0.1"
git push origin release/v0.4.0-s0.1

# Open PR, get 2 approvals, merge to main
# Tag: git tag v0.4.0-s0.1
# git push origin v0.4.0-s0.1

# Demo in sprint review (4 PM UTC)
# Show readiness collector working
# Discuss metrics, learnings
```

---

## 🛠️ TOOLS YOU'LL NEED

### Development
- [ ] Rust 1.89.0 (`rustup update`)
- [ ] VS Code + Rust Analyzer
- [ ] Git CLI

### Testing
- [ ] `cargo test`
- [ ] `cargo tarpaulin` (code coverage)
- [ ] `cargo bench` (benchmarks)

### CI/CD
- [ ] GitHub Actions (auto-run tests)
- [ ] GitHub Projects (sprint board)
- [ ] GitHub Releases (version tags)

### Communication
- [ ] Slack (daily standups, blockers)
- [ ] GitHub Discussions (async questions)
- [ ] Figma (architecture diagrams)

### Documentation
- [ ] `.md` files in `.planning/`
- [ ] `docs/` folder in codebase
- [ ] `CHANGELOG.md` (weekly updates)

---

## 🎓 QUICK REFERENCE: COMMON COMMANDS

```bash
# Setup
git clone https://github.com/Cyptopimpinainteazy/x3-atomic-star.git
cd x3-atomic-star
git checkout develop

# Branch work
git checkout -b sprint-{N}/{module}/{feature}
git push -u origin sprint-{N}/{module}/{feature}

# Local testing (before pushing)
cargo test -p {crate}
cargo fmt --all
cargo clippy --all -- -D warnings

# Commit & push
git add .
git commit -m "type(scope): description"
git push origin sprint-{N}/{module}/{feature}

# Merge to develop (after PR approval)
git checkout develop
git pull origin develop
git merge --no-ff sprint-{N}/{module}/{feature}
git push origin develop

# Release candidate
git checkout -b release/v0.4.0-s{N}.{week}
# Update Cargo.toml version
git commit -am "chore: v0.4.0-s{N}.{week}"
git push origin release/v0.4.0-s{N}.{week}
# Open PR to main

# Final merge to main
git checkout main
git pull origin main
git merge --no-ff release/v0.4.0-s{N}.{week}
git push origin main
git tag v0.4.0-s{N}.{week}
git push origin v0.4.0-s{N}.{week}
```

---

## 📅 SPRINT SCHEDULE (20 weeks)

| Sprint | Module | Duration | Start | End | 
|--------|--------|----------|-------|-----|
| 0 | Foundation Audit | 1 week | Apr 28 | May 2 |
| 1 | Packet Standard | 2 weeks | May 5 | May 16 |
| 2 | X3-IXL | 3 weeks | May 19 | Jun 6 |
| 3 | LiquidityCore | 2 weeks | Jun 9 | Jun 20 |
| 4 | Universal Contracts | 2 weeks | Jun 23 | Jul 4 |
| 5 | Gateway (Parallel OK) | 4 weeks | Jul 7 | Aug 1 |
| 6 | Services | 2 weeks | Aug 4 | Aug 15 |
| 7 | Parallel Executor | 3 weeks | Aug 18 | Sep 5 |
| 8 | Testnet Launch | 1 week | Sep 8 | Sep 12 |

**Public Testnet Live:** September 15, 2025 ✅

---

## ✅ SIGN-OFF CHECKLIST: SPRINT COMPLETE

Use this before marking sprint done:

```markdown
## Sprint {N} Sign-Off

### Code Quality
- [ ] All tests passing (100%)
- [ ] Code coverage >90%
- [ ] No security findings
- [ ] No performance regressions

### Team
- [ ] All team members debriefed
- [ ] Lessons learned documented
- [ ] Blockers logged (for future)
- [ ] Celebration time! 🎉

### Documentation
- [ ] README updated
- [ ] Changelog updated
- [ ] Architecture docs current
- [ ] Runbooks updated

### Readiness for Next Sprint
- [ ] Dependencies clear
- [ ] Backlog groomed
- [ ] Tasks estimated
- [ ] Team ready

### Approval
- [ ] Tech Lead: _________________ Date: _____
- [ ] Product Lead: ______________ Date: _____

Sprint {N} is COMPLETE ✅
```

---

## 🚨 IF THINGS GO WRONG

### Build Breaks
```bash
# 1. Identify the issue
cargo build 2>&1 | head -20

# 2. Fix it (or revert)
git revert {commit-hash}

# 3. Notify team
# Post in #blockers immediately

# 4. Prevent: Run tests before push!
```

### Test Failures
```bash
# Run locally
cargo test -p {failing-crate}

# Debug
cargo test -p {failing-crate} -- --nocapture

# Fix the test or the code
# Commit: git commit -m "fix: {test failure cause}"
```

### Merge Conflicts
```bash
# Pull latest develop
git fetch origin
git merge origin/develop

# Resolve conflicts manually
# (Use VS Code GUI for easier visual merge)

# After fixing:
git add .
git commit -m "merge: resolve conflicts"
git push origin {your-branch}

# Re-request review
```

### Performance Regression
```bash
# Run benchmark
cargo bench -p {crate}

# Compare with baseline
# If >10% slower: investigate or revert

# Document in PR
```

---

## 💬 GETTING HELP

**Technical questions?**
- Post in `#x3-dev` Slack channel
- Include: module, issue, error message
- Tag: `@tech-lead`

**Design questions?**
- Post in GitHub Discussions
- Reference: design doc or RFC
- Tag: `@architecture-team`

**Blocked & urgent?**
- Post in `#blockers`
- Include: what's blocked, needed by when
- Tag: `@sprint-lead`

**Can't push/merge?**
- Check GitHub status page
- Try: `git fetch --all && git status`
- Ask DevOps if CI is down

---

## 🎯 END STATE: TESTNET READY (Week 20)

When Sprint 8 ends (Sep 12), we should have:

- ✅ 8 major modules complete
- ✅ ~23,000 LOC shipped
- ✅ 100+ integration tests passing
- ✅ All 6 chain adapters deployed
- ✅ Emergency pause + refund tested
- ✅ Parallel executor proven correct
- ✅ Public documentation + guides
- ✅ Ready for mainnet readiness audit

**Deployment:** Public testnet goes live Sep 15 🚀


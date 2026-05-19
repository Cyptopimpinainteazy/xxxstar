# GitHub Projects Setup Guide

## Creating the Sprint Board

### Via GitHub Web UI (5 minutes)

1. **Navigate to:** Repository → Projects → New project
2. **Name:** "X3 V0.4 Sprint Board"
3. **Template:** "Table"
4. **Visibility:** Public (or Private if preferred)

### Board Configuration

#### Columns to Create
1. **🔴 Backlog** — Unstarted tasks
2. **🟡 In Progress** — Currently being worked on
3. **🔵 In Review** — Awaiting code review
4. **🟢 Done** — Completed this sprint

#### Custom Fields

**Field 1: Sprint**
- Type: Single select
- Options: sprint-0, sprint-1, sprint-2, ..., sprint-8

**Field 2: Module**
- Type: Single select
- Options: kernel, packet-standard, x3-ixl, gateway, services, parallel-executor, liquidity-core, universal-contracts

**Field 3: Priority**
- Type: Single select
- Options: 🔴 Critical, 🟡 High, 🔵 Medium, 🟢 Low

**Field 4: Effort (Hours)**
- Type: Number
- Used for capacity planning

**Field 5: Assignee**
- Type: People
- Required: Yes

---

## Sprint 0 Initial Tasks

### Phase 0.1: Canonical Supply Invariant Audit
**Title:** [SPRINT-0] Kernel: Canonical Supply Invariant Audit  
**Sprint:** sprint-0  
**Module:** kernel  
**Priority:** 🔴 Critical  
**Effort:** 6 hours  
**Assignee:** @lojak  

**Description:**
```
Audit and test the canonical supply invariant in x3-kernel.

Acceptance Criteria:
- [ ] SupplyLedger::check_invariant() reviewed
- [ ] 1,000+ random ops tested
- [ ] No violations found
- [ ] Tests added

Files: pallets/x3-kernel/src/
```

### Phase 0.2: Emergency Halt Path
**Title:** [SPRINT-0] Kernel: Emergency Halt Path Verification  
**Sprint:** sprint-0  
**Module:** kernel  
**Priority:** 🔴 Critical  
**Effort:** 5 hours  
**Assignee:** @lojak  

### Phase 0.3: Mint/Burn Guards
**Title:** [SPRINT-0] Kernel: Mint/Burn Permission Guards  
**Sprint:** sprint-0  
**Module:** kernel  
**Priority:** 🔴 Critical  
**Effort:** 4 hours  
**Assignee:** @lojak  

### Phase 0.4: Balance Reconciliation
**Title:** [SPRINT-0] Kernel: Cross-Domain Balance Reconciliation  
**Sprint:** sprint-0  
**Module:** kernel  
**Priority:** 🟡 High  
**Effort:** 4 hours  
**Assignee:** @lojak  

### Phase 0.5: Readiness Crate
**Title:** [SPRINT-0] Infrastructure: Readiness Report Crate  
**Sprint:** sprint-0  
**Module:** kernel  
**Priority:** 🟡 High  
**Effort:** 7 hours  
**Assignee:** @lojak  

---

## Automated Workflows (Optional)

### Auto-move to "In Review" on PR
```
Trigger: Pull Request opened
Action: Move card to "🔵 In Review"
Condition: Label = "sprint-0"
```

### Auto-move to "Done" on PR Merge
```
Trigger: Pull Request merged
Action: Move card to "🟢 Done"
Condition: Label = "sprint-0"
```

---

## Sprint 0 Board Template (View)

```
🔴 BACKLOG          | 🟡 IN PROGRESS      | 🔵 IN REVIEW        | 🟢 DONE
─────────────────────────────────────────────────────────────────────────
Phase 0.5           | Phase 0.1           |                     | 
Readiness Crate     | Supply Audit        |                     |
6h @lojak           | 6h @lojak (Mon-Tue) |                     |
                    |                     |                     |
                    |                     | (Will fill as       |
                    |                     |  PRs are created)   |
```

---

## GitHub Projects CLI Commands (Optional)

```bash
# List all projects
gh project list

# Create new project
gh project create --title "X3 V0.4 Sprint Board" --owner {org/user}

# Add issue to project
gh project item add {project-id} --issue {issue-number}

# Update item field
gh project item-field set {project-id} {item-id} --field Sprint --value sprint-0

# View project board
gh project view {project-id}
```

---

## Weekly Sprint Metrics (Captured Friday EOD)

Add to the project description (or a pinned issue):

```
## Sprint 0 Metrics (Apr 26 - May 3, 2026)

**Velocity:** X hours completed / Y hours estimated

**Task Completion:**
- [ ] Phase 0.1: ✅ Complete
- [ ] Phase 0.2: ✅ Complete
- [ ] Phase 0.3: ✅ Complete
- [ ] Phase 0.4: ✅ Complete
- [ ] Phase 0.5: ✅ Complete

**Build Time:** 4m 35s (target: <5 min) ✅

**Test Coverage:** 95% (target: >90%) ✅

**Code Review Time:** 18 hours (target: <24 hours) ✅

**Blockers:** None

**Next Sprint:** Sprint 1 - Packet Standard
```

---

## Backlog Management

### Triaging New Issues (Friday EOD)

For each new issue:
1. Add to project
2. Assign sprint (if known)
3. Assign module
4. Set priority
5. Estimate hours
6. Assign owner
7. Move to column

### Backlog Refinement (Before Sprint Planning)

```
For Sprint {N+1}:
- [ ] All tasks have effort estimate
- [ ] All tasks have clear AC
- [ ] Dependencies identified
- [ ] Blockers logged
- [ ] Owners assigned
- [ ] Ready for sprint kickoff
```

---

## Integration with Pull Requests

### PR Labels (Auto-Applied)

```
Label: sprint-0
When: PR branch starts with "sprint-0/"

Label: kernel
When: PR modifies pallets/x3-kernel/

Label: needs-review
When: PR opened

Label: ready-to-merge
When: PR has 2 approvals + tests passing
```

### PR Links Issues

In PR description:
```
Fixes #123 (Phase 0.1 issue)
Related to #124
```

This auto-links the PR to the GitHub Project card.

---

## Sprint Ceremony Tracking

### Sprint Planning (Monday 9 AM UTC)
- [ ] Review backlog
- [ ] Estimate new tasks
- [ ] Assign ownership
- [ ] Confirm sprint goal
- [ ] **Document in:** Sprint 0 description

### Daily Standup (Mon-Fri 10 AM UTC)
- Each person: "What today? Any blockers?"
- Update board after standup

### Sprint Review (Friday 4 PM UTC)
- [ ] Demo completed work
- [ ] Review metrics
- [ ] Capture learnings
- [ ] **Document in:** Sprint 0 summary comment

### Sprint Retrospective (Friday 4:30 PM UTC)
- What went well?
- What could improve?
- Action items for next sprint
- **Document in:** `.planning/RETRO_SPRINT_0.md`

---

## Dashboard View (Friday Reports)

Create a "Status" issue pinned to the project:

```markdown
# Sprint 0 Status (Week 1)

**Status:** 🟢 ON TRACK

**Completion:** 5/5 phases (100%)

**Metrics:**
- Build time: 4m 35s ✅
- Test coverage: 95% ✅
- Code reviews: 18h avg ✅
- Blockers: None ✅

**Next:** Sprint 1 kickoff Monday

Updated: May 3, 2026 EOD
```

---

## Ready to Use

1. **Create project** in GitHub web UI
2. **Add columns** as specified above
3. **Create custom fields** (Sprint, Module, Priority, Effort, Assignee)
4. **Add issues** for each phase task
5. **Use board** as single source of truth
6. **Update** in standup and sprint ceremonies

**Go live:** When Sprint 0 starts (Monday, Apr 29)


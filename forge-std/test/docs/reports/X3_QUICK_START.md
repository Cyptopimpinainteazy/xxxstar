# X3 Development: Quick Start Guide

**TL;DR**: X3 is tracked via milestones (epics → tasks) and formalized via OpenSpec proposals. Use both together.

---

## 📋 How to Track Your Work

### Step 1: Check the Master Milestone
See `/home/lojak/Desktop/x3-chain-master/docs/reports/X3_MILESTONE_TRACKING.md`

This shows:
- All 9 epics
- Current status
- Dependencies
- Quality gates

### Step 2: Find Your Task
Pick an epic, find the task that's blocking you.

Example:
- **EPIC-1**: Core Protocol  
  - Task: L0-01 Genesis Configuration Engine  
  - Status: [ ] Not started

### Step 3: Convert Task → GitHub Issue (or similar)
Create a tracking issue with:
- [ ] Acceptance criteria from milestone
- [ ] Link to relevant OpenSpec proposal
- [ ] Estimated completion date

### Step 4: Link to Proposal
If you're implementing against a proposal, reference it:

```
Implements X3-PROPOSAL-001

- [x] Kernel invariants defined
- [x] TLA+ spec written
- [ ] Coq proofs pending
```

### Step 5: Update Milestone Status
When done, mark the task complete in the milestone doc.

---

## 📐 When to Write an OpenSpec Proposal

Write a proposal when you're:

✅ **Starting** a major feature or protocol change  
✅ **Refactoring** significant architecture  
✅ **Changing** incentives (slashing, rewards, fees)  
✅ **Deciding** between competing designs  
✅ **Documenting** a controversial decision

❌ **Don't** write proposals for:
- Bug fixes
- UI improvements
- Documentation updates
- Small refactors

---

## 🚀 Current Phase: EPIC-1 Execution

You're currently in **Phase 1: Core Protocol & Kernel**

### What's In Progress
- [x] Design phase (complete)
- [ ] Implementation phase (NOW)

### Your Next Steps
1. ✅ Review docs/reports/X3_MILESTONE_TRACKING.md
2. ✅ Review openspec/X3_PROPOSAL_FRAMEWORK.md
3. 🔄 Pick an EPIC-1 task to start with
4. 📝 Create corresponding GitHub issue
5. 🔨 Code + commit
6. ✅ Mark task complete when done

### Recommended Starting Tasks (In Order)
1. **L0-01**: Genesis Configuration Engine
   - Just JSON schemas + version management
   - Blocks everything else
   - 1-2 days

2. **L0-02**: Append-Only Change Ledger
   - Simple file or SQLite WAL
   - Audit trail only
   - 1 day

3. **L0-03**: Capability Registry
   - Define capabilities
   - Validation logic
   - 1-2 days

4. **L1-01**: Operator Registry
   - Data structures
   - Registration logic
   - 1 day

Then you can test these with **L0-05: Local Chain Simulator**.

---

## 🧪 How Proposals Map to Epics

```
X3-PROPOSAL-001 (Kernel) 
    ↓
EPIC-1 (Core Protocol)
    ├─ Task L0-01 (Genesis)
    ├─ Task L0-02 (Ledger)
    ├─ Task L0-03 (Capabilities)
    ├─ Task L1-01 (Operator Registry)
    ├─ Task L1-02 (Bonding)
    ├─ Task L1-03 (Slashing)
    ├─ Task L1-04 (Governance)
    └─ Task L1-05 (Attack Sim)

X3-PROPOSAL-002 (Slashing & Economics)
    ↓
EPIC-6 (Economic Modeling)
    └─ [Tasks for modeling/simulation]
```

---

## 📊 Current Milestone Status

| Epic | Tasks | Status | Target |
|------|-------|--------|--------|
| EPIC-1 | 10/10 | 🟡 IN PROGRESS | Month 3 |
| EPIC-2 | 5/5 | 🔴 PENDING | Month 5 |
| EPIC-3 | 10/10 | 🔴 PENDING | Month 4 |
| EPIC-4 | 5/5 | 🔴 PENDING | Month 6 |
| EPIC-5 | 5/5 | 🔴 PENDING | Month 3 |
| EPIC-6 | 6/6 | 🔴 PENDING | Month 4 |
| EPIC-7 | 5/5 | 🔴 PENDING | Month 5 |
| EPIC-8 | 6/6 | 🟡 IN PROGRESS | Month 3 |
| EPIC-9 | 6/6 | 🔴 PENDING | Month 8 |

**Legend**:
- 🟡 IN PROGRESS: Some tasks done
- 🟢 COMPLETE: All tasks done
- 🔴 PENDING: Not started (blocked or not yet)

---

## 🎯 Critical Path (What Must Be Done First)

```
EPIC-1 (Kernel)
  ↓ (Month 2)
EPIC-2 (Formal Proofs)
  ↓ (Month 3)
EPIC-8 (Whitepaper)
  ↓
EPIC-9 (Mainnet)
```

**Parallel Tracks** (can start after EPIC-1):
- EPIC-3 (Command Center UI)
- EPIC-5 (Operator Onboarding)
- EPIC-6 (Economic Modeling)

**Cannot Start Until**:
- EPIC-7 (Devnet) needs EPIC-1 + EPIC-5
- EPIC-4 (Agents) blocks L2 features

---

## 💾 File Locations

| What | Where |
|------|-------|
| Master Milestones | `/docs/reports/X3_MILESTONE_TRACKING.md` |
| Proposal Framework | `/openspec/X3_PROPOSAL_FRAMEWORK.md` |
| Whitepaper (LaTeX) | `/whitepaper/x3-whitepaper.tex` |
| Contracts (Rust) | `/node/src/` |
| Command Center | `/command-center/src/` |
| Devnet Config | `/devnet/genesis.json` |

---

## 📋 Common Workflows

### Workflow 1: "I want to start a task"

```bash
1. Pick task from docs/reports/X3_MILESTONE_TRACKING.md
2. Check if it's blocked (dependencies)
3. Create GitHub issue with acceptance criteria
4. Reference related proposal (if any)
5. Start coding
```

### Workflow 2: "I found a design issue"

```bash
1. Open the relevant OpenSpec proposal
2. Add your concern to "Open Questions"
3. Request review from core team
4. Wait for consensus before coding
```

### Workflow 3: "I want to propose a major change"

```bash
1. Check if a proposal exists already
2. If not, copy the proposal template
3. Fill in all sections (especially rationale)
4. Submit for review (min 1 week)
5. Incorporate feedback
6. Mark as ACCEPTED
7. Update milestones accordingly
```

### Workflow 4: "I'm done with a task"

```bash
1. Code is committed + tests pass
2. Update task status to [x] in milestone
3. Close GitHub issue
4. Reference the commit in the issue
5. Move to next task
```

---

## ⚠️ Important Rules

**DON'T**:
- Start a task that's blocked by another task
- Merge code without linking to a GitHub issue
- Change proposals after implementation starts
- Skip the formal verification step (EPIC-2)

**DO**:
- Link all code to issues
- Reference proposals in commits
- Test on devnet before mainnet
- Keep milestones updated weekly
- Ask questions in "Open Questions" before coding

---

## 🤝 How to Get Help

| Problem | Who | How |
|---------|-----|-----|
| Unclear requirement | @protocol-core | Comment on GitHub issue |
| Design decision | @protocol-core | Open OpenSpec proposal |
| Implementation help | @dev-team | Pair program |
| Formal verification | @crypto-team | Review TLA+/Coq |
| Devnet deployment | @devnet-ops | Staging environment |

---

## 📈 Success Metrics

You'll know you're on track when:

- ✅ EPIC-1 tasks are 50%+ complete (end of Month 1)
- ✅ EPIC-2 formal proofs are passing (end of Month 2)
- ✅ Whitepaper is published (end of Month 3)
- ✅ Devnet is live with real operators (end of Month 4)
- ✅ Mainnet ready-to-launch checklist is all green (end of Month 7)

---

## 🚀 Launch Readiness Checklist

Before you can flip the switch on mainnet:

- [ ] L1 node binary passes all tests
- [ ] TLA+ proofs are verified
- [ ] Devnet ran 4 weeks without incident
- [ ] Governance passed 5+ proposals
- [ ] Slashing was triggered and worked correctly
- [ ] All operators bonded successfully
- [ ] External audit cleared the code
- [ ] Whitepaper published

---

## 📞 Questions?

If something isn't clear:

1. Check `/docs/reports/X3_MILESTONE_TRACKING.md` for the big picture
2. Check `/openspec/X3_PROPOSAL_FRAMEWORK.md` for design decisions
3. Check the relevant epic's tasks for specifics
4. Ask in GitHub issues or proposal discussions

---

**Good luck building X3. Make it boring. Make it correct. Make it survive.** 🚀

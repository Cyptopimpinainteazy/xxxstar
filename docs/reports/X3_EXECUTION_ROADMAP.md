# X3 Execution Roadmap (Month-by-Month)

**This is your execution plan. Follow it step-by-step.**

---

## Owners & CI ✅

**Roadmap owner:** @team/core  
**Month owners:** Add per-month owners using the `monthly-checklist` issue template.  

**Month owners (linked to monthly issues):**
- Month 1: @team/core — Issue #1257
- Month 2: @team/core — Issue #1258
- Month 3: @team/core — Issue #1259
- Month 4: @team/core — Issue #1260
- Month 5: @team/core — Issue #1261
- Month 6: @team/core — Issue #1262
- Month 7: @team/core — Issue #1263
- Month 8: @team/core — Issue #1264

*(Per-month issues created using the `monthly-checklist` template. Replace `@team/core` with specific GitHub handles and set `Target Completion` dates in each issue.)*

**Required CI checks (machine-checkable acceptance gates):**
- `build` — builds: `cargo build --all --locked` (artifact: `build.log`)
- `x3-invariants` — TLA+ + property tests (artifact: `tla-check.log`)
- `simulator-replay` — deterministic simulator replay test (artifact: `simulator.log`)
- `slashing-property` — property-based slashing tests (artifact: `slashing.log`)

> Tip: For each month's acceptance gate, reference the CI job name above so pass/fail is explicit.

---

## MONTH 1: Foundation & Kernel (NOW)

### What You're Building
The L1 blockchain core — immutable, verifiable, replayable.

### Critical Path Tasks
```
L0-01 (Genesis Config)
  ↓
L0-02 (Audit Ledger)
  ↓
L0-03 (Capability Registry)
  ↓
L0-04 (Module Loader)
  ↓
L0-05 (Local Simulator)
```

### Parallel Work
- **L1-01**: Operator Registry
- **L1-02**: Bonding Escrow
- **EPIC-8 Start**: Begin whitepaper (abstract, motivation)

### Acceptance Gate
✅ You must have:
- [ ] Rust node compiles without warnings
- [ ] Genesis config is deterministic + versioned
- [ ] Ledger prevents tampering
- [ ] Capabilities can be registered and queried
- [ ] Local simulator produces identical outputs on replay

### Deliverable
```
x3-node (binary)
├── genesis.v0.json
├── capability.toml
└── simulator.log
```

---

## MONTH 2: Slashing & Verification (PARALLEL)

### What You're Building
Economic enforcement + mathematical proof.

### Critical Tasks
```
L1-03 (Slashing Engine)
  ↓
L1-04 (Governance v0)
  ↓
EPIC-2: TLA+ Proofs
  ↓
EPIC-6 Start: Economics Modeling
```

### Acceptance Gate
✅ You must have:
- [ ] Slashing function is deterministic
- [ ] No two different operators can slash for same fault
- [ ] Bonds can never go negative
- [ ] TLA+ proof checker validates all invariants
- [ ] Economic equilibrium proven (slash > attack)

### Deliverable
```
x3-whitepaper (Chapters 1-4)
├── Motivation
├── Architecture
├── Consensus
└── Economics (draft)

X3.tla (formal spec)
├── SlashingSafety proven
├── GovernanceSafety proven
└── NoNegativeBonds proven
```

---

## MONTH 3: Publication & UI Start (PARALLEL)

### What You're Building
Public credibility + the command center beginning.

### Critical Tasks
```
EPIC-8: Whitepaper → PDF
  ↓
EPIC-3 Start: Tauri + Three.js
  ↓
EPIC-5 Start: Operator CLI
```

### Acceptance Gate
✅ You must have:
- [ ] Whitepaper published (PDF)
- [ ] External auditor scheduled
- [ ] Tauri app boots with empty scene
- [ ] x3-operator CLI has `--help`
- [ ] Docker image builds and runs node

### Deliverable
```
x3-whitepaper.pdf (complete)

x3-command-center (binary)
├── Main.tsx
├── Three.js scene rendering
└── IPC router

x3-operator (CLI binary)
├── x3-operator doctor
├── x3-operator init
└── x3-operator bond
```

---

## MONTH 4: Command Center & Economics (PARALLEL)

### What You're Building
The operator apps/dash-legacy-2-legacy-2board + proof that incentives work.

### Critical Tasks
```
EPIC-3: Level 0 Panels
  ├─ Genesis Panel
  ├─ Capabilities Panel
  ├─ Ledger Panel
  └─ Simulator Panel

EPIC-6: Complete Economics
  ├─ Reward curves
  ├─ Slash equilibrium
  ├─ Fee dynamics
  └─ Attack cost calculation
```

### Acceptance Gate
✅ You must have:
- [ ] Genesis panel shows immutable config
- [ ] Slashing simulator calculates penalties correctly
- [ ] Economics report proves attack is unprofitable
- [ ] Operator CLI can list validators
- [ ] Dashboard reads live node state via RPC

### Deliverable
```
Command Center (L0)
├── Genesis Editor Panel
├── Capability Registry Panel
├── Ledger Viewer Panel
└── Simulator Panel (deterministic replay)

Economics Report
├── ROI curves (validator, GPU, storage)
├── Attack cost analysis
├── Governance capture risk
└── Fee market simulation
```

---

## MONTH 5: Devnet Prep & Agents (PARALLEL)

### What You're Building
Real devnet infrastructure + autonomous agent framework.

### Critical Tasks
```
EPIC-5: Operator Onboarding Complete
  ├─ CLI full feature set
  ├─ Public apps/dash-legacy-2-legacy-2board live
  └─ Operator guide published

EPIC-4 Start: Agent Framework
  ├─ Agent manifest schema
  ├─ Agent supervisor
  └─ Kill switch mechanism

EPIC-7 Start: Devnet Phase 0 (solo)
```

### Acceptance Gate
✅ You must have:
- [ ] Operator guide is published + publicly accessible
- [ ] Devnet rewards/slashing active
- [ ] First agent can register and act
- [ ] Agent supervisor can halt agents
- [ ] Agent actions are logged on-chain

### Deliverable
```
Devnet v0.1 (solo phase)
├── genesis.json (mainnet equivalent)
├── operator guide (public website)
├── apps/dash-legacy-2-legacy-2board (https://devnet.x3.app)
└── faucet (x3 faucet --amt 1000)

Agent Framework
├── agent.toml (manifest schema)
├── supervisor (Rust process)
└── kill switch (time-locked)
```

---

## MONTH 6: Public Devnet & L2 (PARALLEL)

### What You're Building
The actual devnet with real actors outside your machine.

### Critical Tasks
```
EPIC-7: Phase 1 (Public Operators)
  ├─ First external operator registers
  ├─ Bonding escrowed on-chain
  ├─ Rewards flowing
  └─ Leaderboard live

EPIC-4: Complete Agent Framework
  ├─ Agent treaties (contracts)
  ├─ Multi-agent negotiation
  └─ Agent + DAO attack simulator

EPIC-3: Level 1 Panels
  ├─ Operator Dashboard
  ├─ Topology Map
  ├─ Slashing Simulator
  └─ Agent Supervisor
```

### Acceptance Gate
✅ You must have:
- [ ] ≥3 independent validators running devnet
- [ ] Slashing triggered (intentionally) and executed correctly
- [ ] Operators earned and withdrew rewards
- [ ] Agents coordinate without trust breaking
- [ ] Level 1 command center panels show real data

### Deliverable
```
Devnet v0.2 (public phase)
├── ≥3 independent validators
├── Operator leaderboard
├── Slashing history
└── Incident playbook

Command Center (L1)
├── Validator/Operator stats
├── Network topology
├── Slashing simulator
└── Agent supervisor
```

---

## MONTH 7: Adversarial Devnet & Mainnet Prep

### What You're Building
Breaking your own chain intentionally + final mainnet readiness.

### Critical Tasks
```
EPIC-7: Phase 2 (Adversarial Testing)
  ├─ Intentional double-signs
  ├─ Network partitions
  ├─ Governance spam
  ├─ Agent collusion
  └─ Paid bug bounties

Mainnet Readiness Checklist
  ├─ Audit complete (external)
  ├─ All invariants proven
  ├─ Devnet stable 4 weeks
  └─ Genesis ceremony rehearsed
```

### Acceptance Gate
✅ You must have:
- [ ] Devnet survived 4 weeks of adversarial attacks
- [ ] All attacks detected and penalized correctly
- [ ] Zero critical bugs found
- [ ] External audit passed
- [ ] Mainnet readiness checklist all green

### Deliverable
```
Devnet v0.3 (adversarial)
├── Attack results report
├── Incidents resolved
├── Post-mortems written
└── Fixes committed

Mainnet Readiness
├── ✅ Code audit complete
├── ✅ Formal proofs verified
├── ✅ Economics sound
├── ✅ Governance tested
└── ✅ All validators ready
```

---

## MONTH 8: Mainnet Genesis & Launch 🚀

### What You're Building
The final blastoff.

### Critical Tasks
```
EPIC-9: Genesis Ceremony
  ├─ Parameter freeze
  ├─ Validator coordination
  ├─ Ceremony rehearsal (dry run)
  ├─ Real ceremony (witnessed)
  └─ First block produced

Mainnet Stabilization
  ├─ Monitor for 1 week
  ├─ No rollbacks
  ├─ Governance passes proposals
  └─ Declare success
```

### Acceptance Gate
✅ You must have:
- [ ] Genesis hash signed by ≥5 validators
- [ ] First block finalized
- [ ] Governance vote passes (no founder intervention)
- [ ] 1 week chain uptime
- [ ] Public announcement

### Deliverable
```
MAINNET LIVE
├── genesis.json (immutable)
├── Chain ID (e.g., "x3-1")
├── Block 1 (and counting up)
└── Governance working
```

---

## Post-Launch: Stewardship

### MONTH 9+: Self-Governance

You now step back. The DAO governs:

- Protocol upgrades (via governance vote)
- Parameter adjustments (via governance vote)
- Emergency responses (via guardian + DAO)
- Operator slashing (automatic)

**Your role**: Advisor, not ruler.

---

## 📊 Month-by-Month At a Glance

```
MONTH 1  [████ EPIC-1 START ████]
MONTH 2  [████ EPIC-1 CONTINUE ████] [██ EPIC-2/6 START ██]
MONTH 3  [████ EPIC-1 DONE ████] [███ EPIC-2/3/8 ███]
MONTH 4  [██ EPIC-3 ██] [██ EPIC-6 ██] [█ EPIC-5 █]
MONTH 5  [██ EPIC-3/4/5 ██] [█ EPIC-7 START █]
MONTH 6  [██ EPIC-3/4/5 DONE ██] [███ EPIC-7 ███]
MONTH 7  [████ EPIC-7 ADVERSARIAL ████] [█ MAINNET PREP █]
MONTH 8  [███████ LAUNCH ███████]
```

---

## 🎯 Weekly Tracking (Simple Template)

Use this every Friday:

```markdown
# Week of [DATE]

## Completed
- [ ] L0-01: Genesis config done
- [ ] Rust node compiles
- [ ] Ledger prevents tampering

## In Progress
- [ ] L0-02: Audit ledger
- [ ] L0-03: Capability registry

## Blocked By
- None

## Risks
- None identified

## Next Week
- Start L0-03
- Begin whitepaper abstract
```

---

## 🚨 Red Flags (Stop If You See These)

If any of these happen, **go back or ask for help**:

🚫 **Invariant broken**: Don't continue until fixed  
🚫 **Security audit fails**: Don't launch until resolved  
🚫 **Devnet crashes**: Don't advance phase until root caused  
🚫 **Governance vote fails**: Don't ignore — rethink parameters  
🚫 **Attack succeeds on devnet**: You're not ready for mainnet  

---

## ✅ Full Checklist to Mainnet

Print this out. Use it as your guide.

### Month 1 Checklist
- [ ] Rust node runs
- [ ] Genesis config immutable
- [ ] Ledger tamperproof
- [ ] Capabilities registrable
- [ ] Simulator deterministic

### Month 2 Checklist
- [ ] Slashing works
- [ ] Governance validates proposals
- [ ] TLA+ proofs pass
- [ ] Economics equilibrium proven

### Month 3 Checklist
- [ ] Whitepaper published
- [ ] Tauri app boots
- [ ] x3-operator CLI works
- [ ] Docker images available

### Month 4 Checklist
- [ ] Command center shows genesis
- [ ] Economics report complete
- [ ] CLI lists validators
- [ ] Dashboard reads live state

### Month 5 Checklist
- [ ] Operator guide published
- [ ] Agent framework registered
- [ ] Devnet runs solo
- [ ] Agents can be supervised

### Month 6 Checklist
- [ ] ≥3 external validators
- [ ] Slashing triggered and worked
- [ ] Operators earned rewards
- [ ] Level 1 panels show data

### Month 7 Checklist
- [ ] Devnet survived 4 weeks
- [ ] All attacks detected
- [ ] External audit passed
- [ ] Mainnet readiness green

### Month 8 Checklist
- [ ] Genesis ceremony completed
- [ ] Block 1 produced
- [ ] Governance voted
- [ ] 1 week uptime

---

## 🚀 You're Ready When

You can say "YES" to all of these:

- "My L1 is provably safe (TLA+)" ✅
- "My incentives are sound (economic modeling)" ✅
- "My devnet survived attacks" ✅
- "My governance worked without capture" ✅
- "External auditors cleared the code" ✅
- "Operators are bonded and ready" ✅
- "My ceremony is documented and rehearsed" ✅

**Then you launch.**

---

**This is your map. Follow it. Update it weekly. Ship it.**

Good luck. 🚀

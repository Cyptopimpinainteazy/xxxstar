# Constitutional Jury Selection via Voir Dire

## ⚖️ The Model (As You Specified)

All jury member selection now follows **proper voir dire** with these constraints:

### 1️⃣ Lawyers Do Voir Dire, NOT Selection
- Lawyers **question & strike** candidates
- Lawyers do **NOT hand-pick** jurors
- Final empanelment is **randomized** from remaining qualified pool
- This **preserves entropy** and prevents capture

### 2️⃣ Dual-Counsel Model (Always)
- **DA Counsel** ("prosecution"): Questions for safety/procedural concerns
- **Defense Counsel**: Questions for fairness/due-process concerns
- Both see **anonymized juror profiles ONLY** (no identities)
- Both have **symmetric strike limits** (identical constraints)
- Strike justifications logged with formal reason codes

### 3️⃣ Lawyer Actions Are First-Class Evidence
Every strike is recorded in Scrap Yard audit trail:
```json
{
  "actor": "counsel-da-alice",
  "action": "JUROR_STRIKE",
  "juror_profile_hash": "juror-x9f2",  // No actual ID exposed
  "reason_code": "SAFETY_CONCERN",      // Formal reason
  "case_id": "appeal-20260211-001",
  "timestamp": "2026-02-11T13:02:11Z"
}
```
Later appellate review can **question the lawyers themselves**.

### 4️⃣ Anonymization Until Post-Decision Review
- Lawyers see only **profile hashes**, not juror IDs
- Identity mapping is **hidden** during voir dire
- Only revealed **after jury has voted** (audit purpose)
- Prevents identity-based manipulation

### 5️⃣ Symmetric Constraints on Both Sides
```
Peremptory strikes (cause-free):     max 3 per side
For-cause strikes (reasoned):        max 10 per side
Total candidates:                    unlimited
Minimum jury size:                   3 members
```

### 6️⃣ Bias Pattern Detection
System automatically audits lawyers for:
- **Ossified patterns** (same strike reason > 5x = flag)
- **Section targeting** (>70% from one section = audit)
- **Extreme peremptory use** (all 3 strikes = scrutiny)

### 7️⃣ Mutual Strike Analysis
When **both DA and Defense strike the same juror**:
- This is an **anomaly** worth investigation
- Why did everyone agree?
- Recorded and flagged for appellate scrutiny

## 📊 Implementation Architecture

```
VoirDireManager (swarm/jury/voir_dire.py)
├── anonymize_candidates()           # Hash juror IDs
├── strike_juror()                   # Record strike with reason
├── finalize_empanelment()          # Random selection from remaining
├── detect_lawyer_bias_patterns()   # Audit strike histories
└── resolve_mutual_strikes()        # Anomaly detection

JuryManager (swarm/jury/manager.py)
├── create_session_via_voir_dire() # NEW: Proper voir dire workflow
└── create_session()                # Legacy: Simple jury creation (backward compat)
```

## 🧪 Test Coverage (13 Tests, All Passing)

**Voir Dire Core (7 tests)**
- ✅ Anonymization (identities hidden from lawyers)
- ✅ Dual-counsel symmetric strikes
- ✅ Peremptory strike limits enforced
- ✅ Mutual strikes detected
- ✅ Randomized empanelment
- ✅ Lawyer bias pattern detection
- ✅ Complete audit trail generation

**Jury Integration (3 tests)**
- ✅ Full workflow: anonymization → voir dire → empanelment → voting
- ✅ Handles insufficient candidates gracefully
- ✅ Backward compatibility with simple jury creation

**Jury Manager (3 tests)**
- ✅ Voir dire empanelment process
- ✅ Simple jury creation (legacy)
- ✅ Commit-reveal voting lifecycle

## 🛠️ Usage Example

```python
from swarm.jury import JuryManager

jm = JuryManager()

# Create jury via PROPER voir dire
success, session, audit = jm.create_session_via_voir_dire(
    case_id="case-2026-001",
    task_ids=["amendment-proposal"],
    candidate_ids=["agent-1", "agent-2", ..., "agent-12"],
    candidate_data={
        "agent-1": {"reputation": 0.75, "section": "governance", ...},
        ...
    },
    prosecution_counsel_id="da-counsel-alice",
    defense_counsel_id="defense-counsel-bob",
    jury_size=6,
)

if success:
    # Session created with full voir dire audit trail
    audit_trail = session.metadata["voir_dire_audit"]
    print(f"Seated: {len(session.members)}")
    print(f"DA strikes: {audit_trail['prosecution_strikes']}")
    print(f"Defense strikes: {audit_trail['defense_strikes']}")
```

## 🎯 Why This Design (Your Principles)

✅ **Slow Power**
- Multi-step process: anonymize → question → strike → randomize → vote
- No single person makes arbitrary selections
- Both sides constrain each other

✅ **Corruption Visibility**
- All strikes logged with reason codes
- Bias patterns automatically detected
- Lawyers can be audited for systematic bias

✅ **Entropy Preservation**
- Randomized selection from approved pool
- Not hand-chosen by single lawyer
- Prevents capture by any one actor

✅ **Auditable After Fact**
- Identity mappings hidden until post-vote
- Strike histories linked to specific lawyers
- Mutual strikes flagged for investigation
- Full replay-able audit trail in Scrap Yard

## 📝 Deprecated

The old `lawyer.py` hand-picking system is now **deprecated** in favor of voir dire.
The `create_session()` method without voir dire parameters is kept for backward compatibility only.

**New canonical path**: `create_session_via_voir_dire()`

## 🏁 Result

You now have a jury system that:
- ✅ Requires lawyer participation (see-dire questioning)
- ✅ BUT constrains lawyers (symmetric, documented, monitored)
- ✅ Preserves randomness (final selection is shuffled)
- ✅ Makes corruption noisy (bias patterns surface early)
- ✅ Supports appellate review (lawyers are auditable)

"Who guards the guardians? Paperwork and time." 🛠️

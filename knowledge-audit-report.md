# X3 Atomic Star — Knowledge Entry Audit Report

**Date:** 2026-05-10  
**Scope:** All markdown knowledge files in `Cyptopimpinainteazy/x3-atomic-star`  
**Files reviewed:** 100 markdown files across `.ai/`, `.legion/`, `.swarm/`, `.x3/`, `.planning/`, `.reports/`, `.roo/`, `.traycer/`, `.github/prompts/`, and root-level docs  
**Devin Knowledge Notes:** 0 (no Devin-managed knowledge notes exist for this org/repo)

---

## PART 1: DUPLICATES FOUND

### D1. `.legion/` vs `.swarm/prompts/` — 5 Agent Definitions Exist Twice

| Agent | `.legion/` (detailed) | `.swarm/prompts/` (condensed) |
|---|---|---|
| ARCHITECT | 34 lines, full focus areas, output files, ranking criteria | 11 lines, condensed summary of same content |
| AUDITOR | 32 lines, 10-item check list, 7 output files | 12 lines, 2 output files, missing check items |
| FIXER | 20 lines, 6-step process, secrets protection rule | 13 lines, no process steps, no secrets rule |
| INTEGRATOR | 28 lines, 7-item priority list, tracking rules | 15 lines, condensed version of same rules |
| SCANNER | 25 lines, script commands, X3 focus areas | 13 lines, stripped of scripts/focus areas |

**Risk:** The `.swarm/prompts/` versions drop important rules present in `.legion/` (e.g., Fixer's secrets protection, Auditor's 10-item check list). An agent loading only the swarm version will miss critical guardrails.

**Recommendation:** Consolidate into one location. The `.legion/` versions are more complete — either delete `.swarm/prompts/{ARCHITECT,AUDITOR,FIXER,INTEGRATOR,SCANNER}.md` and point references there, or merge the `.legion/` content into the `.swarm/prompts/` versions and delete `.legion/`.

---

### D2. `X3_END_TO_END_GAPS_MASTER_PLAN.md` — Exact Duplicate in Two Locations

| Location | Lines | Difference |
|---|---|---|
| `/X3_END_TO_END_GAPS_MASTER_PLAN.md` (root) | 343 | Uses `docs/implementation_plan_atlas.md` links |
| `/apps/atlas-sphere-clean/X3_END_TO_END_GAPS_MASTER_PLAN.md` | 344 | Uses `implementation_plan.md` (relative) links + trailing "Allo" typo |

Content is 99.5% identical. Only relative link paths differ, plus a stray "Allo" at EOF in the atlas-sphere copy.

**Recommendation:** Delete `apps/atlas-sphere-clean/X3_END_TO_END_GAPS_MASTER_PLAN.md`. Keep the root version as canonical. Fix any references.

---

### D3. Two Commander Prompts — Overlapping Role, Different Config

| File | Identity | Context Loaded | Procedure |
|---|---|---|---|
| `.swarm/prompts/COMMANDER.md` | "Level 10 Swarm Commander" | 11 files incl. AGENT_ROSTER, task_queue, TASK_CHAIN | 8-step cycle starting with `.scripts/x3_level10_swarm.sh` |
| `.swarm/prompts/COMMANDER_LEVEL10.md` | "Swarm Commander" | 12 files incl. ENGINEERING_CONSTITUTION, SWARM_CONFIG, DRIFT/EVAL reports | 9-step procedure without shell scripts |

Both claim to be the Level 10 commander. They load different context sets and follow different procedures.

**⚠️ NEEDS YOUR INPUT:** Which Commander prompt is canonical? They can't both be the "real" one — agents will get confused about which one to load. Suggest merging into one, or renaming to clarify distinct roles (e.g., `COMMANDER_SWARM.md` vs `COMMANDER_CONTROL_PLANE.md`).

---

### D4. Cost Control Rules — Stated in 3 Places

| Location | Content |
|---|---|
| `.x3/COST_CONTROL.md` | 16-line standalone cost rules |
| `.roo/rules.md` (lines 50-54) | Inline "COST GUARDRAIL" block — same rules |
| `.ai/execution_prompts.md` (lines 6-11) | Inline "COST GUARDRAIL" block — same rules |

**Recommendation:** Keep `.x3/COST_CONTROL.md` as the single source of truth. Replace inline copies with "See `.x3/COST_CONTROL.md`".

---

### D5. Acceptance Criteria / Proof Standard — Stated in 3 Places

| Location | Content |
|---|---|
| `.x3/ACCEPTANCE_CRITERIA.md` | 11-line standalone criteria |
| `.x3/context/X3_ENGINEERING_CONSTITUTION.md` (lines 40-49) | "Proof Standard" section — identical criteria |
| `.x3/evals/X3_EVALS.md` | Overlapping but slightly different criteria (adds drift and mutation gate checks) |

**Recommendation:** Keep `.x3/ACCEPTANCE_CRITERIA.md` as canonical. Add the drift/mutation checks from X3_EVALS.md into it. Reference it from the Constitution instead of duplicating.

---

### D6. Core Invariants — Stated in 3 Places

| Location | Content |
|---|---|
| `.x3/invariants/X3_INVARIANTS.md` | 57-line detailed invariant definitions with "Required proof" sections |
| `.x3/context/X3_ENGINEERING_CONSTITUTION.md` (lines 17-36) | Same invariants, condensed |
| `.x3/attacks/BREAK_THE_CHAIN_SCENARIOS.md` | Same invariants expressed as attack scenarios |

**Recommendation:** Keep `.x3/invariants/X3_INVARIANTS.md` as the single invariant source. The Constitution and Attack Scenarios should reference it rather than restate the formulas.

---

### D7. "No stubs / no fake completion" Hard Rules — Stated in 5+ Places

Appears in: `.ai/system_prompt.md`, `.roo/rules.md`, `.swarm/prompts/COMMANDER.md`, `.x3/context/X3_ENGINEERING_CONSTITUTION.md`, `.traycer/X3_DEEP_DIVE_SPEC.md`

**Recommendation:** Define once in `.x3/context/X3_ENGINEERING_CONSTITUTION.md` (the project constitution). All other files should reference it: "See Non-Negotiables in `.x3/context/X3_ENGINEERING_CONSTITUTION.md`."

---

### D8. `.ai/tasks.md` is a Subset of `.ai/execution_prompts.md`

`.ai/tasks.md` (46 lines) is a condensed version of the 5 passes defined in `.ai/execution_prompts.md` (269 lines). Same passes, same steps, less detail.

**Recommendation:** Delete `.ai/tasks.md`. It adds no information beyond what's in `.ai/execution_prompts.md`.

---

### D9. Sprint 0 Details — Spread Across 4 Files

Sprint 0 is described in:
- `.planning/SPRINT_0_IMMEDIATE_EXECUTION.md` (453 lines — most detailed, day-by-day)
- `.planning/SPRINT_0_LAUNCH_CHECKLIST.md` (394 lines — overlapping checklist)
- `.planning/REFERENCE_CARD.md` (Sprint 0 section)
- `.planning/SPRINT_DETAILED_PLANS.md` (Sprint 0 section)

**Recommendation:** Keep `SPRINT_0_IMMEDIATE_EXECUTION.md` as the canonical Sprint 0 plan. The LAUNCH_CHECKLIST covers different ground (pre-launch infra) so it can stay, but its Sprint 0 mission/phases section should reference the IMMEDIATE_EXECUTION doc instead of repeating it.

---

### D10. Drift Report vs Mutation Gate — Near-identical Reports

| File | Paths | Danger-zone |
|---|---|---|
| `.x3/reports/DRIFT_REPORT.md` | 371 | 49 |
| `.x3/reports/MUTATION_GATE.md` | 372 | 49 |

Same danger-zone file list, same required actions. These are generated from the same worktree scan but framed differently (drift vs mutation).

**Recommendation:** These are generated reports from different scripts (drift detector vs mutation gate). They serve different purposes (drift = docs/code mismatch; mutation = patch authorization). Keep both but add a header noting they are auto-generated and may overlap.

---

### D11. DO_NOT_TOUCH Protected Items — Stated in 3 Places

| Location | Content |
|---|---|
| `.x3/DO_NOT_TOUCH.md` | 12-line list of protected paths |
| `.x3/mutations/DANGER_ZONES.md` | Overlapping list (same paths + runtime/pallets/contracts) |
| `.roo/rules.md` (lines 57-58, 69) | Inline reference to same items |

**Recommendation:** Keep `.x3/DO_NOT_TOUCH.md` as the authoritative "never touch" list. `DANGER_ZONES.md` has a broader scope (includes code directories, not just secrets) so it serves a different purpose — keep both. `.roo/rules.md` should say "See `.x3/DO_NOT_TOUCH.md`" instead of inlining.

---

## PART 2: CONTRADICTIONS FOUND

### ⚠️ C1. Sprint 0 Team Size — 1 vs 2 Engineers

| Source | Team Size |
|---|---|
| `.planning/SPRINT_DETAILED_PLANS.md` | **2 engineers** |
| `.planning/SPRINT_0_IMMEDIATE_EXECUTION.md` | **1 engineer** (lead: @lojak) |
| `.planning/SPRINT_0_LAUNCH_CHECKLIST.md` | **1 engineer** (lead: @lojak) |
| `.planning/REFERENCE_CARD.md` | **1 engineer** (@lojak) |

**NEEDS YOUR INPUT:** Is Sprint 0 a 1-person or 2-person effort? Three sources say 1, one says 2.

---

### ⚠️ C2. Sprint 0 Start Date — 2025 vs 2026

| Source | Date |
|---|---|
| `.planning/QUICK_EXECUTION_GUIDE.md` | Monday, **April 28, 2025** |
| `.planning/SPRINT_0_IMMEDIATE_EXECUTION.md` | April 26, **2026** |
| `.planning/SPRINT_0_LAUNCH_CHECKLIST.md` | April 26, **2026** |
| `.planning/REFERENCE_CARD.md` | Apr 29 – Sep 15, **2026** |

**NEEDS YOUR INPUT:** QUICK_EXECUTION_GUIDE says 2025 — this is likely a typo and should be 2026 to match the other 3 sources. Confirm?

---

### ⚠️ C3. Supply Invariant Formula — Missing `x3vm` Component

| Source | Formula |
|---|---|
| `.x3/invariants/X3_INVARIANTS.md` | `canonical_supply == native + evm + svm + external_locked + pending` |
| `.x3/context/X3_ENGINEERING_CONSTITUTION.md` | Same (no x3vm) |
| `.planning/SPRINT_DETAILED_PLANS.md` Phase 0.1 | `canonical_supply == native + evm + svm + **x3vm** + external_locked + pending` |

**NEEDS YOUR INPUT:** Should the canonical supply formula include `x3vm` as a separate balance bucket? The planning doc adds it, but the invariant definitions and constitution both omit it.

---

### ⚠️ C4. `.roo/rules.md` "Ask Before Mutating" vs Execution Prompts

- `.roo/rules.md` line 21-22: "Do not use write, command, or MCP tools automatically. **Ask before mutating files.**"
- `.ai/execution_prompts.md`: Multiple passes say to "Run" scripts and "Create" files directly without asking.

**NEEDS YOUR INPUT:** Is the "ask before mutating" rule still in effect, or does it only apply to certain agents/modes? The execution prompts explicitly instruct running scripts and creating files, which contradicts this rule.

---

### ⚠️ C5. Auditor Output Files Mismatch

| Source | Output Files |
|---|---|
| `.legion/AUDITOR.md` | 7 files: DEEP_AUDIT_REPORT, SECURITY_BLOCKERS, MAINNET_READINESS_DELTA, FINAL_AUDIT_REPORT, FINAL_BOSS_AUDIT, P0_BLOCKERS, NEXT_10_PATCHES |
| `.swarm/prompts/AUDITOR.md` | 2 files: P0_BLOCKERS, MAINNET_READINESS_DELTA |

**Impact:** An agent using the swarm version will only produce 2 of the 7 expected outputs.

**Recommendation (part of D1 consolidation):** Merge into one complete list.

---

### ⚠️ C6. Smell Scan / File Count Discrepancies Between Reports

| Metric | `SCAN_BOOTSTRAP_SUMMARY.md` | `ROO_CODE_SETUP.md` |
|---|---|---|
| Smell scan lines | 32,089 | 55,881 |
| File count | 114,395 | 114,435 |

These are from different scan runs but neither is marked as superseding the other.

**Recommendation:** Add timestamps and "supersedes" headers to generated reports so it's clear which is current.

---

### ⚠️ C7. `claude.prompt.md` — Not X3 Knowledge

`.github/prompts/claude.prompt.md` is a 3,960-line copy of Anthropic's Claude system prompt (product info, refusal handling, etc.). This is not X3-specific knowledge — it's Claude's generic system prompt.

**NEEDS YOUR INPUT:** Was this intentionally placed here? It doesn't contain X3-specific guidance and could confuse agents that load `.github/prompts/` files as project context. Recommend deleting or replacing with actual X3-specific Claude instructions.

---

## PART 3: RECOMMENDED CONSOLIDATION ACTIONS

### Safe to Do (No Contradictions — Pure Dedup)

| # | Action | Files Affected |
|---|---|---|
| 1 | Delete `apps/atlas-sphere-clean/X3_END_TO_END_GAPS_MASTER_PLAN.md` (exact duplicate of root) | 1 file removed |
| 2 | Delete `.ai/tasks.md` (subset of `.ai/execution_prompts.md`) | 1 file removed |
| 3 | Replace inline cost guardrail blocks in `.roo/rules.md` and `.ai/execution_prompts.md` with reference to `.x3/COST_CONTROL.md` | 2 files edited |
| 4 | Replace inline invariant formulas in Constitution with reference to `.x3/invariants/X3_INVARIANTS.md` | 1 file edited |
| 5 | Replace inline acceptance criteria in Constitution with reference to `.x3/ACCEPTANCE_CRITERIA.md` | 1 file edited |

### Needs Your Decision First

| # | Decision Needed | Options |
|---|---|---|
| A | Which agent dir is canonical? | Keep `.legion/` (more complete) OR merge into `.swarm/prompts/` |
| B | Which Commander prompt is canonical? | Merge the two OR rename for distinct roles |
| C | Sprint 0 team size: 1 or 2? | Fix SPRINT_DETAILED_PLANS.md to 1, or update the others to 2 |
| D | Sprint 0 date: 2025 or 2026? | Fix QUICK_EXECUTION_GUIDE.md year to 2026 (likely typo) |
| E | Supply formula: include `x3vm` or not? | Update formula everywhere to match one canonical version |
| F | "Ask before mutating" rule: still active? | Clarify scope in `.roo/rules.md` |
| G | Delete/replace `claude.prompt.md`? | Remove generic Claude prompt, or replace with X3-specific instructions |

---

## PART 4: SUMMARY

- **12 duplicate clusters** identified across 100 files
- **7 contradictions** requiring your decision
- **5 safe consolidation actions** ready to execute on your approval
- **7 decisions** needed before remaining consolidation can proceed
- **0 Devin knowledge notes** exist — all knowledge lives in-repo markdown files

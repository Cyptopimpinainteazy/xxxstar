# Orchestra Implementation Blueprint

This document outlines the step-by-step implementation plan to transition the system to the "Orchestra" architecture.

## Phase 0: Naming & Constitution (Days 1-2)
**Goal:** Lock terminology and immutable laws.

- [x] **Create `orchestra/SCORE_v1.md`**
    - [x] Define the 10 Immutable Commandments.
    - [x] Define constitutional definitions (Orchestra, Score, Agent, Jury, etc.).
- [x] **Create `orchestra/naming_conventions.md`**
    - [x] Map all functional roles to Orchestra Sections (Strings, Woodwinds, Brass, Percussion).
    - [x] Define naming standards for files, classes, and IDs.
- [x] **Establish Code Structure**
    - [x] Initialize `orchestra/` python package.
    - [x] Create `orchestra/core/enums.py`: `Section` types, `AgentRole` enums.
    - [x] Create `orchestra/core/consts.py`: Immutable constants (e.g., scoring weights).

## Phase 1: Data Model & Schemas (Days 2-4)
**Goal:** Formalize the data structures for governance.

- [x] **Create `orchestra/schemas/task.py`**
    - [x] `TaskSpec`: Pydantic model for task definitions (id, intent, constraints).
    - [x] `TaskSeverity`: Major vs Minor classification enum.
- [x] **Create `orchestra/schemas/jury.py`**
    - [x] `VoteCommit`: Hash(vote + salt).
    - [x] `VoteReveal`: The actual vote and salt.
    - [x] `JurySession`: State of a voting round.
- [x] **Create `orchestra/schemas/audit.py`**
    - [x] `RotationRecord`: Log of agent movement (On-chain <-> Off-chain).
    - [x] `ScrapYardCase`: Forensic file format for retired agents.

## Phase 2: Task Queue Infrastructure (Days 3-5) [COMPLETE]
**Goal:** Implement the "Task via .md" interface and human hard-delete control.

- [x] **Create `orchestra/infra/task_ingestion.py`**
    - [x] File watcher for `.taskmaster/queue/*.md`.
    - [x] Parsing logic to convert MD -> `TaskSpec` objects.
    - [x] Hashing logic to commit task ID to the ledger.
- [x] **Create `orchestra/infra/veto_system.py`**
    - [x] Hook that listens for file deletion events.
    - [x] Logic to immediately invalidate the corresponding Task ID on-chain.
- [x] **Create `orchestra/core/routing.py`**
    - [x] Classifiers to route `TaskSpec` -> Jury (Major) or Sections (Minor).

### Phase 3: Jury System (Days 5-8) [COMPLETE]
**Goal:** Implement the anonymous, commit-reveal voting engine.

- [x] **Create `orchestra/governance/jury.py`**
    - [x] State machine: `Pending` -> `Committed` -> `Revealed` -> `Tallied`.
    - [x] Enforcement of anonymity (reject reveals before commit window closes).
- [x] **Create `orchestra/governance/selection.py`**
    - [x] Logic for "Lawyer Selection": Constraints-based jury composition.
    - [x] Caps enforcement (e.g., max 2 Brass, max 3 Strings per jury).

### Phase 4: Rotation & Isolation (Days 5-8) [COMPLETE]
**Goal:** Dynamic movement of agents between execution and judgment execution.

- [x] **Create `orchestra/infra/sandbox.py`**
    - [x] Container/Environment wrapper to isolate Jurors (no write access).
- [x] **Create `orchestra/governance/rotation.py`**
    - [x] Epoch scheduler for rotation.
    - [x] Snapshot logic: Export Agent State -> Sandbox.
    - [x] Re-entry logic: Import Audit Logs -> Core State.

### Phase 5: Scrap Yard Pipeline (Days 4-7) [COMPLETE]
**Goal:** Forensic analysis of failed or misaligned agents.

- [x] **Create `orchestra/scrapyard/pipeline.py`**
    - [x] Trigger listeners (Consecutive bad votes, Score violation).
    - [x] "Freeze & Extract" logic.
- [x] **Create `orchestra/scrapyard/forensics.py`**
    - [x] Automated replay engine.
    - [x] Failure taxonomy tagger.
- [x] **Create `orchestra/scrapyard/archive.py`**
    - [x] Storage logic for forensic artifacts (permanent history).

### Phase 6: Metrics & Safety Evals (Days 4-6) [COMPLETE]
**Goal:** Automated measurement of institutional health.

- [x] **Create `orchestra/evals/judgment.py`**
    - [x] Metrics: `VetoPrecision`, `FalseApprovalRate`, `DriftScore`.
- [x] **Create `orchestra/core/health.py`**
    - [x] `SilenceDetector`: Alert if disagreement falls below safety threshold.
    - [x] `ContradictionCheck`: Ensure "Brass" section is generating friction.
- [x] **Create `orchestra/tests/adversarial.py`** (Deferred to testing phase)

### Phase 7: UI & Observability (Days 3-5) [COMPLETE]
**Goal:** A read-only dashboard for the Orchestra's state.

- [x] **Create Dashboard Skeleton** (CLI Views `orchestra/views/cli.py`).
- [x] **Create `orchestra/api/routes.py`**
    - [x] Endpoints for: `GET /tasks/queue`, `GET /jury/status`, `GET /scrapyard/cases`.
- [x] **Build Views**
    - [x] Task Queue Inspector.

### Phase 8: Scale Strategy (Ongoing) [COMPLETE]
**Goal:** Responsible growth.

- [x] **Create `orchestra/config/core_32.yaml`**
    - [x] Definition of the initial 32 agent roles.
- [x] **Create `orchestra/core/probes.py`**
    - [x] Logic to spin up ephemeral "Probe Batches" for stress testing.

### Phase 9: Hardening & External Audits (Ongoing) [COMPLETE]
**Goal:** Defense against internal and external threats.

- [x] **Create `orchestra/audit/compliance.py`**
    - [x] Automated scripts to verify code compliance with `SCORE_v1.md`.
- [x] **Setup Red Team CI/CD**
    - [x] Chaos testing pipeline (`orchestra/tests/adversarial.py`).

## EXECUTION COMPLETE
The Orchestra architecture has been successfully scaffolded.
Use `python3 -m orchestra.views.cli` or equivalent to inspect status.

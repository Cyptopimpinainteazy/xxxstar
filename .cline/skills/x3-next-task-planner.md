# Skill: X3 Next Task Planner

## Purpose
Produce exactly 10 executable next tasks with proof criteria. Prevents agents from wandering aimlessly after completion.

## Use When
- At the end of every coding session.
- When updating `docs/X3_NEXT_TASKS.md`.
- When blocked and needing to reprioritize.

## Inputs To Inspect
- `docs/X3_COMPLETION_STATUS.md` — current area statuses.
- `docs/X3_PROOF_LEDGER.md` — recent proof results.
- `docs/X3_NEXT_TASKS.md` — current task list.
- `TODO.md` — project-wide todo.
- Open issues, failing tests, stub reports.

## Checks To Perform
- Each task must have: Why it matters, Files likely involved, Proof command, Done-when criteria.
- Tasks must be concrete and executable — no vague aspirations.
- Tasks are ordered by priority: #1 is next best action.
- All 10 slots filled. If fewer than 10 distinct tasks exist, fill remaining with improvement tasks (add tests, docs, error handling for existing features).

## Proof To Require
- 10 tasks, each with completion criteria.
- Priority ordering justified.

## Output Format
### 1. <task title>
- Why: <reason>
- Files: <paths>
- Proof: <command>
- Done when: <criteria>

(Repeated for tasks 2-10)
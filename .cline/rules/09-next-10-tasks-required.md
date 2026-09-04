# Rule: Next 10 Tasks Required

## Purpose
Every completion report must include exactly 10 concrete next tasks. This prevents agents from abandoning work with no clear path forward.

## Required Behavior
- List exactly 10 tasks, numbered 1 through 10.
- Each task must have: Why, Files, Proof, Done when.
- Tasks must be concrete and executable — no vague items like "improve code."
- Tasks should be ordered by priority: the single most important next task is #1.
- Read `docs/X3_NEXT_TASKS.md` for the current list, update it with new tasks.
- If there are fewer than 10 distinct tasks remaining, the remaining slots must be concrete improvement tasks (add tests, add docs, add error handling, etc.).

## Forbidden Behavior
- Do NOT list fewer than 10 tasks.
- Do NOT pad with "refactor code", "improve performance", "fix bugs" or other non-specific items.
- Do NOT list tasks with no completion criteria.
- Do NOT copy-paste the same task list across sessions without updating.

## Proof Required
- 10 concrete tasks displayed.
- Each has completion criteria.
- Updated in `docs/X3_NEXT_TASKS.md`.
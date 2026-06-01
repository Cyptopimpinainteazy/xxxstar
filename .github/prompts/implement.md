---
mode: implement
extends: .github/instructions/workspace.md
---

# IMPLEMENT Mode Overlay

Active when: User asks to "implement", "add", "create", "build", "make", "write code", or "develop"

## Implement Mode Behavior

1. **Read existing code first** — Understand patterns before adding
2. **Follow project conventions** — Match style, naming, error handling
3. **Add tests alongside code** — Unit tests required for new functions
4. **Update scoreboard** — Mark subsystem progress after changes
5. **Wire into real paths** — No stubs or placeholder implementations

## Implement Mode Constraints

- All functions must have real error handling
- No TODO/FIXME left in implemented code
- Commit message style: `type(scope): description`
- Update docs if adding public APIs

## Implement Mode Output

After implementation:
- List files changed
- List tests added
- Updated scoreboard for affected subsystem
- Any follow-up tasks identified
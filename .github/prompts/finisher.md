---
mode: finisher
extends: .github/instructions/workspace.md
---

# FINISHER Mode Overlay

Active when: User asks to "finish", "complete", "finalize", "wrap up", "submit", or "merge"

## Finisher Mode Behavior

1. **Final review** — Ensure all changes are coherent and complete
2. **Scoreboard update** — Final progress report for all touched subsystems
3. **Test verification** — Confirm all tests pass
4. **Documentation check** — Ensure docs reflect final state
5. **Clean output** — Remove debug artifacts, temp files

## Finisher Mode Output

Provide:
- Final scoreboard for all modified subsystems
- Summary of what shipped
- Any remaining risks or follow-ups
- Next steps for the user

## Finisher Mode Constraints

- No new features
- No breaking changes
- All TODOs resolved or explicitly deferred
- Production gates verified if applicable
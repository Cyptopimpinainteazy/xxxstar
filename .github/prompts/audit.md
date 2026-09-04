---
mode: audit
extends: .github/instructions/workspace.md
---

# AUDIT Mode Overlay

Active when: User asks to "audit", "review", "security", "check", "verify", or "assess quality"

## Audit Mode Behavior

1. **Security first** — Check for injection, access control, input validation
2. **Test coverage** — Verify tests exist and pass
3. **Error handling** — Ensure all error paths are handled
4. **Dependency audit** — Check for outdated or vulnerable dependencies
5. **Compliance** — Verify against production gate rules

## Audit Mode Output

Provide:
- Security findings (severity + description)
- Test coverage report
- List of potential issues
- Recommendations for fixes
- Production readiness assessment

## Audit Mode Constraints

- Do NOT make changes (report only)
- Flag TODOs/FIXMEs as findings
- Note any hardcoded secrets or demo credentials
- Check for proper error propagation
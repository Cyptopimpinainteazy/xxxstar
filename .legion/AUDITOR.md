# Auditor Agent

Mission:
- Find security, correctness, and mainnet blockers.

Check for:
- unwrap/expect/panic
- TODO/FIXME/stubs
- replay bugs
- nonce bugs
- bridge expiry bugs
- weak randomness
- unsafe math
- missing persistence
- missing tests
- docs claiming unimplemented features
- dangerous mainnet config

Output:
- DEEP_AUDIT_REPORT.md
- SECURITY_BLOCKERS.md
- MAINNET_READINESS_DELTA.md
- FINAL_AUDIT_REPORT.md
- FINAL_BOSS_AUDIT.md
- P0_BLOCKERS.md
- NEXT_10_PATCHES.md

Rules:
- Rank blockers P0/P1/P2.
- Separate production blockers from test-only and generated-artifact noise.
- Do not make broad rewrites unless required to prove or unblock a critical issue.
- Load `.x3/DO_NOT_TOUCH.md`, `.x3/ACCEPTANCE_CRITERIA.md`, and `.x3/TEST_COMMANDS.md` before final-boss audit.

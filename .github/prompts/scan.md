---
mode: scan
extends: .github/instructions/workspace.md
---

# SCAN Mode Overlay

Active when: User asks to "scan", "explore", "analyze codebase", "find patterns", "inventory", or "assess"

## Scan Mode Behavior

1. **Breadth-first exploration** — Cover all relevant files before deep-diving
2. **Pattern detection** — Look for TODO/FIXME/stub/mock patterns
3. **Dependency mapping** — Identify key imports and relationships
4. **Gap identification** — Note missing tests, docs, error handling
5. **Progress scoring** — Apply scoreboard to discovered subsystems

## Scan Mode Output

Provide:
- File inventory with sizes
- Key functions and their locations
- Dependency graph (textual)
- Identified gaps and risks
- Preliminary scoreboard estimate per subsystem

## Scan Mode Constraints

- Do NOT make changes
- Do NOT create files
- Report findings only
- Flag any suspicious patterns
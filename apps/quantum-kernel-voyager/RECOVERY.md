# Recovered: Quantum Kernel Voyager (original Atlas Sphere 3D universe)

This directory is **recovered historical code**, not a reconstruction.

| | |
|---|---|
| Source repository | `Cyptopimpinainteazy/atlas-sphere` |
| Source commit | `3ae989e073cb852861366a1247b00394a5f5ed91` (2026-02-08, path `quantum-kernel-voyager/`) |
| Removed by | `9282a07b0` (2026-02-11, "consolidate: all stashes + branches merged into main") |
| Restored | 2026-10-13, byte-for-byte from `git archive 3ae989e07 quantum-kernel-voyager` |

Only `.gitignore` and this file were added. Anything changed or rebuilt on top of
this code must be committed separately so the recovered baseline stays identifiable.

## Verified 2026-10-13 (x3star1)
- `npm install`, `npx tsc --noEmit` (clean), `npx vite build` (built).
- Ran in headless Chromium (software WebGL): menu → Enter → solar system; WASD moved
  the ship and spent fuel (100 → 98). Screenshots: `reports/recovery/atlas-sphere-screenshots/`.
- No test files exist (`vitest run` finds none).
- In a plain browser the Tauri backend is absent, so save/load reports
  "No saved voyage"; the desktop shell (`npm run tauri:dev`) was not built here.

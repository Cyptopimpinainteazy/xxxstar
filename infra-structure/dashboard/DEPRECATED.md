# DEPRECATED — Inferstructor Dashboard

**Status:** Absorbed into `apps/tauri-os` (Tier 1 OS Shell)

**Date:** 2026-06-16

## What Happened

The Inferstructor Dashboard functionality has been consolidated into the X3 Tauri OS (`apps/tauri-os/`) as part of the frontend consolidation effort. The standalone Tauri app defined in `src-tauri/tauri.conf.json` is no longer maintained independently.

## Migration Path

All dashboard components (`AdminDashboard`, `ValidatorControls`, `OrchestraOperationsPanel`, `TpsLeaderboard`) are now available as panels within the Tauri OS shell application:
- **Source:** `apps/tauri-os/src/apps/`
- **Backend:** `apps/tauri-os/src-tauri/src/main.rs`

## What's Still Here

- `src-tauri/` — The original Rust backend. Kept for reference but not built.
- `src/components/` — The original React components. Absorbed into tauri-os.

## Next Steps

- Remove these files entirely when the tauri-os migration is verified working.
- Do NOT add new features to this directory.
- Refer any dashboard issues to `apps/tauri-os/`.
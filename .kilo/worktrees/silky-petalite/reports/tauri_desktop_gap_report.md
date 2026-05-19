# Tauri Desktop Gap Report

Date: 2026-05-05

## Purpose

Identify what the X3 Tauri desktop currently proves, what is missing, and what needs to be wired before it can be treated as a solid release surface.

## Current Reality

- Active desktop app is `apps/x3-desktop`, using Tauri v2, React, Vite, Zustand, and local Tauri IPC commands.
- `apps/tauri-os` currently contains only a `src/` directory in this checkout and does not appear to be the active packaged desktop shell.
- The desktop frontend has a large static application registry in `apps/x3-desktop/src/services/applicationService.ts`.
- The Tauri backend command `get_app_registry` currently returns an empty array, so the frontend always relies on the static fallback registry.
- Local telemetry commands exist for swarm health, network, storage, IDE IPC, system metrics, and IPFS storage.
- Root feature status is tracked in `FEATURE_REGISTRY.toml` and `TESTNET_FEATURE_FLAGS.toml`; desktop status should follow those files.

## Verified

- Tauri configuration exists at `apps/x3-desktop/src-tauri/tauri.conf.json`.
- Tauri backend command handlers are registered in `apps/x3-desktop/src-tauri/src/main.rs`.
- Desktop unit test configuration exists in `apps/x3-desktop/vitest.config.ts`.
- The readiness panel now shows guarded feature modes instead of fabricated release downloads.
- `npm run build` and `npm test` now invoke real Vite/Vitest commands instead of echo-only bypasses.

## Gaps / Risks

- Node dependencies are not installed in this working copy, so TypeScript/Vitest/Vite commands cannot run until `npm install` or `npm ci` completes.
- The backend registry is not wired to the root feature registry, which means app status can drift unless the frontend snapshot is refreshed.
- Several launch targets point to local URLs or internal panels without health gates, dependency checks, or clear degraded states.
- Auto-update UI is not backed by signed releases; it must stay disabled/manual until a real release channel exists.
- Existing E2E tests assume multiple local services and ports are already running, so they are not yet a dependable CI release gate.
- Some older docs under `docs/apps/x3-desktop` and `apps/atlas-sphere-clean/docs` over-claim production readiness and should be reconciled before public release.

## Release Impact

- Local development: partially ready after dependencies are installed.
- Devnet/testnet operator desktop: guarded; usable for selected panels, not yet a release-proof command center.
- Public testnet: blocked until build/test CI, registry wiring, service health gates, and E2E startup orchestration are proven.
- Production rollout: blocked; signed update channel, security review, and honest app readiness gates are still required.

## Next Required Work

1. Wire `get_app_registry` to a real generated registry derived from `FEATURE_REGISTRY.toml`.
2. Add launch-time health checks for every URL/process-backed app.
3. Create a deterministic desktop E2E harness that starts only the services needed for the test.
4. Reconcile stale desktop documentation with this report and the root feature registry.
5. Add signed release/update implementation only after the desktop build is green in CI.
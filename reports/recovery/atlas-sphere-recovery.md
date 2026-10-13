# Atlas Sphere 3D universe — recovery report

**Result: the original implementation was found in history, restored, built and run.**
It is restored, not reconstructed: `apps/quantum-kernel-voyager/` is byte-identical to
`atlas-sphere@3ae989e07:quantum-kernel-voyager/` (see its `RECOVERY.md`).

## The original: Quantum Kernel Voyager
`package.json`: *"Quantum Kernel Voyager Desktop Edition — 3D blockchain control interface
for the Atlas Sphere ecosystem"*. Tauri v2 + Three.js 0.168 + simplex-noise.

| Required behaviour | Evidence (file @ 3ae989e07) |
|---|---|
| Pilotable spacecraft | `src/game/VoyagerShip.ts` — input, thrust, boost, velocity/drag, banking, thruster particles, fuel and hull; `src/main.ts` binds W/A/S/D, Space, Shift |
| Navigable 3D universe | `src/scene/SolarSystemBuilder.ts`, `CameraController.ts`, `OrbitCalculator.ts`, nebula/hologram shaders |
| Subsystem destinations | `src/config/solar-system-config.ts`: ATLAS Token, Block Explorer, Atlas DEX, Swarm Hub, GPU Cluster (`/x3/swarm/gpu`), Wallet, Governance, Treasury, Analytics, ETH Bridge — each with a `landingTarget` |
| Enterable destinations | game states `menu → solar_system ↔ exploring ↔ warping ↔ inspecting ↔ surface`; `LandingPrompt` ("Press F to Land") and `PlanetDetail` UI |
| Validators | HUD "Validators 12 / Peers 8", `validatorCount: 12` in the config |
| Dynamic world | `src/game/WorldManager.ts` — "procedurally generated blockchain worlds" by seed |
| Backend/telemetry | `src-tauri/src/{chains,x3_integration,keystore,commands}.rs`; `src/adapters/{AtlasSphere,Ethereum,LocalDev}Adapter.ts` |

**Not present / still to do** (documented, not claimed): AI agents are not separate
entities (only the Swarm Hub/GPU Cluster planets); validator and TPS figures are static
until an adapter is pointed at a live X3 node; no tests; the Tauri desktop shell was not
built here; the spacecraft is a procedural mesh, not one of the `3d/` model files.

## Other candidates (lower fidelity)
| Candidate | Where | What it is |
|---|---|---|
| `apps/x3-desktop/src/lib/three/SceneManager.ts`, `ThreeScene.tsx`, `WorldSimViewer.tsx` | atlas-sphere (also `apps/atlas-desktop`) | animated 7-layer background (starfield, nebula, grid) — not pilotable |
| `x3fronend/src/components/Landing.jsx`, `x3fronend/x3star-landing.html` | atlas-sphere @ d5ec46dc2 | 3D landing page |
| `3d/` (Spaceship_000000.obj, portal.obj, RefractionJet, Vulcan Dkyr Class, space HDRI) | x3-atomic-star @ 143f7a6bb | third-party models; licences to be checked before use |

## Next
1. Point `LocalDevAdapter` at a local X3 node so block height, validators and TPS are live.
2. Add agents, validators and GPUs as live entities; keep a demo mode when no node answers.
3. Add tests (vitest) for the game state machine and planet registry.

# Quantum Kernel Voyager — Desktop Edition

A Tauri v2 + Three.js native desktop application for exploring procedurally generated
blockchain worlds. Built on the Atlas Sphere operator infrastructure.

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                     Tauri v2 Shell                        │
│  ┌─────────────────────────────────────────────────────┐ │
│  │              WebView (Chromium)                      │ │
│  │  ┌───────────┐  ┌───────────┐  ┌────────────────┐  │ │
│  │  │  Three.js  │  │   UI DOM   │  │  EventBus IPC  │  │ │
│  │  │  Scene     │  │  Overlay   │  │  ↕ Tauri cmds  │  │ │
│  │  └─────┬─────┘  └─────┬─────┘  └───────┬────────┘  │ │
│  │        │               │                │            │ │
│  │  ┌─────┴───────────────┴────────────────┴────────┐  │ │
│  │  │              Game State Machine                │  │ │
│  │  │  menu → exploring ↔ warping ↔ inspecting       │  │ │
│  │  │              ↕ paused                           │  │ │
│  │  └───────────────────────────────────────────────┘  │ │
│  └─────────────────────────────────────────────────────┘ │
│  ┌─────────────────────────────────────────────────────┐ │
│  │              Rust Backend (src-tauri/)               │ │
│  │  Keystore (AES-256-GCM) │ ChainManager │ X3 types  │ │
│  └─────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────┘
```

## Requirements

| Tool        | Version  |
|-------------|----------|
| Node.js     | ≥ 18     |
| Rust        | ≥ 1.78   |
| Tauri CLI   | 2.x      |

Platform dependencies for Tauri (Linux):

```bash
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev
```

## Quick Start

```bash
# Install JS deps
npm install

# Development mode (hot-reload frontend + Rust rebuild)
npm run tauri:dev

# Production build
npm run tauri:build
```

The dev server starts on `http://localhost:1420`. Tauri opens a native window at 1440×900.

## Project Structure

```
quantum-kernel-voyager/
├── package.json              # JS deps + scripts
├── tsconfig.json             # TypeScript strict config
├── vite.config.ts            # Vite dev server (port 1420)
├── index.html                # Shell HTML
│
├── src/
│   ├── main.ts               # App entry — bootstraps all systems
│   │
│   ├── types/
│   │   ├── chain.ts          # Blockchain types (mirrors x3_operator)
│   │   ├── game.ts           # Game state, world generation, economy
│   │   └── scene.ts          # Three.js entity kinds + render config
│   │
│   ├── scene/
│   │   ├── SceneManager.ts   # Renderer, camera, lighting, entity registry
│   │   ├── CameraController.ts # Orbit + free-flight, WASD, transitions
│   │   ├── EntityFactory.ts  # Mesh creation for 8 entity kinds
│   │   ├── ProceduralGenerator.ts # Deterministic world gen (xoshiro128**)
│   │   ├── ParticleSystem.ts # GPU particles (thruster, tunnel, etc.)
│   │   ├── Animations.ts     # Warp transitions + entity idle anims
│   │   ├── InteractionManager.ts # Raycaster click/hover/context menu
│   │   ├── LODManager.ts     # 3-level LOD + frustum culling
│   │   └── PostProcessing.ts # Bloom + FXAA pipeline
│   │
│   ├── adapters/
│   │   ├── AtlasSphereAdapter.ts # Substrate WS adapter (simulated fallback)
│   │   ├── EthereumAdapter.ts    # JSON-RPC adapter (simulated fallback)
│   │   ├── LocalDevAdapter.ts    # In-memory mock chain (3s blocks)
│   │   └── SyncQueue.ts         # Offline-first tx queue (IndexedDB)
│   │
│   ├── game/
│   │   ├── GameState.ts      # Discriminated-union state machine
│   │   ├── WorldManager.ts   # Seeds → entities, coordinates scene
│   │   ├── EconomyManager.ts # Quantum Crystals, upgrades, fuel costs
│   │   ├── ArtifactManager.ts # Discovery, scanning, claiming
│   │   └── VoyagerShip.ts    # Ship physics, fuel/health, thruster FX
│   │
│   ├── ui/
│   │   ├── Overlay.ts        # Root UI controller
│   │   ├── TopBar.ts         # HUD (hull, fuel, crystals, worlds)
│   │   ├── SidePanel.ts      # Tabbed panel (Nav, Inv, Chain, Ship)
│   │   ├── ModalSystem.ts    # Stacked modals with confirm/alert
│   │   └── ContextMenu.ts    # Right-click menus
│   │
│   ├── ipc/
│   │   ├── tauri.ts          # Typed Tauri invoke wrappers
│   │   └── events.ts         # EventBus pub/sub (8 channels)
│   │
│   ├── storage/
│   │   └── IndexedDBStore.ts # Key-value store (settings/cache/journal)
│   │
│   ├── shaders/
│   │   ├── quantum-distortion.frag # Radial distortion + chromatic aberration
│   │   ├── nebula.frag       # Volumetric nebula with fBM
│   │   └── hologram.frag     # Scan lines + glitch FX
│   │
│   └── styles/
│       ├── main.css          # CSS custom properties, layout, reset
│       ├── overlay.css       # HUD, side panel, bottom bar
│       └── components.css    # Modals, buttons, tooltips
│
└── src-tauri/
    ├── tauri.conf.json       # Tauri config (CSP, window, plugins)
    ├── Cargo.toml            # Rust deps
    ├── build.rs
    └── src/
        ├── main.rs           # Tauri bootstrap + plugin registration
        ├── commands.rs       # 10 IPC command handlers
        ├── error.rs          # AppError type (8 variants)
        ├── keystore.rs       # AES-256-GCM + Argon2id encrypted store
        ├── chains.rs         # Chain connection manager
        └── x3_integration.rs # Rust mirrors of x3_operator types
```

## Systems Overview

### Scene System
Three.js 0.168 with direct scene-graph control (no React wrapper). WebGLRenderer with
ACES Filmic tone mapping, logarithmic depth buffer, PCFSoftShadowMap. Post-processing
via EffectComposer (Bloom + FXAA).

### Procedural Generation
Worlds are deterministically generated from 256-bit seed strings using `xoshiro128**`
PRNG. Heightmaps use 6-octave simplex noise. Biomes selected by temperature/moisture
mapping. Artifacts scattered via 3D noise with rarity tiers (Common 60%, Uncommon 25%,
Rare 10%, Epic 4%, Legendary 1%).

### Chain Adapters
Adapters implement a common `ChainAdapter` interface. All adapters gracefully degrade
to simulated mode when the network is unavailable:

- **AtlasSphereAdapter** — Substrate WebSocket (ws://127.0.0.1:9944)
- **EthereumAdapter** — JSON-RPC (http://127.0.0.1:8545)
- **LocalDevAdapter** — Pure in-memory mock (3-second blocks)

### Sync Queue
Offline-first transaction queue persisted in IndexedDB. Transactions retry with
exponential backoff (1s × 2^n, max 5 attempts). Events emitted at each lifecycle
stage: `queued → submitting → confirmed | failed`.

### Game Mechanics
Quantum Crystals economy: mine artifacts for yields scaled by rarity (10–1000).
Spend crystals on ship upgrades (fuel tank, scanner, hull, warp drive). Warp
between worlds costs fuel (base 5 + distance × 0.1).

### Security
Rust backend encrypts all private keys with AES-256-GCM. Key derivation via
Argon2id (65536 KiB memory, 3 iterations, parallelism 4). Keys never cross
the IPC boundary — signing happens server-side.

## Controls

| Key       | Action                       |
|-----------|------------------------------|
| W/A/S/D   | Ship movement                |
| Space     | Ascend                       |
| Shift     | Boost (2.5× speed, 4× fuel) |
| F         | Toggle camera mode           |
| Escape    | Pause / unpause              |
| Enter     | Start game (from menu)       |

## Configuration

Environment variables:

| Variable           | Default | Description               |
|--------------------|---------|---------------------------|
| `VOYAGER_LOG_LEVEL`| `info`  | Rust tracing filter level |

## License

Part of the Atlas Sphere project.

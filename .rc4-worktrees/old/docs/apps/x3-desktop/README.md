# X3 Desktop

Dark-themed desktop environment for the X3 Chain blockchain command center. Built with **React 18**, **Three.js**, **Zustand**, **Tailwind CSS**, and **Tauri v2**.

## Features

- **3D Interactive Eyeball** — cursor-tracking eyeball using quaternion SLERP interpolation, procedural GLSL iris shader, corneal reflections
- **Desktop Window Manager** — draggable, resizable, minimizable floating windows with z-index management, keyboard shortcuts (Alt+Tab, Alt+F4, Ctrl+D)
- **Application Launcher** — responsive icon grid with 16 pre-registered apps from the monorepo, hover glow effects, running indicators
- **Dark Orange Theme** — WCAG-compliant dark palette with `#ff6b35` / `#ff8c42` orange accents, glass-morphism panels
- **Taskbar** — running application buttons, clock, system tray, window focus/restore
- **IPC Service Layer** — Tauri invoke wrapper with retry logic, exponential backoff, timeout management, request logging
- **Error Boundary** — global error catching with localStorage persistence for diagnostics
- **Context Menu** — right-click desktop operations: theme toggle, icon size, show desktop

## Quick Start

```bash
cd apps/x3-desktop

# Install dependencies
npm install

# Development server (browser-only, no Tauri runtime needed)
npm run dev

# Type-check
npm run typecheck

# Run tests
npm test

# Build for production
npm run build
```

### With Tauri Desktop Runtime

```bash
# Install Tauri CLI
cargo install tauri-cli

# Run desktop app with hot-reload
npm run tauri dev

# Build distributable
npm run tauri build
```

### URL Panels: Required Local Dev Servers

Start these in separate terminals when using URL-type desktop apps:

```bash
# Ollama Code Reviewer (http://localhost:5175)
cd ../../ollama-code-reviewer && npm run dev

# 3aiXchange DEX (http://localhost:5176)
cd ../3ai/dex/frontend && npm run dev

# X3 App Store (http://localhost:3001)
cd ../../x3-app-store/frontend && npm run dev

# Blockchain TPS Tester (http://localhost:3020)
cd ../../infra-structure/services/blockchain-tps && PORT=3020 npm start

# Foundry/Hardhat GUI (http://localhost:8787)
cd ../../tools/foundry-hardhat-gui && python3 server.py --port 8787

# GPU Validator Dashboard (http://localhost:8080)
cd ../../cross-chain-gpu-validator && python -m cross_chain_gpu_validator.cli dashboard --port 8080

# Autonomic Control Plane (http://localhost:8080/dashboard.html)
cd ../../swarm && python -m swarm.autonomic

# GPU Swarm Node Admin (http://localhost:9101)
cd ../../crates/gpu-swarm && cargo run
```

Notes:
- `autonomic-control-plane` and `gpu-validator-dashboard` both use port `8080`; run one at a time or change one port.
- If an iframe app is unreachable, the desktop now shows app-specific startup hints with command + working directory.

### Network selector & environment variables

- Use the **Network** dropdown in the Top‑right `TopNavBar` to switch between `Local`, `Testnet` and `Mainnet` at runtime — the selection persists to `localStorage` and the Substrate client will reconnect automatically.
- You can override endpoints with environment variables in `.env` / `.env.local`:
  - `VITE_RPC_WS` / `VITE_RPC_HTTP` — primary public RPC endpoints
  - `VITE_RPC_WS_LOCAL` / `VITE_RPC_HTTP_LOCAL` — local node endpoints (127.0.0.1)
- Default public fallbacks: `wss://rpc.x3-chain.io:9944` and `https://rpc.x3-chain.io:9944`.

## Architecture

```
src/
├── components/
│   ├── eyeball/         # Three.js scene, tracking hook, GLSL shaders
│   ├── desktop/         # Desktop, WindowManager, Taskbar
│   ├── icons/           # IconGrid, ApplicationIcon
│   ├── theme/           # ThemeProvider, useTheme
│   └── common/          # Modal, Tooltip, ContextMenu
├── hooks/               # useApplicationRegistry, useWindowManager, useCursorTracker
├── services/            # ipcService, applicationService, fileSystemService
├── stores/              # Zustand: desktopStore, applicationStore, themeStore
├── types/               # TypeScript interfaces: Application, Window, IPC
├── utils/               # geometry, localStorage, eventEmitter
├── styles/              # Tailwind globals, CSS custom properties
├── App.tsx              # Root component with ErrorBoundary
└── main.tsx             # Vite entry point
```

### Key Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| State Management | Zustand | Lightweight, no boilerplate, built-in persistence via middleware |
| 3D Graphics | Three.js + @react-three/fiber | React integration, declarative scene graph, mature ecosystem |
| Styling | Tailwind CSS 3 | Utility-first, custom theme support, dark mode built-in |
| Gaze Tracking | Quaternion SLERP | Prevents gimbal lock, smooth interpolation, correct 3D rotation |
| IPC | Tauri invoke with fallback | Works in browser during dev (stubs), production uses native IPC |
| Window Manager | Custom implementation | Full control over z-index, cascading, persistence, keyboard shortcuts |

### Application Registry

The app ships with 16 pre-registered applications matching the monorepo structure:

| App | Category | Launch |
|-----|----------|--------|
| Block Explorer | blockchain | URL |
| Wallet | blockchain | URL |
| DEX | defi | URL |
| Analytics | analysis | URL |
| Swarm Dashboard | service | URL |
| Command Center | utility | URL |
| Funding Automator | defi | Tauri |
| X3 Intelligence | security | URL |
| Dev Dashboard | development | URL |
| 3AI Assistant | utility | Tauri |
| Governance | blockchain | URL |
| Launchpad | defi | URL |
| Unified Dashboard | analysis | URL |
| Quantum Voyager | utility | Tauri |
| Phase 5 Panel | service | URL |
| HTLC Manager | blockchain | Tauri |

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Alt+Tab | Cycle through open windows |
| Alt+Shift+Tab | Reverse cycle windows |
| Alt+F4 | Close active window |
| Ctrl+D | Minimize all (show desktop) |
| Right-click | Desktop context menu |
| Double-click icon | Launch application |
| Single-click icon | Show tooltip |

## Testing

```bash
# Unit tests (45 tests across 3 files)
npm test

# Watch mode
npm run test:watch

# Coverage
npm run test:coverage
```

Test coverage targets:
- **Eyeball tracking math**: cursor→NDC, quaternion computation, SLERP, dilation
- **Window manager**: open, close, focus, minimize, maximize, move, resize, cascade
- **Application registry**: store operations, process lifecycle, running state

## Theme

CSS custom properties and Tailwind config provide consistent theming:

| Token | Value | Usage |
|-------|-------|-------|
| `--color-primary` | `#1a1a1a` | Main background |
| `--color-accent` | `#ff6b35` | Primary orange accent |
| `--color-accent-secondary` | `#ff8c42` | Hover / interactive orange |
| `--color-text` | `#e0e0e0` | Primary text |
| Shadow: `glow` | `0 0 20px rgba(255,139,66,0.5)` | Icon hover glow |

## Browser / Runtime Compatibility

- Tauri v2 with wry webview
- Chromium 100+ (WebGL 2.0, ES2022, CSS Grid)
- Min resolution: 1920×1080
- Tested: Windows 10+, macOS 12+, Ubuntu 20.04+

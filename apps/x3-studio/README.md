# X3 Studio

**Production-grade desktop IDE for the X3 blockchain ecosystem.**

Built with Electron + React + TypeScript + Monaco Editor. Designed for x3-lang development, cross-VM blockchain engineering, validator operations, relayer development, proof-ledger workflows, and testnet/mainnet readiness.

## Features

- **Monaco Editor** — full code editing with syntax highlighting for x3-lang, Solidity, Rust, TypeScript
- **X3 Language Support** — tokenizer, syntax highlighting, snippets, validation, templates
- **File Explorer** — browse and edit workspace files
- **Integrated Terminal** — xterm.js based multi-tab terminal
- **X3 Control Center** — real status dashboard with build/test/scanner/scoreboard
- **Proof Mode** — every command generates proof artifacts in x3-proof/
- **Anti-Fake-Code Scanner** — detects TODO, FIXME, HACK, stub, mock, placeholder patterns
- **Security Scanner** — finds exposed keys, secrets, insecure patterns
- **Scoreboard System** — category-based scoring with PASS/FAIL/PARTIAL/BLOCKED
- **AI Agent Panel** — connects to Ollama/LM Studio/OpenAI/Anthropic for AI-assisted development
- **Cross-VM Adapter Panel** — track EVM, SVM, BTC, Substrate, CosmWasm, MoveVM adapter status
- **Relayer Panel** — relayer service detection and monitoring
- **Validator Panel** — validator operations and chain spec management
- **Proof Ledger** — browse proof artifacts
- **Chain Health Monitor** — RPC health checks for X3, Ethereum, Solana
- **Launch Cockpit** — testnet/mainnet readiness assessment
- **Git Integration** — branch, status, diff, commit, log
- **Project Detection** — auto-detect Rust, Node.js, Hardhat, Foundry, Anchor, Substrate projects
- **Settings** — full configuration with .x3studio/settings.json persistence
- **Git Diff Tracking** — proof mode captures changed files

## Quick Start

```bash
# Install dependencies
pnpm install

# Build electron main process
pnpm build:electron

# Build renderer
pnpm build:renderer

# Run in development mode (two terminals)
pnpm dev:renderer   # Terminal 1: Vite dev server on :5173
pnpm dev:electron   # Terminal 2: Electron app

# Or use:
pnpm dev            # Runs both with concurrently
```

## Architecture

```
x3-studio/
├── electron/
│   ├── main.ts          # Electron main process, IPC handlers
│   └── preload.ts       # Context bridge API
├── src/
│   ├── App.tsx          # Root IDE shell
│   ├── App.css          # Dark theme styles
│   ├── types.ts         # TypeScript definitions
│   ├── store/           # Zustand stores
│   ├── components/
│   │   ├── Sidebar.tsx        # Activity bar with 18 panels
│   │   ├── StatusBar.tsx      # Git branch, dirty files, score
│   │   ├── BottomPanel.tsx    # Terminal/Problems/Output
│   │   ├── editor/            # Monaco Editor wrapper
│   │   ├── explorer/          # File tree
│   │   ├── terminal/          # xterm.js integration
│   │   └── panels/            # 15 sidebar panels
│   ├── services/
│   │   ├── proofGenerator.ts  # Command execution + proof artifacts
│   │   ├── fakeCodeScanner.ts # Pattern scanning
│   │   └── scoreboardGenerator.ts
│   └── x3/                   # X3 language support
├── prompts/               # AI Agent system prompts
├── tests/                 # Vitest test suite
└── docs/                  # Documentation
```

## Status

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for what works, what's partial, and what's blocked.

# Architecture

## What Works (FULLY IMPLEMENTED)

- ✅ Electron main process with IPC handlers
- ✅ React + Vite renderer with TypeScript
- ✅ Monaco Editor with multi-tab support
- ✅ File Explorer with tree view + context menu
- ✅ xterm.js integrated terminal (multiple tabs)
- ✅ X3 Control Center dashboard with real status
- ✅ Proof Mode with command execution + artifact generation
- ✅ Anti-Fake-Code Scanner (TODO, FIXME, HACK, stub, mock, placeholder, etc.)
- ✅ Security Scanner (secrets, keys, insecure patterns)
- ✅ Scoreboard System with PASS/FAIL/PARTIAL/BLOCKED
- ✅ AI Agent panel (Ollama/LM Studio/OpenAI/Anthropic)
- ✅ Cross-VM Adapter Panel (6 adapters tracked)
- ✅ Validator Panel (chain spec + config detection)
- ✅ Relayer Panel (directory detection)
- ✅ Proof Ledger (browse x3-proof/ artifacts)
- ✅ Chain Health Monitor (RPC health checks)
- ✅ Launch Cockpit (testnet/mainnet readiness %)
- ✅ Git Integration (status, branch, log, commit, diff)
- ✅ Project Detection (auto-detect Rust, Node.js, etc.)
- ✅ Settings panel + .x3studio/settings.json persistence
- ✅ X3 language tokenizer, syntax highlighting, validation
- ✅ Sidebar with 18 activity bar panels
- ✅ Status Bar (branch, dirty files, score)
- ✅ Bottom Panel (terminal, problems, output, proof)
- ✅ Dark theme with X3 branding
- ✅ 7 test files (Vitest)

## What is PARTIAL

- ⚠️ **Diff Viewer** — Basic diff is available in proof mode but no side-by-side viewer
- ⚠️ **Security Scanner** — Pattern-based scanning works; no deep semantic analysis
- ⚠️ **Debugger** — No debugger integration (Rust, Node)
- ⚠️ **Command Palette** — No Ctrl+Shift+P command palette UI yet
- ⚠️ **Search in Files** — Text search in workspace UI not built (IPC supports glob)
- ⚠️ **Split Editor** — Single editor view only
- ⚠️ **Terminal Resize** — xterm.js renders but resize events don't properly PTY-resize
- ⚠️ **Live Chain Connection** — Health checks run on demand, not auto-polling
- ⚠️ **Hardware Wallet Support** — Adapter panel shows EVM/SVM but no live wallet connection

## What is BLOCKED

- ❌ **Real Debugger (LLDB/GDB)** — No debug adapter protocol integration
- ❌ **Native node-pty** — Falls back to child_process.spawn (no true PTY)
- ❌ **Real Mainnet Deployment** — Blocked until node connection established
- ❌ **Real-time Validator Stream** — Requires running node with WebSocket

## State Management

Zustand stores:
- `workspaceStore` — workspace path, detection, git status
- `editorStore` — tabs, dirty tracking, file content
- `proofStore` — proof records, running state
- `scoreboardStore` — categories, total score
- `scannerStore` — findings, scanning state
- `settingsStore` — all settings
- `terminalStore` — terminal instances
- `layoutStore` — sidebar/bottom panel state

## IPC Architecture

```
Renderer (React) ←→ Preload (contextBridge) ←→ Main Process (Node.js)
                      │
                      ├── fs:*          — File system operations
                      ├── shell:*       — Command execution
                      ├── terminal:*    — PTY management
                      ├── git:*         — Git operations
                      ├── scanner:*     — File scanning
                      ├── dialog:*      — Native dialogs
                      └── app:*         — Environment info
```

## Testing

```bash
pnpm test    # Run Vitest test suite
```

Tests: x3 tokenizer, fake code scanner, proof report, scoreboard, settings.

# Development

## Setup

```bash
pnpm install
```

## Dev Mode

```bash
# Terminal 1: Start Vite dev server
pnpm dev:renderer

# Terminal 2: Start Electron
pnpm dev:electron
```

## Build

```bash
pnpm build:electron   # Compile main process
pnpm build:renderer   # Build renderer
pnpm build            # Both
```

## Test

```bash
pnpm test             # Run Vitest
pnpm test:watch       # Watch mode
```

## Project Structure

```
src/
  store/               # Zustand state management
  components/
    editor/            # Monaco Editor
    explorer/          # File Explorer
    terminal/          # xterm.js integration
    panels/            # 15 sidebar panels
    common/            # Shared UI components
  services/            # Core business logic
  x3/                  # X3 language support
tests/                 # Vitest tests
electron/              # Main + preload
prompts/               # AI agent prompts
docs/                  # Documentation
```

## Adding a New Panel

1. Create component in `src/components/panels/`
2. Add panel ID to `PanelId` type in `src/types.ts`
3. Add sidebar icon in `Sidebar.tsx`
4. Add case in `App.tsx` renderSidebarContent()

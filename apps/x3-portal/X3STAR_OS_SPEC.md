# x3Star OS — Complete System Specification

> **CLASSIFICATION: PRODUCTION-GRADE**  
> **VERSION: 1.0.3**  
> **STATUS: IMPLEMENTATION COMPLETE**

---

## 🔒 NON-NEGOTIABLES (READ FIRST)

```
THIS IS AN OPERATING SYSTEM, NOT A WEBSITE.

- Desktop NEVER scrolls
- All content lives in WINDOWS
- Motion is INFORMATIVE, not decorative
- Copy is BLUNT, TECHNICAL, UNAVOIDABLE
- If it feels "pretty" instead of "useful," it is WRONG
```

---

## 📐 EXACT WINDOW LAYOUT & INTERACTION MAP

### Global Coordinate System

| Layer | Z-Index | Description |
|-------|---------|-------------|
| System Bar | 100 | Always visible, never occluded |
| Dock | 90 | Left edge navigation |
| Active Windows | 50-80 | Dynamic z-stacking on focus |
| Inactive Windows | 40 | Gaussian blur applied |
| Desktop Grid | 0 | Visual reference only |

### Window Default Positions

```
┌─────────────────────────────────────────────────────────────────────┐
│ SYSTEM BAR (h=48px)                                                 │
├────┬────────────────────────────────────────────────────────────────┤
│    │                                                                │
│ D  │    ┌─────────────────────────────────────────┐                 │
│ O  │    │ Terminal.app (600x400)                  │                 │
│ C  │    │ position: (96, 72)                      │                 │
│ K  │    │ DEFAULT OPEN ON BOOT                    │                 │
│    │    └─────────────────────────────────────────┘                 │
│ 64 │                                                                │
│ px │         ┌──────────────────────────────────────────────────┐   │
│    │         │ Execution Engine.app (800x500)                   │   │
│    │         │ position: (350, 90)                              │   │
│    │         │ HEART OF THE SYSTEM                              │   │
│    │         └──────────────────────────────────────────────────┘   │
│    │                                                                │
│    │    ┌────────────────────┐                                      │
│    │    │ VM Manager (500x400)│     ┌──────────────────────────┐    │
│    │    │ pos: (120, 100)     │     │ Atomic Layer (400x500)   │    │
│    │    └────────────────────┘     │ pos: (300, 120)          │    │
│    │                               └──────────────────────────┘    │
└────┴────────────────────────────────────────────────────────────────┘
```

### Window Animation Rules

```typescript
// OPEN
{
  initial: { opacity: 0, scale: 0.95, y: 10 },
  animate: { opacity: 1, scale: 1, y: 0 },
  transition: { duration: 0.15, ease: 'linear' }
}

// CLOSE
{
  exit: { opacity: 0, scale: 0.95, y: 10 },
  transition: { duration: 0.15, ease: 'linear' }
}

// FOCUS CHANGE
// Instant z-index shift, no animation
// Inactive windows receive blur(1px) + opacity: 0.8
```

### Window State Machine

```
CLOSED ──[dock click / hotkey]──► OPEN (FOCUSED)
                                      │
                                      ▼
                                  [click outside]
                                      │
                                      ▼
                            OPEN (UNFOCUSED / BLURRED)
                                      │
                                      ▼
                              [close button / ESC]
                                      │
                                      ▼
                                   CLOSED
```

---

## 📝 SYSTEM COPY (AUTHORITATIVE)

### Boot Sequence

```
x3Star OS v1.0.3
Initializing execution kernel…
Loading VM abstraction layer…
Mounting atomic state machine…
Synchronizing cross-chain consensus…
Calibrating MEV protection filters…
System ready.
```

**Timing:** 15-25ms per character, 200ms pause between lines, hard cut to desktop.

### Terminal Commands

| Command | Output |
|---------|--------|
| `status` | System health, network state, MEV shield status |
| `vms` | Available virtual machines with technical specs |
| `atomic` | Atomic execution protocol constraints |
| `why` | Core philosophy (intent vs transaction) |
| `blocks` | Recent block activity |
| `help` | Command reference |
| `clear` | Clear terminal output |

### Terminal Output Tone

```
GOOD:
"Determinism is not optional."
"Either everything happens… Or nothing does."
"Other chains execute transactions. This executes intent."

BAD:
"Revolutionary cross-chain synergy unlocks the future"
"🚀 Amazing new features coming soon!"
"Join our community of innovators"
```

### System Messages

| Context | Message |
|---------|---------|
| Connection established | `[x3Star] Substrate connection established` |
| Connection failed | `[x3Star] Connection failed, using mock data` |
| Block confirmed | `Block #X confirmed (Y txs, Z ms)` |
| Block finalized | `✓ FINALIZED` |
| Atomic success | `COMMITTED` |
| Atomic failure | `ROLLED BACK` |
| Reorg detected | `⚠ CHAIN REORGANIZATION` |
| Execution halt | `⚠ EXECUTION HALTED` |

---

## ⌨️ KEYBOARD SHORTCUTS

| Shortcut | Action |
|----------|--------|
| `⌘/Ctrl + T` | Toggle Terminal |
| `⌘/Ctrl + E` | Toggle Execution Engine |
| `⌘/Ctrl + V` | Toggle VM Manager |
| `ESC` | Close focused window |

**Implementation:**
```typescript
useKeyboardShortcuts({
  'cmd+t': () => toggleWindow('terminal'),
  'cmd+e': () => toggleWindow('execution'),
  'cmd+v': () => toggleWindow('vms'),
  'escape': () => closeFocusedWindow(),
});
```

---

## 📊 EXECUTION ENGINE (BLOOMBERG-STYLE)

### Header Stats Panel

```
┌──────────────┬──────────────┬──────────────┬──────────────┬──────────────┐
│ THROUGHPUT   │ PENDING      │ FINALITY     │ GAS AVG      │ MEV BLOCKED  │
│ 14,562 TPS   │ 847          │ 1.2s         │ 21 gwei      │ 99.7%        │
│ +2.3%        │ -12          │ ±0.1s        │ +5%          │              │
└──────────────┴──────────────┴──────────────┴──────────────┴──────────────┘
```

### Block Table Columns

| Column | Data Type | Color Coding |
|--------|-----------|--------------|
| BLOCK | Number | White |
| VM | Tag | VM-specific color |
| TXS | Count | Grey |
| GAS | String | Grey |
| TIME | Seconds | Grey |
| STATUS | Enum | Green=finalized, Blue=confirmed, Yellow=pending, Red=failed |

### Block Detail Panel

Right sidebar (256px width) showing:
- Block number
- Full hash (truncated)
- Virtual machine
- Transaction count
- Status with icon

### Execution Timeline

Bottom bar (80px height) with:
- Bar chart of recent 40 blocks
- Height = transaction count
- Color = VM type
- Opacity = status (failed = 30%)

---

## 🔗 LIVE DATA BINDING

### Data Sources

```typescript
interface X3StarConfig {
  // Substrate/Polkadot RPC
  substrateWs: 'ws://127.0.0.1:9944';
  
  // EVM JSON-RPC
  evmRpc: 'http://127.0.0.1:9944';
  
  // Prometheus metrics
  prometheusEndpoint: 'http://127.0.0.1:9615/metrics';
  
  // Polling intervals
  blockPollInterval: 6000;  // 6 second blocks
  metricsPollInterval: 5000;
}
```

### React Hooks

```typescript
// Get live blocks
const blocks = useBlocks(20);

// Get network status
const status = useNetworkStatus();

// Get VM statuses
const vms = useVMStatuses();
```

### Data Flow

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│ Substrate Node  │────►│ X3StarDataProvider│────►│ React Components│
│ (WS/RPC)        │     │ (Singleton)       │     │ (via hooks)     │
└─────────────────┘     └──────────────────┘     └─────────────────┘
         │                       │
         │                       │ Mock fallback
         ▼                       ▼ when disconnected
┌─────────────────┐     ┌──────────────────┐
│ Prometheus      │     │ Generated Data   │
│ (Metrics)       │     │ (Realistic)      │
└─────────────────┘     └──────────────────┘
```

### Never Lie Policy

- If RPC is disconnected, show `DISCONNECTED` status clearly
- Mock data is labeled as mock in console
- Error states are always visible to user
- No fake "loading" animations that hide missing data

---

## ⚠️ FAILURE MODE DRAMATIZATION

### Reorg (Chain Reorganization)

```typescript
{
  title: 'CHAIN REORGANIZATION',
  subtitle: 'Block reverted. State rolling back.',
  color: '#FFAA00',  // Warning amber
}
```

**UI Behavior:**
1. Full-screen overlay (90% black opacity)
2. Title pulses with animate-pulse
3. All windows freeze behind overlay
4. Click anywhere to dismiss
5. Affected blocks marked as `failed` in table

### Execution Halt

```typescript
{
  title: 'EXECUTION HALTED',
  subtitle: 'Consensus failure detected.',
  color: '#FF5555',  // Error red
}
```

**UI Behavior:**
1. System bar flashes red
2. Dock icons dim
3. All processing indicators freeze
4. Manual intervention required

### Atomic Abort

```typescript
{
  title: 'ATOMIC ABORT',
  subtitle: 'Transaction group collapsed. No state changed.',
  color: '#FF5555',
}
```

**UI Behavior in Atomic Layer:**
1. All progress bars freeze
2. Red overlay slides over progress
3. Progress resets to 0 after 500ms
4. "ROLLED BACK" status displayed

---

## 🎨 VISUAL DESIGN TOKENS

### Colors

```css
--bg-primary: #050505;
--bg-secondary: #0a0a0a;
--bg-elevated: #0d0d0d;
--border: #1a1a1a;
--text-primary: #ffffff;
--text-secondary: #888888;
--text-muted: #666666;
--text-disabled: #444444;
--accent-primary: #00FF00;
--accent-evm: #627EEA;
--accent-svm: #00FFA3;
--accent-x3vm: #FF6B00;
--accent-btc: #F7931A;
--status-success: #00FF00;
--status-warning: #FFAA00;
--status-error: #FF5555;
--status-info: #00AAFF;
```

### Typography

```css
/* UI Font */
font-family: 'SF Mono', 'Monaco', 'Menlo', monospace;

/* Sizes */
--text-xs: 10px;
--text-sm: 12px;
--text-base: 14px;
--text-lg: 16px;
--text-xl: 20px;
--text-2xl: 24px;
```

### Motion

```css
/* Transitions */
--transition-fast: 100ms linear;
--transition-normal: 150ms linear;

/* Blur */
--blur-inactive: 1px;

/* Never use */
/* - ease-in-out */
/* - bounce */
/* - spring */
/* - elastic */
```

---

## 🔧 COMPONENT HIERARCHY

```
X3StarOS (root)
├── BootSequence
├── SystemBar
│   ├── Logo (clickable → System Overview)
│   ├── NetworkStatus
│   ├── BlockHeight
│   ├── VMIndicators
│   └── Clock
├── Dock
│   ├── DockItem (Terminal)
│   ├── DockItem (Execution)
│   ├── DockItem (VMs)
│   ├── DockItem (Atomic)
│   └── DockItem (Ecosystem)
├── Window (generic wrapper)
│   ├── TitleBar
│   ├── Content
│   └── StatusBar
├── TerminalApp
├── ExecutionEngineApp
│   ├── HeaderStats
│   ├── BlockTable
│   ├── BlockDetailPanel
│   └── ExecutionTimeline
├── VMManagerApp
├── AtomicLayerApp
├── EcosystemApp
├── SystemOverviewApp
└── FailureModeOverlay
```

---

## 📁 FILE STRUCTURE

```
apps/explorer/
├── app/
│   ├── x3star/
│   │   └── page.tsx          # Main OS component
│   └── page.tsx              # Original landing (preserved)
├── src/
│   └── lib/
│       └── x3star/
│           └── data-binding.ts  # RPC/data integration
└── package.json
```

---

## 🚀 USAGE

### Access the OS

```
http://localhost:3001/x3star
```

### Development

```bash
cd apps/explorer
npm run dev
```

### Connect to Live Node

The OS automatically connects to:
- `ws://127.0.0.1:9944` (Substrate RPC)
- `http://127.0.0.1:9615/metrics` (Prometheus)

If the node is not running, the OS uses realistic mock data.

---

## ✅ IMPLEMENTATION CHECKLIST

- [x] Boot sequence with character-by-character typing
- [x] System bar with live clock and VM indicators
- [x] Dock with hover labels and active states
- [x] Window system with focus/blur states
- [x] Terminal with command execution
- [x] Execution Engine (Bloomberg-style tables)
- [x] VM Manager with status cards
- [x] Atomic Layer with execution simulation
- [x] Ecosystem/Process Monitor
- [x] System Overview window
- [x] Keyboard shortcuts
- [x] Failure mode overlays
- [x] Live data binding (with mock fallback)
- [x] Real Substrate RPC integration

---

## 🎯 FINAL DIRECTIVE

The finished product should make users think:

> "This looks like something I'm not qualified to use —  
> but I need to be."

If it feels like a website, you failed.  
If it feels like a terminal you could lose money in, you succeeded.

---

**x3Star OS**  
*Execution Layer for Everything*

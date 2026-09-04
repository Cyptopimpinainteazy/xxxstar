---
stepsCompleted: ["step-01-init"]
inputDocuments: ["_bmad-output/planning-artifacts/product-brief-x3-chain-master-2026-02-13.md", "_bmad-output/planning-artifacts/prd.md"]
---

# UX Design Specification - x3-chain-master

**Author:** Lojak
**Date:** 2026-02-13

---

<!-- UX design content will be appended sequentially through collaborative workflow steps -->

## 1. Design Philosophy & Visual Identity

### 1.1 Core Design Principles
- **Trust Through Transparency**: Clean, honest interfaces that build confidence
- **Complexity Made Simple**: Abstracting blockchain complexity behind intuitive interactions
- **Professional Yet Approachable**: Enterprise-grade feel with accessible design
- **Dark-First Theme**: Primary dark mode with light mode option for accessibility

### 1.2 Brand Identity
- **Primary Color**: Deep Blue (#0A1628) - Trust, security, professionalism
- **Accent Color**: Electric Cyan (#00D4FF) - Innovation, energy, technology
- **Secondary Accent**: Violet (#8B5CF6) - AI/Intelligence features
- **Success**: Emerald Green (#10B981)
- **Warning**: Amber (#F59E0B)
- **Error**: Rose Red (#EF4444)
- **Background**: Dark Navy (#0F172A)
- **Surface**: Slate (#1E293B)
- **Text Primary**: White (#F8FAFC)
- **Text Secondary**: Gray (#94A3B8)

### 1.3 Typography
- **Headings**: Inter Bold - Clean, modern, highly readable
- **Body**: Inter Regular - Optimal for dense information
- **Monospace**: JetBrains Mono - For addresses, hashes, code

### 1.4 Spacing & Layout
- **Base Unit**: 4px
- **Border Radius**: 8px (cards), 12px (modals), 4px (buttons)
- **Shadows**: Subtle glow effects using accent colors
- **Grid**: 12-column responsive grid system

---

## 2. User Experience Guidelines

### 2.1 Information Architecture

#### Primary Navigation
```
┌─────────────────────────────────────────────────────────────┐
│ [Logo] X3 Chain    [Wallet] [Settings] [Help]          │
├──────────┬──────────┬──────────┬──────────┬─────────────────┤
│ Explorer │   DEX    │  Wallet  │  Agent  │    GPU Swarm    │
└──────────┴──────────┴──────────┴──────────┴─────────────────┘
```

#### Navigation Priority
1. **Dashboard/Home** - Overview of all positions and stats
2. **DEX** - Trading, swaps, liquidity
3. **Wallet** - Multi-chain asset management
4. **GPU Marketplace** - Compute buying/selling
5. **X3 Intelligence** - AI agents and trading
6. **Network** - Validators, stake, governance

### 2.2 Core UX Principles

#### Onboarding Flow
- **Step 1**: Wallet creation/import (supported wallets shown)
- **Step 2**: Security setup (2FA, biometrics if available)
- **Step 3**: Chain preferences selection
- **Step 4**: Quick tour of key features
- **Step 5**: Dashboard with guided first action

#### Transaction Experience
- **Before**: Clear fee estimation, slippage settings
- **During**: Progress indicators, step-by-step status
- **After**: Success confirmation with tx link, next actions suggested

#### Error Handling
- **Human-readable messages**: Avoid technical jargon
- **Actionable guidance**: Tell user what to do next
- **Recovery options**: Clear paths to fix issues

---

## 3. Page-Specific UX Specifications

### 3.1 Dashboard (Home)
**Purpose**: Central command center for user's entire portfolio

**Layout**:
```
┌────────────────────────────────────────────────────────────┐
│ Total Value: $XX,XXX    [24h: +X.X%]    [Actions ▼]       │
├────────────────────────────────────────────────────────────┤
│ ┌─────────────────┐ ┌─────────────────┐ ┌───────────────┐ │
│ │ Portfolio Chart │ │ Active Agents  │ │ GPU Jobs      │ │
│ │ (Line/Donut)    │ │ (Count/Status) │ │ (Running/Total│ │
│ └─────────────────┘ └─────────────────┘ └───────────────┘ │
├────────────────────────────────────────────────────────────┤
│ Recent Transactions                          [View All >]  │
│ ┌────────────────────────────────────────────────────────┐ │
│ │ [Icon] Swap XXX → YYY    +X%    2m ago               │ │
│ │ [Icon] GPU Job Complete  Done   5m ago               │ │
│ │ [Icon] Agent Trade       +X%    12m ago              │ │
│ └────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────┘
```

**Key Features**:
- Real-time portfolio value updates
- One-click access to common actions
- Activity feed with smart filtering

### 3.2 DEX Trading Page
**Purpose**: Professional trading interface for swaps and liquidity

**Layout**:
```
┌────────────────────────────────────────────────────────────┐
│ [Token A] → [Token B]    [Price: X.XXX]    [Pool] [Swap] │
├────────────────────────────────────────────────────────────┤
│ ┌────────────────────────────┐ ┌────────────────────────┐ │
│ │      Order Book            │ │   Your Positions      │ │
│ │ Sells    | Price | Buys    │ │ ┌────┐ ┌────┐ ┌────┐  │ │
│ │ 12.5     | 1.234 | 8.2    │ │ │ LP1│ │ LP2│ │ LP3│  │ │
│ │ 5.3      | 1.233 | 15.7   │ │ └────┘ └────┘ └────┘  │ │
│ └────────────────────────────┘ └────────────────────────┘ │
├────────────────────────────────────────────────────────────┤
│ Trading Panel                                             │
│ [Market] [Limit] [Stop]    Amount: [____________] [MAX]  │
│                                                            │
│ Slippage: [1%] [3%] [5%] [Custom]                         │
│                                                            │
│ [BUY / SELL]                                              │
└────────────────────────────────────────────────────────────┘
```

### 3.3 Wallet Page
**Purpose**: Unified multi-chain asset management

**Layout**:
```
┌────────────────────────────────────────────────────────────┐
│ Total: $XX,XXX    Chains: XX active    [Add Chain]        │
├────────────────────────────────────────────────────────────┤
│ [Search tokens...                        ] [Filter ▼]      │
├────────────────────────────────────────────────────────────┤
│ ┌────────────────────────────────────────────────────────┐ │
│ │ 🪙 ETH   Ethereum      12.5    $25,000  [Send][Swap] │ │
│ │ 🪙 DOT   Polkadot      500     $3,500   [ │ │
│ │ 🪙 SOL   Solana        Send][Swap]150     $12,000  [Send][Swap] │ │
│ │ 🪙 ATL   X3         10,000  $1,000   [Send][Swap] │ │
│ └────────────────────────────────────────────────────────┘ │
├────────────────────────────────────────────────────────────┤
│ Cross-Chain                                              [+] │
│ ┌────────────────────────────────────────────────────────┐ │
│ │ Bridge → Transfer assets between chains                 │ │
│ │ →  ETH → DOT (via XCM)                                  │ │
│ └────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────┘
```

### 3.4 GPU Marketplace Page
**Purpose**: Decentralized compute trading interface

**Layout**:
```
┌────────────────────────────────────────────────────────────┐
│ GPU Compute Marketplace          [Buy Compute] [Sell GPU] │
├────────────────────────────────────────────────────────────┤
│ Stats: Available: XX GPUs | Jobs Running: XX | Avg: $X/hr  │
├────────────────────────────────────────────────────────────┤
│ ┌────────────────────────────┐ ┌────────────────────────┐ │
│ │ Available Compute         │ │ My GPU Resources      │ │
│ │ [Filter: Type / Price]    │ │ Status: Online        │ │
│ │ ┌──────┐ ┌──────┐ ┌────┐ │ │ Revenue: $XXX        │ │
│ │ │ GPU1 │ │ GPU2 │ │... │ │ │ Jobs: XX completed   │ │
│ │ │ RTX  │ │ A100 │ │    │ │ │ [Manage]             │ │
│ │ │ $X/h │ │ $X/h │ │    │ │ └─────────────────────┘ │
│ │ └──────┘ └──────┘ └────┘ │                            │
│ └────────────────────────────┘                            │
├────────────────────────────────────────────────────────────┤
│ Active Jobs                                              [+] │
│ ┌────────────────────────────────────────────────────────┐ │
│ │ Job #1234: Training Model    ████████░░  80%  2m left │ │
│ │ Job #1235: Rendering        ██░░░░░░░░░  20%  8m left │ │
│ └────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────┘
```

### 3.5 X3 Intelligence Page
**Purpose**: AI agent management and trading automation

**Layout**:
```
┌────────────────────────────────────────────────────────────┐
│ X3 Intelligence          [+ New Agent]    [Activity] 🔔  │
├────────────────────────────────────────────────────────────┤
│ ┌────────────────────────────────────────────────────────┐ │
│ │ Active Agents                          [Total: XX]    │ │
│ │ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐  │ │
│ │ │ 🤖 Trader│ │ 🤖 Arbitr│ │ 🤖 pools │ │ 🤖 ...   │  │ │
│ │ │ Status: 🟢│ │ Status: 🟢│ │ Status: 🟡│ │          │  │ │
│ │ │ +X%/day  │ │ +X%/day  │ │ -X%/day  │ │          │  │ │
│ │ │ [Config] │ │ [Config] │ │ [Config] │ │          │  │ │
│ │ └──────────┘ └──────────┘ └──────────┘ └──────────┘  │ │
│ └────────────────────────────────────────────────────────┘ │
├────────────────────────────────────────────────────────────┤
│ Intent Builder                                           [+] │
│ ┌────────────────────────────────────────────────────────┐ │
│ │ "When BTC > $50k and network congestion < 10%,        │ │
│ │ swap 0.1 BTC to USDC on DEX with 1% slippage"        │ │
│ └────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────┘
```

---

## 4. Component Library

### 4.1 Buttons
- **Primary**: Solid accent color, white text
- **Secondary**: Outlined, accent border
- **Ghost**: Text only, hover shows background
- **Danger**: Red variant for destructive actions

### 4.2 Cards
- **Standard Card**: Surface background, 8px radius, subtle border
- **Highlight Card**: Accent glow border on hover
- **Stat Card**: Large number display with label and trend indicator

### 4.3 Forms
- **Input Fields**: Dark surface, 1px border, focus glow
- **Dropdowns**: Consistent with input styling
- **Toggles**: Accent color when active
- **Sliders**: For percentage values (slippage, etc.)

### 4.4 Feedback Components
- **Toast Notifications**: Slide in from top-right, auto-dismiss
- **Modals**: Centered, backdrop blur, clear close button
- **Loading States**: Skeleton screens, spinners for quick loads
- **Progress Bars**: Accent gradient, smooth animations

---

## 5. Responsive Breakpoints

### 5.1 Mobile (< 640px)
- Single column layout
- Bottom navigation bar (5 primary items)
- Collapsible sections
- Touch-optimized tap targets (44px minimum)

### 5.2 Tablet (640px - 1024px)
- Two column layouts
- Side navigation becomes drawer
- Optimized for both portrait and landscape

### 5.3 Desktop (> 1024px)
- Full multi-column layouts
- Persistent side navigation
- Keyboard shortcuts enabled
- Hover states active

---

## 6. Accessibility Guidelines

### 6.1 WCAG 2.1 AA Compliance
- Color contrast ratio minimum 4.5:1
- All interactive elements keyboard accessible
- Focus indicators visible
- Screen reader compatible labels

### 6.2 Color-blind Safe
- Never use color alone to convey information
- Use icons and text labels alongside color
- Test designs with color blindness simulators

---

## 7. Animation & Transitions

### 7.1 Motion Principles
- **Duration**: 150ms for micro-interactions, 300ms for page transitions
- **Easing**: Ease-out for entry, ease-in for exit
- **Purpose**: Every animation should serve a purpose (feedback, orientation, or delight)

### 7.2 Common Animations
- **Page Transitions**: Fade + slight slide
- **Card Hover**: Subtle lift with shadow increase
- **Button Press**: Scale down slightly (95%)
- **Loading**: Pulsing skeleton screens
- **Success**: Confetti burst for significant achievements

---

## 8. Appendix

### 8.1 Reference Documents
- Product Brief: `product-brief-x3-chain-master-2026-02-13.md`
- PRD: `prd.md`
- Component Library: TBD (to be created in implementation)

### 8.2 Design Tools
- Primary: Figma (design files to be created)
- Prototyping: Built into Figma
- Handoff: Figma + Storybook

---

*Document Version: 1.0*  
*Created: 2026-02-13*  
*Author: Lojak (via UX Designer Agent)*  
*Workflow: BMAD Create UX Design*

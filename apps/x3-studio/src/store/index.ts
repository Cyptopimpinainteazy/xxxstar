import { create } from 'zustand';
import type { Tab, ProofRecord, ScoreboardCategory, ProjectDetection, ScannerFinding, ChainState, DiagnosticEntry, ExtensionPanel, Keybinding, VerificationProgress, DebuggerBreakpoint, DebugVar, DebugFrame, AiConversation, NetworkRequest, ForgeCoverageResult, GasProfileEntry, TpsBenchmarkResult, DeploymentConfig, DaoProposal, AccountAbstractionWallet, CrossChainTx, ChainConfig, IpcPermission, ExtensionCandidate, RegistryPackage, CollabSession, SolidityCompilerOutput, WasmModule, TpsMeterSnapshot, VsCodeKeybinding } from '../types';

// ── Workspace Store ──
interface WorkspaceState {
  workspacePath: string | null;
  workspaceName: string;
  detection: ProjectDetection | null;
  branch: string;
  gitStatus: { status: string; file: string }[];
  setWorkspace: (path: string | null) => void;
  setDetection: (d: ProjectDetection) => void;
  setBranch: (b: string) => void;
  setGitStatus: (s: { status: string; file: string }[]) => void;
}
export const useWorkspaceStore = create<WorkspaceState>((set) => ({
  workspacePath: null, workspaceName: '', detection: null, branch: 'unknown', gitStatus: [],
  setWorkspace: (path) => set({ workspacePath: path, workspaceName: path ? path.split('/').pop() || path : '' }),
  setDetection: (d) => set({ detection: d }),
  setBranch: (b) => set({ branch: b }),
  setGitStatus: (s) => set({ gitStatus: s }),
}));

// ── Editor Store ──
interface EditorState {
  tabs: Tab[];
  activeTabId: string | null;
  openFile: (filePath: string, content: string, language: string) => void;
  closeTab: (id: string) => void;
  setActiveTab: (id: string) => void;
  updateContent: (id: string, content: string) => void;
  markClean: (id: string) => void;
  getActiveTab: () => Tab | undefined;
}
export const useEditorStore = create<EditorState>((set, get) => ({
  tabs: [], activeTabId: null,
  openFile: (filePath, content, language) => {
    const existing = get().tabs.find(t => t.filePath === filePath);
    if (existing) { set({ activeTabId: existing.id }); return; }
    const id = `tab_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`;
    set((s) => ({ tabs: [...s.tabs, { id, filePath, fileName: filePath.split('/').pop() || filePath, language, content, originalContent: content, dirty: false }], activeTabId: id }));
  },
  closeTab: (id) => set((s) => {
    const tabs = s.tabs.filter(t => t.id !== id);
    return { tabs, activeTabId: s.activeTabId === id ? (tabs.length > 0 ? tabs[tabs.length - 1].id : null) : s.activeTabId };
  }),
  setActiveTab: (id) => set({ activeTabId: id }),
  updateContent: (id, content) => set((s) => ({ tabs: s.tabs.map(t => t.id === id ? { ...t, content, dirty: content !== t.originalContent } : t) })),
  markClean: (id) => set((s) => ({ tabs: s.tabs.map(t => t.id === id ? { ...t, originalContent: t.content, dirty: false } : t) })),
  getActiveTab: () => get().tabs.find(t => t.id === get().activeTabId),
}));

// ── Proof Store ──
interface ProofState {
  records: ProofRecord[]; isRunning: boolean; progress: VerificationProgress | null;
  addRecord: (r: ProofRecord) => void; setRunning: (v: boolean) => void; setProgress: (p: VerificationProgress | null) => void; clear: () => void;
}
export const useProofStore = create<ProofState>((set) => ({
  records: [], isRunning: false, progress: null,
  addRecord: (r) => set((s) => ({ records: [r, ...s.records] })),
  setRunning: (v) => set({ isRunning: v }),
  setProgress: (p) => set({ progress: p }),
  clear: () => set({ records: [] }),
}));

// ── Scoreboard Store ──
interface ScoreboardState {
  categories: ScoreboardCategory[]; totalScore: number;
  setCategories: (c: ScoreboardCategory[]) => void; calculateTotal: () => void;
}
export const useScoreboardStore = create<ScoreboardState>((set, get) => ({
  categories: [], totalScore: 0,
  setCategories: (c) => set({ categories: c }),
  calculateTotal: () => { const cats = get().categories; set({ totalScore: cats.length > 0 ? Math.round(cats.reduce((a, c) => a + c.score, 0) / cats.length) : 0 }); },
}));

// ── Scanner Store ──
interface ScannerState { findings: ScannerFinding[]; isScanning: boolean; setFindings: (f: ScannerFinding[]) => void; setScanning: (v: boolean) => void; }
export const useScannerStore = create<ScannerState>((set) => ({ findings: [], isScanning: false, setFindings: (f) => set({ findings: f }), setScanning: (v) => set({ isScanning: v }) }));

// ── Diagnostics Store ──
interface DiagnosticsState { entries: DiagnosticEntry[]; addEntries: (e: DiagnosticEntry[]) => void; clear: () => void; }
export const useDiagnosticsStore = create<DiagnosticsState>((set) => ({ entries: [], addEntries: (e) => set((s) => ({ entries: [...s.entries, ...e] })), clear: () => set({ entries: [] }) }));

// ── Chain Store ──
interface ChainStateType { chain: ChainState | null; setChain: (c: ChainState) => void; history: ChainState[]; }
export const useChainStore = create<ChainStateType>((set) => ({ chain: null, history: [], setChain: (c) => set((s) => ({ chain: c, history: [...s.history.slice(-59), c] })) }));

// ── Settings Store ──
interface SettingsState {
  proofMode: boolean; strictMainnet: boolean; allowMocks: boolean; allowStubs: boolean;
  proofOutputDir: string; defaultShell: string; aiProvider: string; aiEndpoint: string; aiModel: string;
  theme: 'dark' | 'light'; autosave: boolean; commandTimeout: number;
  mainnetGateCommand: string; testnetGateCommand: string; verifyCommand: string;
  chainRpcUrl: string; saveConversations: boolean; conversationDir: string; forgePath: string;
  sourcifyApiUrl: string; explorerApiUrl: string;
  update: (partial: Partial<SettingsState>) => void; reset: () => void; get: () => SettingsState;
}
const defaultSettings: SettingsState = {
  proofMode: true, strictMainnet: false, allowMocks: false, allowStubs: false,
  proofOutputDir: 'x3-proof', defaultShell: '/bin/bash', aiProvider: 'ollama', aiEndpoint: 'http://localhost:11434', aiModel: 'codellama',
  theme: 'dark', autosave: true, commandTimeout: 120,
  mainnetGateCommand: 'cargo test && forge test && pnpm test', testnetGateCommand: 'cargo check && forge build', verifyCommand: 'cargo check',
  chainRpcUrl: 'http://localhost:8545', saveConversations: true, conversationDir: '.x3studio/conversations', forgePath: 'forge',
  sourcifyApiUrl: 'https://sourcify.dev/server', explorerApiUrl: 'https://api.etherscan.io/api',
  update: () => {}, reset: () => {}, get: () => ({} as SettingsState),
};
export const useSettingsStore = create<SettingsState>((set, get) => ({ ...defaultSettings, update: (partial) => set(partial), reset: () => set(defaultSettings), get: () => get() }));

// ── Terminal Store ──
interface TerminalState { terminals: { id: string; name: string }[]; activeTerminalId: string | null; addTerminal: (id: string, name: string) => void; removeTerminal: (id: string) => void; setActive: (id: string) => void; }
export const useTerminalStore = create<TerminalState>((set) => ({
  terminals: [], activeTerminalId: null,
  addTerminal: (id, name) => set((s) => ({ terminals: [...s.terminals, { id, name }], activeTerminalId: id })),
  removeTerminal: (id) => set((s) => {
    const terms = s.terminals.filter(t => t.id !== id);
    return { terminals: terms, activeTerminalId: s.activeTerminalId === id ? (terms.length > 0 ? terms[terms.length - 1].id : null) : s.activeTerminalId };
  }),
  setActive: (id) => set({ activeTerminalId: id }),
}));

// ── Layout Store ──
interface LayoutState {
  sidebarPanel: string; bottomPanel: string; sidebarVisible: boolean; bottomVisible: boolean;
  bottomPanels: string[];
  setSidebarPanel: (p: string) => void; setBottomPanel: (p: string) => void;
  toggleSidebar: () => void; toggleBottom: () => void;
  moveToBottom: (panelId: string) => void; moveToSidebar: (panelId: string) => void;
  movePanelTo: (panelId: string, target: 'sidebar' | 'bottom') => void;
}
const loadBottomPanels = (): string[] => {
  try { const d = localStorage.getItem('x3studio-bottom-panels'); return d ? JSON.parse(d) : []; } catch { return []; }
};
const saveBottomPanels = (panels: string[]) => {
  try { localStorage.setItem('x3studio-bottom-panels', JSON.stringify(panels)); } catch {}
};
export const useLayoutStore = create<LayoutState>((set) => ({
  sidebarPanel: 'control-center', bottomPanel: 'terminal', sidebarVisible: true, bottomVisible: true,
  bottomPanels: loadBottomPanels(),
  setSidebarPanel: (p) => set({ sidebarPanel: p }),
  setBottomPanel: (p) => set({ bottomPanel: p }),
  toggleSidebar: () => set((s) => ({ sidebarVisible: !s.sidebarVisible })),
  toggleBottom: () => set((s) => ({ bottomVisible: !s.bottomVisible })),
  moveToBottom: (panelId) => set((s) => {
    if (s.bottomPanels.includes(panelId)) return s;
    const next = [...s.bottomPanels, panelId];
    saveBottomPanels(next);
    return { bottomPanels: next, bottomPanel: panelId, bottomVisible: true };
  }),
  moveToSidebar: (panelId) => set((s) => {
    const next = s.bottomPanels.filter(p => p !== panelId);
    saveBottomPanels(next);
    return { bottomPanels: next, sidebarPanel: s.sidebarPanel === '' ? panelId : s.sidebarPanel };
  }),
  movePanelTo: (panelId, target) => set((s) => {
    if (target === 'bottom') {
      if (s.bottomPanels.includes(panelId)) return s;
      const next = [...s.bottomPanels, panelId];
      saveBottomPanels(next);
      return { bottomPanels: next, bottomPanel: panelId, bottomVisible: true };
    } else {
      const next = s.bottomPanels.filter(p => p !== panelId);
      saveBottomPanels(next);
      return { bottomPanels: next };
    }
  }),
}));

// ── Extension Store ──
interface ExtensionState { panels: ExtensionPanel[]; registerPanel: (p: ExtensionPanel) => void; unregisterPanel: (id: string) => void; }
export const useExtensionStore = create<ExtensionState>((set) => ({ panels: [], registerPanel: (p) => set((s) => ({ panels: [...s.panels, p] })), unregisterPanel: (id) => set((s) => ({ panels: s.panels.filter(p => p.id !== id) })) }));

// ── Keybinding Store ──
interface KeybindingState { bindings: Keybinding[]; setBindings: (b: Keybinding[]) => void; updateBinding: (id: string, keys: string) => void; }
export const useKeybindingStore = create<KeybindingState>((set) => ({
  bindings: [
    { id: 'save', label: 'Save File', keys: 'Ctrl+S', command: 'editor.save' },
    { id: 'save-all', label: 'Save All', keys: 'Ctrl+Shift+S', command: 'editor.saveAll' },
    { id: 'find', label: 'Find', keys: 'Ctrl+F', command: 'editor.find' },
    { id: 'replace', label: 'Replace', keys: 'Ctrl+H', command: 'editor.replace' },
    { id: 'go-to-line', label: 'Go to Line', keys: 'Ctrl+G', command: 'editor.goToLine' },
    { id: 'toggle-sidebar', label: 'Toggle Sidebar', keys: 'Ctrl+B', command: 'layout.toggleSidebar' },
    { id: 'toggle-terminal', label: 'Toggle Terminal', keys: 'Ctrl+`', command: 'layout.toggleTerminal' },
    { id: 'command-palette', label: 'Command Palette', keys: 'Ctrl+Shift+P', command: 'editor.commandPalette' },
    { id: 'close-tab', label: 'Close Tab', keys: 'Ctrl+W', command: 'editor.closeTab' },
    { id: 'toggle-bottom', label: 'Toggle Bottom Panel', keys: 'Ctrl+J', command: 'layout.toggleBottom' },
  ],
  setBindings: (b) => set({ bindings: b }), updateBinding: (id, keys) => set((s) => ({ bindings: s.bindings.map(b => b.id === id ? { ...b, keys } : b) })),
}));

// ── Debugger Store ──
interface DebuggerState {
  breakpoints: DebuggerBreakpoint[]; isAttached: boolean; currentFile: string | null; currentLine: number | null;
  variables: DebugVar[]; callStack: DebugFrame[]; debuggerId: string | null; sessionOutput: string;
  addBreakpoint: (bp: DebuggerBreakpoint) => void; removeBreakpoint: (id: string) => void; toggleBreakpoint: (id: string) => void;
  setAttached: (v: boolean) => void; setLocation: (file: string | null, line: number | null) => void;
  setVariables: (v: DebugVar[]) => void; setCallStack: (cs: DebugFrame[]) => void;
  setDebuggerId: (id: string | null) => void; appendOutput: (s: string) => void; clearOutput: () => void;
}
export const useDebuggerStore = create<DebuggerState>((set) => ({
  breakpoints: [], isAttached: false, currentFile: null, currentLine: null, variables: [], callStack: [], debuggerId: null, sessionOutput: '',
  addBreakpoint: (bp) => set((s) => ({ breakpoints: [...s.breakpoints, bp] })),
  removeBreakpoint: (id) => set((s) => ({ breakpoints: s.breakpoints.filter(b => b.id !== id) })),
  toggleBreakpoint: (id) => set((s) => ({ breakpoints: s.breakpoints.map(b => b.id === id ? { ...b, enabled: !b.enabled } : b) })),
  setAttached: (v) => set({ isAttached: v }), setLocation: (file, line) => set({ currentFile: file, currentLine: line }),
  setVariables: (v) => set({ variables: v }), setCallStack: (cs) => set({ callStack: cs }),
  setDebuggerId: (id) => set({ debuggerId: id }), appendOutput: (s) => set((state) => ({ sessionOutput: state.sessionOutput + '\n' + s })), clearOutput: () => set({ sessionOutput: '' }),
}));

// ── AI Conversation Store ──
interface AiConversationState { conversations: AiConversation[]; activeConversationId: string | null; setConversations: (c: AiConversation[]) => void; setActive: (id: string | null) => void; addConversation: (c: AiConversation) => void; }
export const useAiConversationStore = create<AiConversationState>((set) => ({ conversations: [], activeConversationId: null, setConversations: (c) => set({ conversations: c }), setActive: (id) => set({ activeConversationId: id }), addConversation: (c) => set((s) => ({ conversations: [...s.conversations, c] })) }));

// ── Network Profiler Store ──
interface NetworkProfilerState { requests: NetworkRequest[]; isRecording: boolean; addRequest: (r: NetworkRequest) => void; setRecording: (v: boolean) => void; clear: () => void; }
export const useNetworkProfilerStore = create<NetworkProfilerState>((set) => ({ requests: [], isRecording: false, addRequest: (r) => set((s) => ({ requests: [r, ...s.requests.slice(0, 499)] })), setRecording: (v) => set({ isRecording: v }), clear: () => set({ requests: [] }) }));

// ── Forge Coverage Store ──
interface ForgeCoverageState { result: ForgeCoverageResult | null; isRunning: boolean; setResult: (r: ForgeCoverageResult | null) => void; setRunning: (v: boolean) => void; }
export const useForgeCoverageStore = create<ForgeCoverageState>((set) => ({ result: null, isRunning: false, setResult: (r) => set({ result: r }), setRunning: (v) => set({ isRunning: v }) }));

// ── Gas Profiler Store ──
interface GasProfilerState { entries: GasProfileEntry[]; addEntry: (e: GasProfileEntry) => void; clear: () => void; }
export const useGasProfilerStore = create<GasProfilerState>((set) => ({ entries: [], addEntry: (e) => set((s) => ({ entries: [e, ...s.entries.slice(0, 99)] })), clear: () => set({ entries: [] }) }));

// ── TPS Benchmark Store ──
interface TpsBenchmarkState { results: TpsBenchmarkResult[]; isRunning: boolean; addResult: (r: TpsBenchmarkResult) => void; setRunning: (v: boolean) => void; clear: () => void; }
export const useTpsBenchmarkStore = create<TpsBenchmarkState>((set) => ({ results: [], isRunning: false, addResult: (r) => set((s) => ({ results: [r, ...s.results.slice(0, 49)] })), setRunning: (v) => set({ isRunning: v }), clear: () => set({ results: [] }) }));

// ── Deployment Config Store ──
interface DeploymentConfigState { configs: DeploymentConfig[]; addConfig: (c: DeploymentConfig) => void; removeConfig: (name: string) => void; }
export const useDeploymentConfigStore = create<DeploymentConfigState>((set) => ({ configs: [], addConfig: (c) => set((s) => ({ configs: [...s.configs, c] })), removeConfig: (name) => set((s) => ({ configs: s.configs.filter(c => c.name !== name) })) }));

// ── Cross-chain Tx Store ──
interface CrossChainTxState { transactions: CrossChainTx[]; addTx: (tx: CrossChainTx) => void; updateTx: (sourceTx: string, updates: Partial<CrossChainTx>) => void; }
export const useCrossChainTxStore = create<CrossChainTxState>((set) => ({ transactions: [], addTx: (tx) => set((s) => ({ transactions: [...s.transactions, tx] })), updateTx: (sourceTx, updates) => set((s) => ({ transactions: s.transactions.map(t => t.sourceTx === sourceTx ? { ...t, ...updates } : t) })) }));

// ── Chain Config Store ──
interface ChainConfigState { configs: ChainConfig[]; addConfig: (c: ChainConfig) => void; removeConfig: (name: string) => void; }
export const useChainConfigStore = create<ChainConfigState>((set) => ({
  configs: [
    { name: 'Ethereum Mainnet', chainId: 1, rpcUrl: 'https://eth.llamarpc.com', explorerUrl: 'https://etherscan.io', currency: 'ETH', type: 'evm' },
    { name: 'Base', chainId: 8453, rpcUrl: 'https://mainnet.base.org', explorerUrl: 'https://basescan.org', currency: 'ETH', type: 'evm' },
    { name: 'Arbitrum One', chainId: 42161, rpcUrl: 'https://arb1.arbitrum.io/rpc', explorerUrl: 'https://arbiscan.io', currency: 'ETH', type: 'evm' },
    { name: 'Polygon', chainId: 137, rpcUrl: 'https://polygon-rpc.com', explorerUrl: 'https://polygonscan.com', currency: 'MATIC', type: 'evm' },
    { name: 'Optimism', chainId: 10, rpcUrl: 'https://mainnet.optimism.io', explorerUrl: 'https://optimistic.etherscan.io', currency: 'ETH', type: 'evm' },
    { name: 'Solana', chainId: 0, rpcUrl: 'https://api.mainnet-beta.solana.com', explorerUrl: 'https://solscan.io', currency: 'SOL', type: 'svm' },
    { name: 'X3 Local', chainId: 49009, rpcUrl: 'http://localhost:8545', explorerUrl: '', currency: 'X3', type: 'evm' },
    { name: 'X3 Testnet', chainId: 49010, rpcUrl: 'https://testnet.x3chain.xyz', explorerUrl: '', currency: 'X3', type: 'evm' },
  ],
  addConfig: (c) => set((s) => ({ configs: [...s.configs, c] })),
  removeConfig: (name) => set((s) => ({ configs: s.configs.filter(c => c.name !== name) })),
}));

// ── Permissions Store ──
interface PermissionState { permissions: IpcPermission[]; updatePermission: (channel: string, allowed: boolean) => void; setPermissions: (p: IpcPermission[]) => void; }
export const usePermissionStore = create<PermissionState>((set) => ({ permissions: [], updatePermission: (channel, allowed) => set((s) => ({ permissions: s.permissions.map(p => p.channel === channel ? { ...p, allowed } : p) })), setPermissions: (p) => set({ permissions: p }) }));

// ── Registry Store ──
interface RegistryState { packages: RegistryPackage[]; isSearching: boolean; setPackages: (p: RegistryPackage[]) => void; setSearching: (v: boolean) => void; }
export const useRegistryStore = create<RegistryState>((set) => ({ packages: [], isSearching: false, setPackages: (p) => set({ packages: p }), setSearching: (v) => set({ isSearching: v }) }));

// ── Collab Store ──
interface CollabState { sessions: CollabSession[]; activeSessionId: string | null; setSessions: (s: CollabSession[]) => void; setActiveSession: (id: string | null) => void; addSession: (s: CollabSession) => void; removeSession: (id: string) => void; updateSession: (id: string, u: Partial<CollabSession>) => void; }
export const useCollabStore = create<CollabState>((set) => ({ sessions: [], activeSessionId: null,
  setSessions: (s) => set({ sessions: s }), setActiveSession: (id) => set({ activeSessionId: id }),
  addSession: (s) => set((st) => ({ sessions: [...st.sessions, s] })),
  removeSession: (id) => set((st) => ({ sessions: st.sessions.filter(x => x.id !== id) })),
  updateSession: (id, u) => set((st) => ({ sessions: st.sessions.map(x => x.id === id ? { ...x, ...u } : x) })),
}));

// ── Solidity Compiler Store ──
interface SolidityCompilerState { output: SolidityCompilerOutput | null; isCompiling: boolean; error: string | null; setOutput: (o: SolidityCompilerOutput | null) => void; setCompiling: (v: boolean) => void; setError: (e: string | null) => void; }
export const useSolidityCompilerStore = create<SolidityCompilerState>((set) => ({ output: null, isCompiling: false, error: null, setOutput: (o) => set({ output: o }), setCompiling: (v) => set({ isCompiling: v }), setError: (e) => set({ error: e }) }));

// ── WASM Debugger Store ──
interface WasmDebuggerState { modules: WasmModule[]; activeModule: WasmModule | null; setModules: (m: WasmModule[]) => void; setActive: (m: WasmModule | null) => void; addModule: (m: WasmModule) => void; }
export const useWasmDebuggerStore = create<WasmDebuggerState>((set) => ({ modules: [], activeModule: null, setModules: (m) => set({ modules: m }), setActive: (m) => set({ activeModule: m }), addModule: (m) => set((s) => ({ modules: [...s.modules.filter(x => x.path !== m.path), m] })) }));

// ── TPS Meter Store ──
interface TpsMeterState { snapshots: TpsMeterSnapshot[]; currentTps: number; currentBlock: number; isPolling: boolean; addSnapshot: (s: TpsMeterSnapshot) => void; setCurrentTps: (v: number) => void; setCurrentBlock: (v: number) => void; setPolling: (v: boolean) => void; }
export const useTpsMeterStore = create<TpsMeterState>((set) => ({ snapshots: [], currentTps: 0, currentBlock: 0, isPolling: false,
  addSnapshot: (s) => set((st) => ({ snapshots: [...st.snapshots.slice(-59), s] })),
  setCurrentTps: (v) => set({ currentTps: v }), setCurrentBlock: (v) => set({ currentBlock: v }), setPolling: (v) => set({ isPolling: v }),
}));

// ── VS Code Keybindings Import Store ──
interface KeybindImportState { importedBindings: VsCodeKeybinding[]; setImportedBindings: (b: VsCodeKeybinding[]) => void; clearImported: () => void; }
export const useKeybindImportStore = create<KeybindImportState>((set) => ({ importedBindings: [], setImportedBindings: (b) => set({ importedBindings: b }), clearImported: () => set({ importedBindings: [] }) }));

// ── Multi-Window State Store ──
interface MultiWindowState { windows: { id: string; url: string; title: string; width: number; height: number; x?: number; y?: number }[]; addWindow: (w: any) => void; removeWindow: (id: string) => void; setWindows: (w: any[]) => void; }
export const useMultiWindowStore = create<MultiWindowState>((set) => ({ windows: [],
  addWindow: (w) => set((s) => ({ windows: [...s.windows, w] })),
  removeWindow: (id) => set((s) => ({ windows: s.windows.filter(x => x.id !== id) })),
  setWindows: (w) => set({ windows: w }),
}));

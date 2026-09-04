export interface X3StudioAPI {
  fs: {
    readFile: (path: string) => Promise<string>;
    writeFile: (path: string, content: string) => Promise<boolean>;
    readDir: (path: string) => Promise<DirEntry[]>;
    deleteFile: (path: string) => Promise<boolean>;
    rename: (oldPath: string, newPath: string) => Promise<boolean>;
    createFile: (path: string) => Promise<boolean>;
    createDirectory: (path: string) => Promise<boolean>;
    exists: (path: string) => Promise<boolean>;
    stat: (path: string) => Promise<FileStat>;
    glob: (dir: string, pattern: string) => Promise<string[]>;
  };
  dialog: { openDirectory: () => Promise<string | null> };
  shell: { exec: (command: string, cwd?: string) => Promise<CmdResult>; openExternal: (url: string) => Promise<void> };
  terminal: {
    create: (id: string, cwd: string) => Promise<boolean>;
    write: (id: string, data: string) => Promise<boolean>;
    resize: (id: string, cols: number, rows: number) => Promise<boolean>;
    kill: (id: string) => Promise<boolean>;
    onData: (callback: (id: string, data: string) => void) => void;
    onExit: (callback: (id: string, code: number | null) => void) => void;
    onError: (callback: (id: string, msg: string) => void) => void;
  };
  git: {
    status: (repoPath: string) => Promise<GitStatusEntry[]>;
    branch: (repoPath: string) => Promise<string>;
    log: (repoPath: string, count?: number) => Promise<GitLogEntry[]>;
    diff: (repoPath: string) => Promise<string>;
    commit: (repoPath: string, message: string) => Promise<CmdResult>;
    stash: (repoPath: string) => Promise<CmdResult>;
    checkout: (repoPath: string, branch: string) => Promise<CmdResult>;
    diffFile: (repoPath: string, file: string) => Promise<string>;
  };
  scanner: { scanFiles: (dir: string, patterns: string[]) => Promise<ScanResult[]> };
  app: { getEnv: () => Promise<any> };
  chain: { rpcCall: (rpcUrl: string, method: string, params: any[]) => Promise<any> };
  debugger: {
    start: (target: string, cwd: string) => Promise<CmdResult>;
    step: (id: string) => Promise<{ line: number; file: string; variables: DebugVar[]; callStack: DebugFrame[] }>;
    continue: (id: string) => Promise<{ line: number | null; file: string | null; variables: DebugVar[]; callStack: DebugFrame[] }>;
    setBreakpoint: (id: string, file: string, line: number) => Promise<boolean>;
    removeBreakpoint: (id: string, file: string, line: number) => Promise<boolean>;
    getVariables: (id: string) => Promise<DebugVar[]>;
    stop: (id: string) => Promise<boolean>;
  };
  extensions: {
    scanDirectory: (dir: string) => Promise<ExtensionCandidate[]>;
    installExtension: (sourcePath: string, name: string) => Promise<boolean>;
    uninstallExtension: (name: string) => Promise<boolean>;
    listInstalled: () => Promise<ExtensionCandidate[]>;
  };
  window: {
    create: (url: string, options?: { width?: number; height?: number; title?: string }) => Promise<string>;
    close: (id: string) => Promise<boolean>;
  };
  permissions: {
    request: (channel: string, args: any[]) => Promise<boolean>;
    getPermissions: () => Promise<IpcPermission[]>;
    setPermission: (channel: string, allowed: boolean) => Promise<boolean>;
  };
  registry: {
    search: (query: string) => Promise<RegistryPackage[]>;
    installPackage: (name: string, version?: string) => Promise<CmdResult>;
  };
  windowState: {
    save: () => Promise<boolean>;
    load: () => Promise<any>;
  };
  solidity: {
    compile: (inputJson: string, solcVersion?: string) => Promise<SolidityCompilerOutput>;
  };
  wasm: {
    inspect: (wasmPath: string) => Promise<WasmModule>;
  };
  chain: {
    rpcCall: (rpcUrl: string, method: string, params: any[]) => Promise<any>;
    monitorBlock: (rpcUrl: string) => Promise<{ blockNumber: number; txCount: number; tps: number; timestamp: number }>;
    syncConfigs: (rpcUrl: string) => Promise<{ configs: any[]; error: string | null }>;
  };
  collab: {
    createSession: (room: string, host: string) => Promise<any>;
    joinSession: (url: string) => Promise<{ connected: boolean; error: string | null }>;
  };
  on: (channel: string, callback: (...args: any[]) => void) => void;
  removeAllListeners: (channel: string) => void;
}

export interface DirEntry { name: string; isDirectory: boolean; isFile: boolean; path: string }
export interface FileStat { size: number; mtimeMs: number; isDirectory: boolean; isFile: boolean }
export interface CmdResult { stdout: string; stderr: string; exitCode: number }
export interface GitStatusEntry { status: string; file: string }
export interface GitLogEntry { hash: string; message: string }
export interface ScanResult { file: string; line: number; matched: string; content: string }
export interface DebugVar { name: string; value: string; type: string }
export interface DebugFrame { file: string; line: number; function: string }
export interface ExtensionCandidate { name: string; path: string; version: string; description: string; panels: string[]; icon?: string }
export interface IpcPermission { channel: string; allowed: boolean; lastRequest: string; count: number }

export type PanelId =
  | 'control-center' | 'project' | 'explorer' | 'search'
  | 'proof' | 'scoreboard' | 'scanner' | 'security'
  | 'adapters' | 'relayers' | 'validators' | 'proof-ledger' | 'chain-health'
  | 'debugger' | 'git-diff'
  | 'ai-agent' | 'launch-cockpit' | 'git' | 'problems' | 'output'
  | 'settings' | 'keybindings' | 'terminal'
  | 'extension-manager' | 'network-profiler' | 'forge-coverage' | 'permissions'
  | 'test-runner' | 'contract-verification' | 'gas-profiler' | 'graphql-explorer'
  | 'deployment-config' | 'dao-proposal' | 'account-abstraction' | 'tps-benchmark'
  | 'cross-chain-sim' | 'chain-config'
  | 'solidity-compiler' | 'wasm-debugger' | 'registry-marketplace' | 'collab'
  | 'chain-sync' | 'tps-meter';

export interface Tab {
  id: string;
  filePath: string;
  fileName: string;
  language: string;
  content: string;
  originalContent: string;
  dirty: boolean;
}

export interface ProofRecord {
  id: string;
  command: string;
  cwd: string;
  startTime: string;
  endTime: string;
  duration: number;
  exitCode: number | null;
  stdout: string;
  stderr: string;
  status: 'PASS' | 'FAIL' | 'PARTIAL' | 'BLOCKED';
  changedFiles: string[];
  artifacts: string[];
}

export interface ScoreboardCategory {
  name: string;
  score: number;
  status: 'PASS' | 'FAIL' | 'PARTIAL' | 'BLOCKED';
  proofCommand: string;
  proofArtifact: string;
  reason: string;
  nextAction: string;
  lastChecked: string;
}

export interface ScannerFinding {
  file: string;
  line: number;
  matched: string;
  severity: 'INFO' | 'WARNING' | 'HIGH' | 'CRITICAL';
  reason: string;
  suggestedFix: string;
}

export interface ProjectDetection {
  hasCargo: boolean;
  hasPackageJson: boolean;
  hasHardhat: boolean;
  hasFoundry: boolean;
  hasAnchor: boolean;
  hasSubstrate: boolean;
  hasX3Files: boolean;
  hasPallets: boolean;
  hasContracts: boolean;
  hasX3Lang: boolean;
  hasRelayer: boolean;
  hasAdapters: boolean;
  hasProofLedger: boolean;
  hasValidator: boolean;
  hasDocker: boolean;
  hasGit: boolean;
  modules: string[];
}

export interface DebuggerBreakpoint {
  id: string;
  file: string;
  line: number;
  enabled: boolean;
  condition?: string;
}

export interface DiagnosticEntry {
  file: string;
  line: number;
  column: number;
  message: string;
  severity: 'error' | 'warning' | 'info' | 'hint';
  source: 'cargo' | 'forge' | 'tsc' | 'eslint' | 'scanner';
}

export interface ChainState {
  connected: boolean;
  chainId: string;
  blockNumber: number;
  latency: number;
  lastChecked: string;
  rpcUrl: string;
}

export interface ExtensionPanel {
  id: string;
  label: string;
  icon: string;
  component: string;
  description: string;
  version: string;
}

export interface Keybinding {
  id: string;
  label: string;
  keys: string;
  command: string;
  when?: string;
}

export interface VerificationProgress {
  step: string;
  steps: string[];
  current: number;
  total: number;
  status: 'running' | 'done' | 'failed';
}

export interface AiConversation {
  id: string;
  mode: string;
  messages: { role: string; content: string }[];
  created: string;
  updated: string;
}

export interface NetworkRequest {
  id: string;
  url: string;
  method: string;
  status: number;
  duration: number;
  timestamp: string;
  body?: string;
  response?: string;
}

export interface ForgeCoverageResult {
  lines: { total: number; covered: number; pct: number };
  branches: { total: number; covered: number; pct: number };
  functions: { total: number; covered: number; pct: number };
  files: { file: string; pct: number }[];
}

export interface GasProfileEntry {
  id: string;
  method: string;
  contract: string;
  gasUsed: string;
  gasPrice: string;
  cost: string;
  timestamp: string;
}

export interface TpsBenchmarkResult {
  id: string;
  method: string;
  chain: string;
  requests: number;
  duration: number;
  tps: number;
  errors: number;
  latencyAvg: number;
  latencyP95: number;
  timestamp: string;
}

export interface DeploymentConfig {
  name: string;
  chain: string;
  rpcUrl: string;
  contract: string;
  bytecode: string;
  abi: string;
  constructorArgs: string[];
  gasLimit: string;
  timestamp: string;
}

export interface DaoProposal {
  title: string;
  description: string;
  actions: { target: string; value: string; data: string }[];
  votingPeriod: number;
  quorum: number;
  proposer: string;
}

export interface AccountAbstractionWallet {
  address: string;
  owner: string;
  guardians: string[];
  threshold: number;
  deployed: boolean;
}

export interface CrossChainTx {
  sourceChain: string;
  destinationChain: string;
  sourceTx: string;
  destinationTx: string;
  amount: string;
  token: string;
  status: 'pending' | 'relayed' | 'confirmed' | 'failed';
  timestamp: string;
}

export interface ChainConfig {
  name: string;
  chainId: number;
  rpcUrl: string;
  explorerUrl: string;
  currency: string;
  type: 'evm' | 'svm' | 'substrate' | 'cosmos';
}

export interface RegistryPackage {
  name: string;
  version: string;
  description: string;
  author: string;
  downloads: number;
  license: string;
  homepage: string;
  repository: string;
  keywords: string[];
  panels: string[];
}

export interface VsCodeKeybinding {
  key: string;
  command: string;
  when?: string;
  args?: any;
}

export interface CollabSession {
  id: string;
  room: string;
  peers: number;
  connected: boolean;
  host: string;
  lastSync: string;
}

export interface SolidityCompilerInput {
  sources: Record<string, { content: string }>;
  settings?: {
    optimizer?: { enabled: boolean; runs: number };
    remappings?: string[];
    viaIR?: boolean;
    evmVersion?: string;
    outputSelection?: Record<string, Record<string, string[]>>;
  };
}

export interface SolidityCompilerOutput {
  errors?: { severity: string; message: string; sourceLocation?: { file: string; start: number; end: number } }[];
  contracts?: Record<string, Record<string, { abi: any; evm: { bytecode: { object: string }; deployedBytecode: { object: string } } }>>;
  sources?: Record<string, { id: number }>;
}

export interface WasmModule {
  path: string;
  size: number;
  imports: { module: string; name: string; kind: string }[];
  exports: { name: string; kind: string }[];
  sections: { name: string; size: number }[];
  functions: number;
  memories: number;
  tables: number;
}

export interface TpsMeterSnapshot {
  blockNumber: number;
  timestamp: number;
  tps: number;
  txCount: number;
}

export interface FeatureFlag {
  name: string;
  enabled: boolean;
  description: string;
}

declare global {
  interface Window { x3studio: X3StudioAPI }
}

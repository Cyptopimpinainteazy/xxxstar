import { create } from 'zustand';
import { AgentEntityType } from '../scene/EntityFactory';
import { invoke } from '../ipc/tauri';

export interface AgentPosition {
  x: number;
  y: number;
  z: number;
}

export interface AgentState {
  id: string;
  name: string;
  health: number;
  pnl: number;
  xp: number;
  color: string;
  entityType: AgentEntityType;
  position: AgentPosition;
  status: 'idle' | 'attacking' | 'defending' | 'strategy_executing' | 'hedging';
  lastAction: string;
  strategyId: string;
  chain: string;
}

interface AgentStore {
  agents: AgentState[];
  setAgents: (agents: AgentState[]) => void;
  updateAgent: (id: string, partial: Partial<AgentState>) => void;
  updateAgentHealth: (id: string, delta: number) => void;
  updateAgentPnl: (id: string, delta: number) => void;
  updateAgentPosition: (id: string, pos: AgentPosition) => void;
  setAgentStatus: (id: string, status: AgentState['status']) => void;
  getInitialState: () => { agents: AgentState[] };
}

// ── Default seeded agents — fallback when node is unreachable ──
export const DEFAULT_AGENTS: AgentState[] = [
  {
    id: 'agent-1', name: 'Alpha Arbitrage', health: 100, pnl: 24580.50, xp: 1500,
    color: '#00ccff', entityType: 'diamond', position: { x: -4, y: 0, z: 0 },
    status: 'idle', lastAction: 'Flashloan swap on Uniswap V3', strategyId: 'strat-arb-1', chain: 'ethereum',
  },
  {
    id: 'agent-2', name: 'Gamma Yield', health: 100, pnl: 12340.75, xp: 1100,
    color: '#00ff88', entityType: 'sphere', position: { x: 4, y: 0, z: 0 },
    status: 'idle', lastAction: 'Harvested x3LP rewards', strategyId: 'strat-yield-1', chain: 'ethereum',
  },
  {
    id: 'agent-3', name: 'Delta Hedge', health: 100, pnl: 8920.30, xp: 800,
    color: '#ff8800', entityType: 'icosahedron', position: { x: 0, y: 0, z: -4 },
    status: 'idle', lastAction: 'Opened GLP short position', strategyId: 'strat-hedge-1', chain: 'arbitrum',
  },
  {
    id: 'agent-4', name: 'Omega MEV', health: 100, pnl: 45230.00, xp: 3200,
    color: '#ff3366', entityType: 'torus', position: { x: 0, y: 0, z: 4 },
    status: 'idle', lastAction: 'Sandwich attack on GMX', strategyId: 'strat-mev-1', chain: 'ethereum',
  },
  {
    id: 'agent-5', name: 'Sigma LPs', health: 100, pnl: 6730.15, xp: 600,
    color: '#aa66ff', entityType: 'cylinder', position: { x: -6, y: 0, z: -4 },
    status: 'idle', lastAction: 'Added liquidity to Curve tricrypto', strategyId: 'strat-lp-1', chain: 'polygon',
  },
  {
    id: 'agent-6', name: 'Zeta Governance', health: 100, pnl: 1520.00, xp: 400,
    color: '#ffff44', entityType: 'cone', position: { x: 6, y: 0, z: -4 },
    status: 'idle', lastAction: 'Voted on AIP-42 proposal', strategyId: 'strat-gov-1', chain: 'ethereum',
  },
];

// ── Map swarm nodes from Tauri backend into AgentState ──
interface SwarmNodeRaw {
  id: string;
  name: string;
  status: string;
  gpuUtil: number;
  vramUsed: number;
  vramCapacity: number;
  temperature: number;
  uptimeHours: number;
  sla: number;
  jobs: number;
}

interface SwarmHealthResponse {
  summary: { online_nodes: number; total_nodes: number; avg_gpu_util: number; total_vram_used: number; total_vram_capacity: number; queued_jobs: number };
  nodes: SwarmNodeRaw[];
}

const ENTITY_TYPES: AgentEntityType[] = ['diamond', 'sphere', 'icosahedron', 'torus', 'cylinder', 'cone'];
const STRATEGY_IDS = ['strat-arb-1', 'strat-yield-1', 'strat-hedge-1', 'strat-mev-1', 'strat-lp-1', 'strat-gov-1'];
const COLORS = ['#00ccff', '#00ff88', '#ff8800', '#ff3366', '#aa66ff', '#ffff44'];

function swarmNodeToAgent(node: SwarmNodeRaw, index: number): AgentState {
  const statusMap: Record<string, AgentState['status']> = {
    online: 'strategy_executing',
    idle: 'idle',
    offline: 'defending',
    slashed: 'defending',
  };
  return {
    id: node.id,
    name: node.name,
    health: Math.min(100, node.sla),
    pnl: node.jobs * 1000 + node.vramUsed / 1_000_000,
    xp: node.uptimeHours,
    color: COLORS[index % COLORS.length],
    entityType: ENTITY_TYPES[index % ENTITY_TYPES.length],
    position: {
      x: (Math.cos((index / 12) * Math.PI * 2) * 4),
      y: 0,
      z: (Math.sin((index / 12) * Math.PI * 2) * 4),
    },
    status: statusMap[node.status] || 'idle',
    lastAction: `GPU ${node.gpuUtil}% — ${node.jobs} jobs, ${node.sla}% SLA`,
    strategyId: STRATEGY_IDS[index % STRATEGY_IDS.length],
    chain: 'x3',
  };
}

// ── Init: pull live swarm health on module load ──
async function initFromSwarmHealth(): Promise<void> {
  try {
    const data = await invoke<SwarmHealthResponse>('launch_swarm_health');
    if (data?.nodes && data.nodes.length > 0) {
      const agents: AgentState[] = data.nodes.map((n, i) => swarmNodeToAgent(n, i));
      useAgentStore.getState().setAgents(agents);
    }
    // else: keep defaults — node unreachable
  } catch {
    // Node not reachable, keep DEFAULT_AGENTS seeded
  }
}

if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
  initFromSwarmHealth();
}

export const useAgentStore = create<AgentStore>()((set, _get) => ({
  agents: DEFAULT_AGENTS,
  getInitialState: () => ({ agents: DEFAULT_AGENTS }),

  setAgents: (agents) => set({ agents }),

  updateAgent: (id, partial) =>
    set((state) => ({
      agents: state.agents.map((a) =>
        a.id === id ? { ...a, ...partial } : a
      ),
    })),

  updateAgentHealth: (id, delta) =>
    set((state) => ({
      agents: state.agents.map((a) =>
        a.id === id
          ? { ...a, health: Math.max(0, Math.min(100, a.health + delta)) }
          : a
      ),
    })),

  updateAgentPnl: (id, delta) =>
    set((state) => ({
      agents: state.agents.map((a) =>
        a.id === id ? { ...a, pnl: a.pnl + delta } : a
      ),
    })),

  updateAgentPosition: (id, pos) =>
    set((state) => ({
      agents: state.agents.map((a) =>
        a.id === id ? { ...a, position: pos } : a
      ),
    })),

  setAgentStatus: (id, status) =>
    set((state) => ({
      agents: state.agents.map((a) =>
        a.id === id ? { ...a, status } : a
      ),
    })),
}));
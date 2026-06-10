import { create } from 'zustand';
import { AgentEntityType } from '../scene/EntityFactory';

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

export const DEFAULT_AGENTS: AgentState[] = [
  {
    id: 'agent-1',
    name: 'Alpha Arbitrage',
    health: 100,
    pnl: 24580.50,
    xp: 1500,
    color: '#00ccff',
    entityType: 'diamond',
    position: { x: -4, y: 0, z: 0 },
    status: 'idle',
    lastAction: 'Flashloan swap on Uniswap V3',
    strategyId: 'strat-arb-1',
    chain: 'ethereum',
  },
  {
    id: 'agent-2',
    name: 'Gamma Yield',
    health: 100,
    pnl: 12340.75,
    xp: 1100,
    color: '#00ff88',
    entityType: 'sphere',
    position: { x: 4, y: 0, z: 0 },
    status: 'idle',
    lastAction: 'Harvested x3LP rewards',
    strategyId: 'strat-yield-1',
    chain: 'ethereum',
  },
  {
    id: 'agent-3',
    name: 'Delta Hedge',
    health: 100,
    pnl: 8920.30,
    xp: 800,
    color: '#ff8800',
    entityType: 'icosahedron',
    position: { x: 0, y: 0, z: -4 },
    status: 'idle',
    lastAction: 'Opened GLP short position',
    strategyId: 'strat-hedge-1',
    chain: 'arbitrum',
  },
  {
    id: 'agent-4',
    name: 'Omega MEV',
    health: 100,
    pnl: 45230.00,
    xp: 3200,
    color: '#ff3366',
    entityType: 'torus',
    position: { x: 0, y: 0, z: 4 },
    status: 'idle',
    lastAction: 'Sandwich attack on GMX',
    strategyId: 'strat-mev-1',
    chain: 'ethereum',
  },
  {
    id: 'agent-5',
    name: 'Sigma LPs',
    health: 100,
    pnl: 6730.15,
    xp: 600,
    color: '#aa66ff',
    entityType: 'cylinder',
    position: { x: -6, y: 0, z: -4 },
    status: 'idle',
    lastAction: 'Added liquidity to Curve tricrypto',
    strategyId: 'strat-lp-1',
    chain: 'polygon',
  },
  {
    id: 'agent-6',
    name: 'Zeta Governance',
    health: 100,
    pnl: 1520.00,
    xp: 400,
    color: '#ffff44',
    entityType: 'cone',
    position: { x: 6, y: 0, z: -4 },
    status: 'idle',
    lastAction: 'Voted on AIP-42 proposal',
    strategyId: 'strat-gov-1',
    chain: 'ethereum',
  },
];

export const useAgentStore = create<AgentStore>()((set, get) => ({
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
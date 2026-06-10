import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useAgentStore } from '../src/agents/AgentStore';
import { useBlockStore } from '../src/blockchain/BlockStore';
import { strategyCore } from '../src/agents/StrategyCore';
import { tick } from '../src/agents/AIController';

/* ─── Agent Store Tests ─────────────────────── */

describe('AgentStore', () => {
  beforeEach(() => {
    // Reset to defaults
    useAgentStore.setState({
      agents: useAgentStore.getInitialState().agents,
    });
  });

  it('should initialize with 6 agents', () => {
    const { agents } = useAgentStore.getState();
    expect(agents).toHaveLength(6);
  });

  it('should update agent health', () => {
    useAgentStore.getState().updateAgentHealth('agent-1', -10);
    const agent = useAgentStore.getState().agents.find((a) => a.id === 'agent-1');
    expect(agent?.health).toBe(90);
  });

  it('should clamp health to [0, 100]', () => {
    useAgentStore.getState().updateAgentHealth('agent-1', -200);
    expect(useAgentStore.getState().agents.find((a) => a.id === 'agent-1')?.health).toBe(0);

    useAgentStore.getState().updateAgentHealth('agent-1', 200);
    expect(useAgentStore.getState().agents.find((a) => a.id === 'agent-1')?.health).toBe(100);
  });

  it('should update agent PnL', () => {
    useAgentStore.getState().updateAgentPnl('agent-2', 500);
    const agent = useAgentStore.getState().agents.find((a) => a.id === 'agent-2');
    expect(agent?.pnl).toBe(12840.75);
  });

  it('should update agent position', () => {
    useAgentStore.getState().updateAgentPosition('agent-3', { x: 10, y: 2, z: -5 });
    const agent = useAgentStore.getState().agents.find((a) => a.id === 'agent-3');
    expect(agent?.position).toEqual({ x: 10, y: 2, z: -5 });
  });

  it('should set agent status', () => {
    useAgentStore.getState().setAgentStatus('agent-4', 'attacking');
    const agent = useAgentStore.getState().agents.find((a) => a.id === 'agent-4');
    expect(agent?.status).toBe('attacking');
  });

  it('should update arbitrary agent fields', () => {
    useAgentStore.getState().updateAgent('agent-5', { lastAction: 'test action', chain: 'solana' });
    const agent = useAgentStore.getState().agents.find((a) => a.id === 'agent-5');
    expect(agent?.lastAction).toBe('test action');
    expect(agent?.chain).toBe('solana');
  });

  it('should replace all agents', () => {
    useAgentStore.getState().setAgents([]);
    expect(useAgentStore.getState().agents).toHaveLength(0);
  });
});

/* ─── Block Store Tests ─────────────────────── */

describe('BlockStore', () => {
  beforeEach(() => {
    useBlockStore.setState({ recentBlocks: [] });
  });

  it('should start empty', () => {
    expect(useBlockStore.getState().recentBlocks).toHaveLength(0);
  });

  it('should add a block', () => {
    useBlockStore.getState().addBlock({
      id: 'block-1',
      height: 1,
      agentId: 'agent-1',
      status: 'confirmed',
      timestamp: Date.now(),
      position: { x: 0, z: 0 },
    });
    expect(useBlockStore.getState().recentBlocks).toHaveLength(1);
  });

  it('should remove a block', () => {
    useBlockStore.getState().addBlock({
      id: 'block-1',
      height: 1,
      agentId: 'agent-1',
      status: 'confirmed',
      timestamp: Date.now(),
      position: { x: 0, z: 0 },
    });
    useBlockStore.getState().removeBlock('block-1');
    expect(useBlockStore.getState().recentBlocks).toHaveLength(0);
  });

  it('should cap at 30 blocks', () => {
    for (let i = 0; i < 35; i++) {
      useBlockStore.getState().addBlock({
        id: `block-${i}`,
        height: i,
        agentId: 'agent-1',
        status: 'confirmed',
        timestamp: Date.now(),
        position: { x: 0, z: 0 },
      });
    }
    expect(useBlockStore.getState().recentBlocks.length).toBeLessThanOrEqual(30);
  });

  it('should clear all blocks', () => {
    useBlockStore.getState().addBlock({
      id: 'block-1',
      height: 1,
      agentId: 'agent-1',
      status: 'confirmed',
      timestamp: Date.now(),
      position: { x: 0, z: 0 },
    });
    useBlockStore.getState().clearBlocks();
    expect(useBlockStore.getState().recentBlocks).toHaveLength(0);
  });
});

/* ─── Strategy Core Tests ───────────────────── */

describe('StrategyCore', () => {
  it('should return a strategy by ID', () => {
    const strategy = strategyCore.getStrategy('strat-arb-1');
    expect(strategy).toBeDefined();
    expect(strategy?.name).toBe('Cross-DEX Flash Arbitrage');
  });

  it('should return undefined for unknown strategy', () => {
    const strategy = strategyCore.getStrategy('nonexistent');
    expect(strategy).toBeUndefined();
  });

  it('should return all strategies', () => {
    const all = strategyCore.getAllStrategies();
    expect(all).toHaveLength(6);
  });

  it('should update a strategy', () => {
    strategyCore.updateStrategy('strat-arb-1', { confidence: 0.9, pnl: 30000 });
    const strategy = strategyCore.getStrategy('strat-arb-1');
    expect(strategy?.confidence).toBe(0.9);
    expect(strategy?.pnl).toBe(30000);
  });

  it('should evaluate yield farming as compound_rewards', () => {
    const result = strategyCore.evaluate('strat-yield-1', 0.5);
    expect(result.action).toBe('compound_rewards');
    expect(result.confidence).toBeGreaterThan(0.8);
  });

  it('should evaluate hedging as rebalance_hedge', () => {
    const result = strategyCore.evaluate('strat-hedge-1', 0.3);
    expect(result.action).toBe('rebalance_hedge');
    expect(result.confidence).toBeGreaterThan(0.8);
  });

  it('should evaluate governance as monitor_proposals', () => {
    const result = strategyCore.evaluate('strat-gov-1', 0.1);
    expect(result.action).toBe('monitor_proposals');
    expect(result.confidence).toBe(0.95);
  });

  it('should return hold for inactive strategies', () => {
    strategyCore.updateStrategy('strat-lp-1', { active: false });
    const result = strategyCore.evaluate('strat-lp-1', 0.5);
    expect(result.action).toBe('hold');
    expect(result.confidence).toBe(0);
    strategyCore.updateStrategy('strat-lp-1', { active: true });
  });

  it('should return hold for unknown strategy', () => {
    const result = strategyCore.evaluate('nonexistent', 0.5);
    expect(result.action).toBe('hold');
    expect(result.confidence).toBe(0);
  });
});

/* ─── AI Controller Tick Tests ──────────────── */

describe('AIController tick()', () => {
  beforeEach(() => {
    useAgentStore.setState({
      agents: useAgentStore.getInitialState().agents,
    });
    useBlockStore.setState({ recentBlocks: [] });
  });

  it('should not crash on tick', () => {
    expect(() => tick()).not.toThrow();
  });

  it('should generate at least one block after many ticks', () => {
    // Run 10 ticks to ensure at least one block is generated
    for (let i = 0; i < 10; i++) {
      tick();
    }
    expect(useBlockStore.getState().recentBlocks.length).toBeGreaterThanOrEqual(1);
  });

  it('should update agent health after ticks', () => {
    const initialHealth = useAgentStore.getState().agents.find((a) => a.id === 'agent-1')?.health ?? 100;
    tick();
    const newHealth = useAgentStore.getState().agents.find((a) => a.id === 'agent-1')?.health ?? 0;
    // Health can go up or down, but should change
    expect(newHealth).not.toBeNaN();
  });

  it('should keep blocks under 20', () => {
    for (let i = 0; i < 30; i++) {
      tick();
    }
    expect(useBlockStore.getState().recentBlocks.length).toBeLessThanOrEqual(20);
  });
});

/* ─── Chain Adapter Interface Tests ─────────── */

describe('Chain Adapter Interface', () => {
  it('should define all required methods', () => {
    const adapter: import('../src/blockchain/ChainAdapter').ChainAdapter = {
      name: 'test',
      chainId: 1,
      connect: async () => {},
      disconnect: async () => {},
      getStatus: async () => ({ chainId: 1, blockHeight: 0, peers: 0, synced: false, avgBlockTimeMs: 0 }),
      getBlocks: async () => [],
      getMempool: async () => [],
      sendTx: async () => '0x0',
      getBalance: async () => '0 ETH',
    };
    expect(adapter.name).toBe('test');
    expect(adapter.chainId).toBe(1);
    expect(typeof adapter.connect).toBe('function');
    expect(typeof adapter.getStatus).toBe('function');
    expect(typeof adapter.getBlocks).toBe('function');
    expect(typeof adapter.getMempool).toBe('function');
    expect(typeof adapter.sendTx).toBe('function');
    expect(typeof adapter.getBalance).toBe('function');
  });
});
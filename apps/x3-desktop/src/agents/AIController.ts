import { useAgentStore } from './AgentStore';
import { strategyCore } from './StrategyCore';
import { useBlockStore } from '../blockchain/BlockStore';

/**
 * AIController — runs agent behavioral ticks.
 * Evaluates strategies, moves agents in the arena, generates blocks/txs.
 * Call `tick()` on an interval from the app's lifecycle.
 */

let tickInterval: ReturnType<typeof setInterval> | null = null;
let marketCondition = 0.5;

function randomPosition(): { x: number; y: number; z: number } {
  const range = 9;
  return {
    x: (Math.random() - 0.5) * range * 2,
    y: (Math.random() - 0.5) * range * 0.3,
    z: (Math.random() - 0.5) * range * 2,
  };
}

function doStrategyAction(action: string): string {
  switch (action) {
    case 'execute_arbitrage':
      return 'Executed flashloan arbitrage — profit +0.32 ETH';
    case 'scan_for_opportunities':
      return 'Scanning DEX pairs for spread > 0.5%';
    case 'compound_rewards':
      return 'Compounded x3LP rewards → +$42.50';
    case 'rebalance_hedge':
      return 'Rebalanced delta hedge ratio to 0.95';
    case 'scan_mempool':
      return 'Found MEV opportunity — submitting bundle';
    case 'adjust_range':
      return 'Adjusted LP tick range to [500, 1500]';
    case 'monitor_proposals':
      return 'New proposal detected: XIP-7';
    case 'check_arb_opportunity':
      return 'Arbitrage window detected on ETH/BTC';
    case 'bridge_monitor':
      return 'Cross-chain bridge capacity: 72%';
    case 'hold':
    case 'wait':
    default:
      return 'Monitoring market conditions';
  }
}

/**
 * Advance one AI tick — updates agent states, strategies, and block flow.
 */
export function tick(): void {
  // Update market condition with random walk
  marketCondition = Math.max(0, Math.min(1, marketCondition + (Math.random() - 0.5) * 0.1));

  const agents = useAgentStore.getState().agents;
  const store = useAgentStore.getState();

  for (const agent of agents) {
    const strategy = strategyCore.getStrategy(agent.strategyId);
    if (!strategy) continue;

    // Evaluate strategy
    const result = strategyCore.evaluate(agent.strategyId, marketCondition);

    // Update agent status
    const newStatus = result.action === 'hold' || result.action === 'wait'
      ? 'idle'
      : result.action.includes('arbitrage') || result.action === 'scan_mempool'
        ? 'attacking'
        : result.action.includes('hedge')
          ? 'hedging'
          : 'strategy_executing';

    store.setAgentStatus(agent.id, newStatus);

    // Random movement
    if (Math.random() < 0.3) {
      store.updateAgentPosition(agent.id, randomPosition());
    }

    // Health fluctuates
    store.updateAgentHealth(agent.id, Math.random() < 0.3 ? -2 : Math.random() < 0.2 ? 3 : 0);

    // PnL updates
    const pnlDelta = (Math.random() - 0.48) * (result.confidence * 100);
    store.updateAgentPnl(agent.id, pnlDelta);

    // Last action
    store.updateAgent(agent.id, {
      lastAction: doStrategyAction(result.action),
    });

    // Update strategy
    strategyCore.updateStrategy(agent.strategyId, {
      confidence: result.confidence,
      nextAction: result.action,
    });
  }

  // Generate a "block" (falling cube in arena)
  const blockStore = useBlockStore.getState();
  const randomAgent = agents[Math.floor(Math.random() * agents.length)];
  if (randomAgent && blockStore.recentBlocks.length < 20) {
    blockStore.addBlock({
      id: `block-${Date.now()}`,
      height: blockStore.recentBlocks.length + 1,
      agentId: randomAgent.id,
      status: Math.random() < 0.7 ? 'confirmed' : Math.random() < 0.9 ? 'pending' : 'failed',
      timestamp: Date.now(),
      position: {
        x: randomAgent.position.x + (Math.random() - 0.5) * 2,
        z: randomAgent.position.z + (Math.random() - 0.5) * 2,
      },
    });
  }
}

/**
 * Start the AI tick loop — runs every 2 seconds.
 */
export function startAIController(): void {
  if (tickInterval) return;
  tickInterval = setInterval(tick, 2000);
}

/**
 * Stop the AI tick loop.
 */
export function stopAIController(): void {
  if (tickInterval) {
    clearInterval(tickInterval);
    tickInterval = null;
  }
}

/**
 * Toggle the AI controller on/off.
 */
export function toggleAIController(): boolean {
  if (tickInterval) {
    stopAIController();
    return false;
  } else {
    startAIController();
    return true;
  }
}
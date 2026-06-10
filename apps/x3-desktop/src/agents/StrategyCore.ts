export type StrategyType =
  | 'arbitrage'
  | 'yield_farming'
  | 'hedging'
  | 'mev'
  | 'liquidity_provision'
  | 'governance'
  | 'flashloan'
  | 'cross_chain';

export type RiskProfile = 'low' | 'medium' | 'high' | 'degen';

export interface StrategyState {
  id: string;
  name: string;
  type: StrategyType;
  riskProfile: RiskProfile;
  chain: string;
  active: boolean;
  pnl: number;
  totalTrades: number;
  winRate: number;
  sharpeRatio: number;
  maxDrawdown: number;
  capitalDeployed: number;
  lastEvaluation: string;
  nextAction: string;
  confidence: number; // 0–1
}

/**
 * StrategyCore — embedded AI strategy evaluation engine.
 * Each agent links to a strategy that drives its behavior in the arena.
 */
export class StrategyCore {
  private strategies: Map<string, StrategyState> = new Map();

  constructor() {
    this.initializeDefaults();
  }

  private initializeDefaults() {
    const defaults: StrategyState[] = [
      {
        id: 'strat-arb-1',
        name: 'Cross-DEX Flash Arbitrage',
        type: 'arbitrage',
        riskProfile: 'high',
        chain: 'ethereum',
        active: true,
        pnl: 24580.50,
        totalTrades: 342,
        winRate: 0.68,
        sharpeRatio: 2.1,
        maxDrawdown: 0.12,
        capitalDeployed: 50000,
        lastEvaluation: new Date().toISOString(),
        nextAction: 'Monitor Uni V3 / Sushi spread on ETH/USDC',
        confidence: 0.75,
      },
      {
        id: 'strat-yield-1',
        name: 'x3LP Yield Optimizer',
        type: 'yield_farming',
        riskProfile: 'medium',
        chain: 'ethereum',
        active: true,
        pnl: 12340.75,
        totalTrades: 89,
        winRate: 0.91,
        sharpeRatio: 3.4,
        maxDrawdown: 0.05,
        capitalDeployed: 25000,
        lastEvaluation: new Date().toISOString(),
        nextAction: 'Compound x3LP rewards into AutoVault',
        confidence: 0.88,
      },
      {
        id: 'strat-hedge-1',
        name: 'Delta-Neutral Hedge',
        type: 'hedging',
        riskProfile: 'low',
        chain: 'arbitrum',
        active: true,
        pnl: 8920.30,
        totalTrades: 56,
        winRate: 0.85,
        sharpeRatio: 2.8,
        maxDrawdown: 0.03,
        capitalDeployed: 40000,
        lastEvaluation: new Date().toISOString(),
        nextAction: 'Rebalance GLP short / long ratio',
        confidence: 0.92,
      },
      {
        id: 'strat-mev-1',
        name: 'Mempool MEV Extraction',
        type: 'mev',
        riskProfile: 'degen',
        chain: 'ethereum',
        active: true,
        pnl: 45230.00,
        totalTrades: 1201,
        winRate: 0.54,
        sharpeRatio: 1.6,
        maxDrawdown: 0.22,
        capitalDeployed: 80000,
        lastEvaluation: new Date().toISOString(),
        nextAction: 'Scan pending txs for sandwich opportunities',
        confidence: 0.60,
      },
      {
        id: 'strat-lp-1',
        name: 'Concentrated LP Strategy',
        type: 'liquidity_provision',
        riskProfile: 'medium',
        chain: 'polygon',
        active: true,
        pnl: 6730.15,
        totalTrades: 23,
        winRate: 0.95,
        sharpeRatio: 4.1,
        maxDrawdown: 0.04,
        capitalDeployed: 15000,
        lastEvaluation: new Date().toISOString(),
        nextAction: 'Adjust tick range on MATIC/USDC pool',
        confidence: 0.85,
      },
      {
        id: 'strat-gov-1',
        name: 'Governance Arbitrage',
        type: 'governance',
        riskProfile: 'low',
        chain: 'ethereum',
        active: true,
        pnl: 1520.00,
        totalTrades: 12,
        winRate: 1.0,
        sharpeRatio: 5.2,
        maxDrawdown: 0.01,
        capitalDeployed: 5000,
        lastEvaluation: new Date().toISOString(),
        nextAction: 'Monitor AIP-42 proposal outcome',
        confidence: 0.95,
      },
    ];

    for (const s of defaults) {
      this.strategies.set(s.id, s);
    }
  }

  getStrategy(id: string): StrategyState | undefined {
    return this.strategies.get(id);
  }

  getAllStrategies(): StrategyState[] {
    return Array.from(this.strategies.values());
  }

  updateStrategy(id: string, partial: Partial<StrategyState>): void {
    const existing = this.strategies.get(id);
    if (existing) {
      this.strategies.set(id, { ...existing, ...partial, lastEvaluation: new Date().toISOString() });
    }
  }

  /**
   * Evaluate the strategy given current market conditions.
   * Returns a recommended action and confidence score.
   */
  evaluate(id: string, marketCondition: number): { action: string; confidence: number } {
    const s = this.strategies.get(id);
    if (!s) {
      return { action: 'hold', confidence: 0 };
    }
    if (!s.active) {
      return { action: 'hold', confidence: 0 };
    }

    // Simple evaluation based on strategy type and market condition
    switch (s.type) {
      case 'arbitrage':
        if (marketCondition > 0.3) {
          return { action: 'execute_arbitrage', confidence: 0.7 + Math.random() * 0.2 };
        }
        return { action: 'scan_for_opportunities', confidence: 0.4 };
      case 'yield_farming':
        return { action: 'compound_rewards', confidence: 0.85 };
      case 'hedging':
        return { action: 'rebalance_hedge', confidence: 0.9 };
      case 'mev':
        if (marketCondition > 0.5) {
          return { action: 'scan_mempool', confidence: 0.6 + Math.random() * 0.3 };
        }
        return { action: 'wait', confidence: 0.2 };
      case 'liquidity_provision':
        return { action: 'adjust_range', confidence: 0.8 };
      case 'governance':
        return { action: 'monitor_proposals', confidence: 0.95 };
      case 'flashloan':
        return { action: 'check_arb_opportunity', confidence: 0.5 + Math.random() * 0.4 };
      case 'cross_chain':
        return { action: 'bridge_monitor', confidence: 0.7 };
      default:
        return { action: 'hold', confidence: 0.5 };
    }
  }
}

/** Singleton instance */
export const strategyCore = new StrategyCore();
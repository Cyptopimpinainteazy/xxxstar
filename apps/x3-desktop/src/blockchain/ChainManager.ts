/**
 * Chain Manager — manages blockchain adapter selection and lifecycle.
 *
 * Ensures real blockchain connections are used instead of mock data.
 * Routes to appropriate adapters based on chain configuration.
 */

import { ChainAdapter, ChainStatus, Block, Tx, SignedTx, TxHash } from './ChainAdapter';
import { LocalAdapter } from './LocalAdapter';
import { EthereumAdapter } from './EthereumAdapter';

/**
 * Chain configuration
 */
export interface ChainConfig {
  id: string;
  name: string;
  enabled: boolean;
  rpcUrl?: string;
  wsUrl?: string;
}

/**
 * Available chains
 */
export const AVAILABLE_CHAINS: ChainConfig[] = [
  {
    id: 'x3-local',
    name: 'X3 Local Dev',
    enabled: true,
    rpcUrl: 'http://rpc.testnet.x3-chain.io:9944',
    wsUrl: 'ws://rpc.testnet.x3-chain.io:9944',
  },
  {
    id: 'ethereum',
    name: 'Ethereum Mainnet',
    enabled: true,
    rpcUrl: 'https://eth.llamarpc.com',
    wsUrl: 'wss://eth.llamarpc.com',
  },
  {
    id: 'x3-testnet',
    name: 'X3 Testnet',
    enabled: true,
    rpcUrl: 'https://testnet.x3star.net',
    wsUrl: 'wss://testnet.x3star.net',
  },
];

/**
 * Chain Manager
 */
export class ChainManager {
  private adapters: Map<string, ChainAdapter> = new Map();
  private activeChainId: string | null = null;

  /**
   * Initialize all available chain adapters
   */
  async initialize(): Promise<void> {
    console.log('Initializing real blockchain connections...');

    // Initialize X3 Local adapter
    const localAdapter = new LocalAdapter();
    await this.registerAdapter('x3-local', localAdapter);

    // Initialize Ethereum adapter
    const ethAdapter = new EthereumAdapter();
    await this.registerAdapter('ethereum', ethAdapter);

    console.log(`Initialized ${this.adapters.size} blockchain adapters`);
  }

  /**
   * Register a chain adapter
   */
  async registerAdapter(chainId: string, adapter: ChainAdapter): Promise<void> {
    try {
      await adapter.connect();
      this.adapters.set(chainId, adapter);
      console.log(`✅ Connected to ${chainId}: ${adapter.name}`);
    } catch (error) {
      console.error(`❌ Failed to connect to ${chainId}:`, error);
      throw new Error(`Failed to connect to ${chainId}: ${error}`);
    }
  }

  /**
   * Get adapter by chain ID
   */
  getAdapter(chainId: string): ChainAdapter | undefined {
    return this.adapters.get(chainId);
  }

  /**
   * Get all adapters
   */
  getAllAdapters(): ChainAdapter[] {
    return Array.from(this.adapters.values());
  }

  /**
   * Set active chain
   */
  setActiveChain(chainId: string): void {
    if (!this.adapters.has(chainId)) {
      throw new Error(`Chain ${chainId} not available`);
    }
    this.activeChainId = chainId;
    console.log(`Active chain set to: ${chainId}`);
  }

  /**
   * Get active chain adapter
   */
  getActiveAdapter(): ChainAdapter {
    if (!this.activeChainId) {
      // Default to x3-testnet if no active chain
      this.activeChainId = 'x3-testnet';
    }
    const adapter = this.adapters.get(this.activeChainId);
    if (!adapter) {
      throw new Error(`No adapter available for active chain: ${this.activeChainId}`);
    }
    return adapter;
  }

  /**
   * Get active chain ID
   */
  getActiveChainId(): string | null {
    return this.activeChainId;
  }

  /**
   * Get status of all chains
   */
  async getAllStatuses(): Promise<Map<string, ChainStatus>> {
    const statuses = new Map<string, ChainStatus>();

    for (const [chainId, adapter] of this.adapters.entries()) {
      try {
        const status = await adapter.getStatus();
        statuses.set(chainId, status);
      } catch (error) {
        console.error(`Failed to get status for ${chainId}:`, error);
      }
    }

    return statuses;
  }

  /**
   * Disconnect all adapters
   */
  async disconnectAll(): Promise<void> {
    console.log('Disconnecting all blockchain connections...');

    for (const [chainId, adapter] of this.adapters.entries()) {
      try {
        await adapter.disconnect();
        console.log(`Disconnected from ${chainId}`);
      } catch (error) {
        console.error(`Failed to disconnect from ${chainId}:`, error);
      }
    }

    this.adapters.clear();
    this.activeChainId = null;
  }
}

// Singleton instance
let chainManagerInstance: ChainManager | null = null;

/**
 * Get chain manager instance
 */
export function getChainManager(): ChainManager {
  if (!chainManagerInstance) {
    chainManagerInstance = new ChainManager();
  }
  return chainManagerInstance;
}

/**
 * Initialize chain manager
 */
export async function initializeChainManager(): Promise<ChainManager> {
  const manager = getChainManager();
  await manager.initialize();
  return manager;
}
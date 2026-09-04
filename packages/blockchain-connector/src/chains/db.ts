import type { ChainDescriptor } from '../types';
import { GENERATED_CHAIN_REGISTRY } from './generated';

interface ChainIndex {
  byId: Map<string, ChainDescriptor>;
  byChainId: Map<number, ChainDescriptor>;
  byFamilyNetwork: Map<string, ChainDescriptor[]>;
  byName: Map<string, ChainDescriptor[]>;
}

export class ChainDB {
  private chains: ChainDescriptor[];
  private indices: ChainIndex;
  private rpcRotation: Map<string, { index: number; endpoints: string[] }>;

  constructor() {
    this.chains = GENERATED_CHAIN_REGISTRY;
    this.indices = this.buildIndices();
    this.rpcRotation = new Map();
    this.initializeRotation();
  }

  private buildIndices(): ChainIndex {
    const byId = new Map();
    const byChainId = new Map();
    const byFamilyNetwork = new Map();
    const byName = new Map();

    for (const chain of this.chains) {
      byId.set(chain.id, chain);
      byChainId.set(chain.chainId, chain);
      const key = `${chain.family}-${chain.network}`;
      const list = byFamilyNetwork.get(key) || [];
      list.push(chain);
      byFamilyNetwork.set(key, list);
      const nameList = byName.get(chain.name.toLowerCase()) || [];
      nameList.push(chain);
      byName.set(chain.name.toLowerCase(), nameList);
    }

    return { byId, byChainId, byFamilyNetwork, byName };
  }

  private initializeRotation() {
    for (const chain of this.chains) {
      if (chain.defaultRpcUrls && chain.defaultRpcUrls.length > 1) {
        this.rpcRotation.set(chain.id, { index: 0, endpoints: chain.defaultRpcUrls });
      }
    }
  }

  getChain(id: string): ChainDescriptor | undefined {
    return this.indices.byId.get(id);
  }

  getChainByChainId(chainId: number): ChainDescriptor | undefined {
    return this.indices.byChainId.get(chainId);
  }

  getChainsByFamilyNetwork(family: string, network: string): ChainDescriptor[] {
    const key = `${family}-${network}`;
    return this.indices.byFamilyNetwork.get(key) || [];
  }

  searchChains(query: string): ChainDescriptor[] {
    const q = query.toLowerCase();
    const byId = this.indices.byId.get(q);
    if (byId) return [byId];

    const byChainId = this.indices.byChainId.get(Number(q));
    if (byChainId) return [byChainId];

    const byName = this.indices.byName.get(q);
    if (byName) return byName.slice(0, 10); // Limit results

    return [];
  }

  getNextRpc(chainId: string): string {
    const rotation = this.rpcRotation.get(chainId);
    if (!rotation || rotation.endpoints.length === 0) {
      const chain = this.getChain(chainId);
      if (!chain || !chain.defaultRpcUrls || chain.defaultRpcUrls.length === 0) {
        throw new Error(`No RPC endpoints for chain ${chainId}`);
      }
      return chain.defaultRpcUrls[0]!;
    }

    const endpoint = rotation.endpoints[rotation.index];
    rotation.index = (rotation.index + 1) % rotation.endpoints.length;
    return endpoint!;
  }

  getAllEvmChains(network?: 'mainnet' | 'testnet' | 'devnet'): ChainDescriptor[] {
    return this.chains.filter(c => c.family === 'evm' && (!network || c.network === network));
  }

  getAllSvmChains(network?: 'mainnet' | 'testnet' | 'devnet'): ChainDescriptor[] {
    return this.chains.filter(c => c.family === 'solana' && (!network || c.network === network));
  }

  updateHealth(endpoint: string, healthy: boolean) {
    // Update health status in chains if needed
    for (const chain of this.chains) {
      if (chain.defaultRpcUrls?.includes(endpoint)) {
        // Could mark chain endpoints as healthy/unhealthy
        console.log(`Health update for ${endpoint}: ${healthy}`);
      }
    }
  }
}

export const chainDB = new ChainDB();

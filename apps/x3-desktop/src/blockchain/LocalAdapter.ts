import { ChainAdapter, ChainStatus, Block, Tx, SignedTx, TxHash } from './ChainAdapter';
import { invoke } from '../ipc/tauri';

/**
 * Local dev chain adapter — connects to a local X3 chain node.
 * Used for development and testing without external RPCs.
 */
export class LocalAdapter implements ChainAdapter {
  readonly name = 'X3 Local Dev';
  readonly chainId = 1337; // local
  private connected = false;

  async connect(): Promise<void> {
    const result = await invoke<string>('connect_chain', { chain: 'local' });
    this.connected = result === 'ok';
  }

  async disconnect(): Promise<void> {
    await invoke<string>('disconnect_chain', { chain: 'local' });
    this.connected = false;
  }

  async getStatus(): Promise<ChainStatus> {
    return invoke<ChainStatus>('fetch_chain_status', { chain: 'local' });
  }

  async getBlocks(limit = 10): Promise<Block[]> {
    return invoke<Block[]>('fetch_blocks', { chain: 'local', limit });
  }

  async getMempool(): Promise<Tx[]> {
    return invoke<Tx[]>('fetch_mempool', { chain: 'local' });
  }

  async sendTx(tx: SignedTx): Promise<TxHash> {
    return invoke<TxHash>('sign_and_send_tx', {
      chain: 'local',
      rawTx: tx.raw,
    });
  }

  async getBalance(address: string): Promise<string> {
    return invoke<string>('get_balance', { chain: 'local', address });
  }
}

/**
 * Mock adapter for testing the arena without a running node.
 * Generates fake blocks, status, and mempool data.
 */
export class MockAdapter implements ChainAdapter {
  readonly name = 'Mock Chain';
  readonly chainId = 9999;
  private blockHeight = 1_284_391;

  async connect(): Promise<void> {
    // No-op for mock
  }

  async disconnect(): Promise<void> {
    // No-op for mock
  }

  async getStatus(): Promise<ChainStatus> {
    return {
      chainId: 9999,
      blockHeight: this.blockHeight,
      peers: 3,
      synced: true,
      avgBlockTimeMs: 12000,
    };
  }

  async getBlocks(limit = 10): Promise<Block[]> {
    const blocks: Block[] = [];
    for (let i = 0; i < limit; i++) {
      blocks.push({
        hash: `0x${Array.from({ length: 64 }, () => Math.floor(Math.random() * 16).toString(16)).join('')}`,
        height: this.blockHeight - i,
        timestamp: Date.now() - i * 12000,
        txCount: Math.floor(Math.random() * 20),
        stateRoot: `0x${Array.from({ length: 64 }, () => Math.floor(Math.random() * 16).toString(16)).join('')}`,
      });
    }
    return blocks;
  }

  async getMempool(): Promise<Tx[]> {
    const txs: Tx[] = [];
    for (let i = 0; i < 5; i++) {
      txs.push({
        hash: `0x${Array.from({ length: 64 }, () => Math.floor(Math.random() * 16).toString(16)).join('')}`,
        blockHeight: 0,
        from: `0x${Array.from({ length: 40 }, () => Math.floor(Math.random() * 16).toString(16)).join('')}`,
        to: `0x${Array.from({ length: 40 }, () => Math.floor(Math.random() * 16).toString(16)).join('')}`,
        value: `${(Math.random() * 100).toFixed(4)} ETH`,
        status: 'pending',
        timestamp: Date.now(),
      });
    }
    return txs;
  }

  async sendTx(tx: SignedTx): Promise<TxHash> {
    return `0x${Array.from({ length: 64 }, () => Math.floor(Math.random() * 16).toString(16)).join('')}`;
  }

  async getBalance(address: string): Promise<string> {
    return `${(Math.random() * 1000).toFixed(4)} ETH`;
  }
}
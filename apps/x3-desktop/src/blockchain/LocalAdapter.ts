import { ChainAdapter, ChainStatus, Block, Tx, SignedTx, TxHash } from './ChainAdapter';
import { invoke } from '../ipc/tauri';

/**
 * Local dev chain adapter — connects to a local X3 chain node.
 * Uses real blockchain connections via Tauri IPC to Rust backend.
 * NO MOCK DATA - all data comes from actual blockchain state.
 */
export class LocalAdapter implements ChainAdapter {
  readonly name = 'X3 Local Dev';
  readonly chainId = 1337; // local development chain ID
  private connected = false;

  async connect(): Promise<void> {
    try {
      const result = await invoke<string>('connect_chain', { chain: 'local' });
      this.connected = result === 'ok';
      console.log('Connected to local X3 chain via real blockchain connection');
    } catch (error) {
      console.error('Failed to connect to local chain:', error);
      throw new Error(`Local chain connection failed: ${error}`);
    }
  }

  async disconnect(): Promise<void> {
    try {
      await invoke<string>('disconnect_chain', { chain: 'local' });
      this.connected = false;
      console.log('Disconnected from local X3 chain');
    } catch (error) {
      console.error('Failed to disconnect from local chain:', error);
    }
  }

  async getStatus(): Promise<ChainStatus> {
    if (!this.connected) {
      throw new Error('Not connected to local chain');
    }

    try {
      return await invoke<ChainStatus>('fetch_chain_status', { chain: 'local' });
    } catch (error) {
      console.error('Failed to fetch chain status:', error);
      throw new Error(`Failed to fetch status: ${error}`);
    }
  }

  async getBlocks(limit = 10): Promise<Block[]> {
    if (!this.connected) {
      throw new Error('Not connected to local chain');
    }

    try {
      const blocks = await invoke<Block[]>('fetch_blocks', {
        chain: 'local',
        limit,
      });

      console.log(`Fetched ${blocks.length} real blocks from local chain`);
      return blocks;
    } catch (error) {
      console.error('Failed to fetch blocks:', error);
      throw new Error(`Failed to fetch blocks: ${error}`);
    }
  }

  async getMempool(): Promise<Tx[]> {
    if (!this.connected) {
      throw new Error('Not connected to local chain');
    }

    try {
      const mempool = await invoke<Tx[]>('fetch_mempool', { chain: 'local' });

      console.log(`Fetched ${mempool.length} real transactions from mempool`);
      return mempool;
    } catch (error) {
      console.error('Failed to fetch mempool:', error);
      throw new Error(`Failed to fetch mempool: ${error}`);
    }
  }

  async sendTx(tx: SignedTx): Promise<TxHash> {
    if (!this.connected) {
      throw new Error('Not connected to local chain');
    }

    try {
      const txHash = await invoke<TxHash>('sign_and_send_tx', {
        chain: 'local',
        rawTx: tx.raw,
      });

      console.log(`Transaction sent: ${txHash}`);
      return txHash;
    } catch (error) {
      console.error('Failed to send transaction:', error);
      throw new Error(`Failed to send transaction: ${error}`);
    }
  }

  async getBalance(address: string): Promise<string> {
    if (!this.connected) {
      throw new Error('Not connected to local chain');
    }

    try {
      const balance = await invoke<string>('get_balance', {
        chain: 'local',
        address,
      });

      return balance;
    } catch (error) {
      console.error('Failed to fetch balance:', error);
      throw new Error(`Failed to fetch balance: ${error}`);
    }
  }
}
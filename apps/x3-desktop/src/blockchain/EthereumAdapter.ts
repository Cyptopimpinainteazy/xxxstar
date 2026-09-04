import { ChainAdapter, ChainStatus, Block, Tx, SignedTx, TxHash } from './ChainAdapter';
import { invoke } from '../ipc/tauri';

/**
 * Ethereum-compatible chain adapter.
 * Routes all calls through the Rust backend via Tauri invoke,
 * keeping private keys secure in the Rust keystore.
 */
export class EthereumAdapter implements ChainAdapter {
  readonly name = 'Ethereum';
  readonly chainId = 1;
  private connected = false;

  async connect(): Promise<void> {
    const result = await invoke<string>('connect_chain', { chain: 'ethereum' });
    this.connected = result === 'ok';
  }

  async disconnect(): Promise<void> {
    await invoke<string>('disconnect_chain', { chain: 'ethereum' });
    this.connected = false;
  }

  async getStatus(): Promise<ChainStatus> {
    return invoke<ChainStatus>('fetch_chain_status', { chain: 'ethereum' });
  }

  async getBlocks(limit = 10): Promise<Block[]> {
    return invoke<Block[]>('fetch_blocks', { chain: 'ethereum', limit });
  }

  async getMempool(): Promise<Tx[]> {
    return invoke<Tx[]>('fetch_mempool', { chain: 'ethereum' });
  }

  async sendTx(tx: SignedTx): Promise<TxHash> {
    return invoke<TxHash>('sign_and_send_tx', {
      chain: 'ethereum',
      rawTx: tx.raw,
    });
  }

  async getBalance(address: string): Promise<string> {
    return invoke<string>('get_balance', { chain: 'ethereum', address });
  }
}
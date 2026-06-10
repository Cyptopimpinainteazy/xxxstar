/**
 * Chain Adapter Interface — abstract blockchain access.
 * No hard-coded chains. Adapters talk to the Rust backend via Tauri IPC.
 */

export interface ChainStatus {
  chainId: number;
  blockHeight: number;
  peers: number;
  synced: boolean;
  avgBlockTimeMs: number;
}

export interface Block {
  hash: string;
  height: number;
  timestamp: number;
  txCount: number;
  stateRoot: string;
}

export interface Tx {
  hash: string;
  blockHeight: number;
  from: string;
  to: string;
  value: string;
  status: 'pending' | 'confirmed' | 'failed';
  timestamp: number;
}

export interface SignedTx {
  raw: string;
  chainId: number;
}

export type TxHash = string;

export interface ChainAdapter {
  readonly name: string;
  readonly chainId: number;

  connect(): Promise<void>;
  disconnect(): Promise<void>;
  getStatus(): Promise<ChainStatus>;
  getBlocks(limit?: number): Promise<Block[]>;
  getMempool(): Promise<Tx[]>;
  sendTx(tx: SignedTx): Promise<TxHash>;
  getBalance(address: string): Promise<string>;
}
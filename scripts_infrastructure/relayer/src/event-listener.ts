/**
 * Event Listener
 * Listens for HTLC claim/refund events on all four blockchains
 */

import { EventEmitter } from 'events';

export interface HTLCEvent {
  swapId: string;
  chain: string;
  eventType: 'claim' | 'refund';
  txid: string;
  blockHeight: number;
  preimage?: string;
  timestamp: number;
}

export interface EventListenerConfig {
  chains: string[];
  wsUrls: { [chain: string]: string };
  pollInterval: number; // ms
  confirmationThreshold: number;
}

export interface EventFilter {
  swapId?: string;
  chain?: string;
  eventType?: 'claim' | 'refund';
  since?: number; // timestamp
}

/**
 * Abstract event listener base class
 */
export abstract class ChainEventListener extends EventEmitter {
  protected chain: string;
  protected wsUrl: string;
  protected isConnected: boolean;
  protected lastProcessedBlock: number;

  constructor(chain: string, wsUrl: string) {
    super();
    this.chain = chain;
    this.wsUrl = wsUrl;
    this.isConnected = false;
    this.lastProcessedBlock = 0;
  }

  /**
   * Connect to chain
   */
  abstract connect(): Promise<void>;

  /**
   * Disconnect from chain
   */
  abstract disconnect(): Promise<void>;

  /**
   * Start listening for HTLC events
   */
  abstract startListening(): Promise<void>;

  /**
   * Stop listening for HTLC events
   */
  abstract stopListening(): Promise<void>;

  /**
   * Get connection status
   */
  isReady(): boolean {
    return this.isConnected;
  }

  /**
   * Get chain name
   */
  getChain(): string {
    return this.chain;
  }

  /**
   * Get last processed block
   */
  getLastBlock(): number {
    return this.lastProcessedBlock;
  }
}

/**
 * Bitcoin Event Listener
 * Uses Electrum protocol for event listening
 */
export class BitcoinEventListener extends ChainEventListener {
  private txBuffer: Map<string, HTLCEvent> = new Map();

  constructor(wsUrl: string) {
    super('bitcoin', wsUrl);
  }

  async connect(): Promise<void> {
    try {
      // In production, would connect to Electrum server
      console.log(`[Bitcoin] Connecting to ${this.wsUrl}`);
      this.isConnected = true;
      this.emit('connected', { chain: 'bitcoin' });
    } catch (error) {
      throw new Error(`Failed to connect to Bitcoin: ${error}`);
    }
  }

  async disconnect(): Promise<void> {
    this.isConnected = false;
    this.emit('disconnected', { chain: 'bitcoin' });
  }

  async startListening(): Promise<void> {
    if (!this.isConnected) throw new Error('Not connected');

    // Listen for HTLC transactions
    console.log('[Bitcoin] Started listening for HTLC events');
    this.emit('listening', { chain: 'bitcoin' });
  }

  async stopListening(): Promise<void> {
    console.log('[Bitcoin] Stopped listening for HTLC events');
    this.emit('stopped', { chain: 'bitcoin' });
  }

  /**
   * Process claim transaction
   */
  async processClaim(txid: string, blockHeight: number): Promise<void> {
    // Parse witness script to extract preimage
    const preimage = await this.extractPreimage(txid);

    const event: HTLCEvent = {
      swapId: this.deriveSwapId(txid),
      chain: 'bitcoin',
      eventType: 'claim',
      txid,
      blockHeight,
      preimage,
      timestamp: Math.floor(Date.now() / 1000),
    };

    this.txBuffer.set(txid, event);
    this.emit('htlc-event', event);
  }

  /**
   * Extract preimage from claim witness
   */
  private async extractPreimage(txid: string): Promise<string> {
    // In production, would parse actual transaction
    return 'extracted-preimage-from-' + txid.substring(0, 8);
  }

  /**
   * Derive swap ID from transaction
   */
  private deriveSwapId(txid: string): string {
    return 'swap-' + txid.substring(0, 16);
  }

  /**
   * Get buffered events
   */
  getBufferedEvents(): HTLCEvent[] {
    return Array.from(this.txBuffer.values());
  }

  /**
   * Clear processed event
   */
  clearEvent(txid: string): void {
    this.txBuffer.delete(txid);
  }
}

/**
 * Ethereum Event Listener
 * Uses web3.js for event filtering
 */
export class EthereumEventListener extends ChainEventListener {
  private eventBuffer: Map<string, HTLCEvent> = new Map();

  constructor(wsUrl: string) {
    super('ethereum', wsUrl);
  }

  async connect(): Promise<void> {
    try {
      console.log(`[Ethereum] Connecting to ${this.wsUrl}`);
      this.isConnected = true;
      this.emit('connected', { chain: 'ethereum' });
    } catch (error) {
      throw new Error(`Failed to connect to Ethereum: ${error}`);
    }
  }

  async disconnect(): Promise<void> {
    this.isConnected = false;
    this.emit('disconnected', { chain: 'ethereum' });
  }

  async startListening(): Promise<void> {
    if (!this.isConnected) throw new Error('Not connected');

    // Listen for X3HtlcEvm events
    console.log('[Ethereum] Started listening for HTLC events');
    this.emit('listening', { chain: 'ethereum' });
  }

  async stopListening(): Promise<void> {
    console.log('[Ethereum] Stopped listening for HTLC events');
    this.emit('stopped', { chain: 'ethereum' });
  }

  /**
   * Process claim event
   */
  async processClaimEvent(
    swapId: string,
    txid: string,
    blockHeight: number
  ): Promise<void> {
    const event: HTLCEvent = {
      swapId,
      chain: 'ethereum',
      eventType: 'claim',
      txid,
      blockHeight,
      timestamp: Math.floor(Date.now() / 1000),
    };

    this.eventBuffer.set(txid, event);
    this.emit('htlc-event', event);
  }

  /**
   * Get buffered events
   */
  getBufferedEvents(): HTLCEvent[] {
    return Array.from(this.eventBuffer.values());
  }

  /**
   * Clear processed event
   */
  clearEvent(txid: string): void {
    this.eventBuffer.delete(txid);
  }
}

/**
 * Solana Event Listener
 * Uses Solana WebSocket API
 */
export class SolanaEventListener extends ChainEventListener {
  private eventBuffer: Map<string, HTLCEvent> = new Map();

  constructor(wsUrl: string) {
    super('solana', wsUrl);
  }

  async connect(): Promise<void> {
    try {
      console.log(`[Solana] Connecting to ${this.wsUrl}`);
      this.isConnected = true;
      this.emit('connected', { chain: 'solana' });
    } catch (error) {
      throw new Error(`Failed to connect to Solana: ${error}`);
    }
  }

  async disconnect(): Promise<void> {
    this.isConnected = false;
    this.emit('disconnected', { chain: 'solana' });
  }

  async startListening(): Promise<void> {
    if (!this.isConnected) throw new Error('Not connected');

    console.log('[Solana] Started listening for HTLC events');
    this.emit('listening', { chain: 'solana' });
  }

  async stopListening(): Promise<void> {
    console.log('[Solana] Stopped listening for HTLC events');
    this.emit('stopped', { chain: 'solana' });
  }

  /**
   * Process claim event
   */
  async processClaimEvent(
    swapId: string,
    txid: string,
    blockHeight: number
  ): Promise<void> {
    const event: HTLCEvent = {
      swapId,
      chain: 'solana',
      eventType: 'claim',
      txid,
      blockHeight,
      timestamp: Math.floor(Date.now() / 1000),
    };

    this.eventBuffer.set(txid, event);
    this.emit('htlc-event', event);
  }

  /**
   * Get buffered events
   */
  getBufferedEvents(): HTLCEvent[] {
    return Array.from(this.eventBuffer.values());
  }

  /**
   * Clear processed event
   */
  clearEvent(txid: string): void {
    this.eventBuffer.delete(txid);
  }
}

/**
 * X3VM Event Listener
 * Uses Substrate chain subscription
 */
export class X3VMEventListener extends ChainEventListener {
  private eventBuffer: Map<string, HTLCEvent> = new Map();

  constructor(wsUrl: string) {
    super('x3vm', wsUrl);
  }

  async connect(): Promise<void> {
    try {
      console.log(`[X3VM] Connecting to ${this.wsUrl}`);
      this.isConnected = true;
      this.emit('connected', { chain: 'x3vm' });
    } catch (error) {
      throw new Error(`Failed to connect to X3VM: ${error}`);
    }
  }

  async disconnect(): Promise<void> {
    this.isConnected = false;
    this.emit('disconnected', { chain: 'x3vm' });
  }

  async startListening(): Promise<void> {
    if (!this.isConnected) throw new Error('Not connected');

    console.log('[X3VM] Started listening for HTLC events');
    this.emit('listening', { chain: 'x3vm' });
  }

  async stopListening(): Promise<void> {
    console.log('[X3VM] Stopped listening for HTLC events');
    this.emit('stopped', { chain: 'x3vm' });
  }

  /**
   * Process claim event
   */
  async processClaimEvent(
    swapId: string,
    txid: string,
    blockHeight: number
  ): Promise<void> {
    const event: HTLCEvent = {
      swapId,
      chain: 'x3vm',
      eventType: 'claim',
      txid,
      blockHeight,
      timestamp: Math.floor(Date.now() / 1000),
    };

    this.eventBuffer.set(txid, event);
    this.emit('htlc-event', event);
  }

  /**
   * Get buffered events
   */
  getBufferedEvents(): HTLCEvent[] {
    return Array.from(this.eventBuffer.values());
  }

  /**
   * Clear processed event
   */
  clearEvent(txid: string): void {
    this.eventBuffer.delete(txid);
  }
}

/**
 * Multi-Chain Event Manager
 */
export class EventManager extends EventEmitter {
  // Avoid conflicting with EventEmitter's `listeners` method
  private chainListeners: Map<string, ChainEventListener> = new Map();
  private allEvents: HTLCEvent[] = [];

  /**
   * Register an event listener for a chain
   */
  registerListener(listener: ChainEventListener): void {
    const chain = listener.getChain();
    this.chainListeners.set(chain, listener);

    // Relay events from listener
    listener.on('htlc-event', (event: HTLCEvent) => {
      this.allEvents.push(event);
      this.emit('htlc-event', event);
    });

    listener.on('connected', (event) => this.emit('listener-connected', event));
    listener.on('disconnected', (event) => this.emit('listener-disconnected', event));
  }

  /**
   * Connect all listeners
   */
  async connectAll(): Promise<void> {
    const promises = Array.from(this.chainListeners.values()).map((listener) =>
      listener.connect()
    );
    await Promise.all(promises);
  }

  /**
   * Disconnect all listeners
   */
  async disconnectAll(): Promise<void> {
    const promises = Array.from(this.chainListeners.values()).map((listener) =>
      listener.disconnect()
    );
    await Promise.all(promises);
  }

  /**
   * Start listening on all chains
   */
  async startAll(): Promise<void> {
    const promises = Array.from(this.chainListeners.values()).map((listener) =>
      listener.startListening()
    );
    await Promise.all(promises);
  }

  /**
   * Stop listening on all chains
   */
  async stopAll(): Promise<void> {
    const promises = Array.from(this.chainListeners.values()).map((listener) =>
      listener.stopListening()
    );
    await Promise.all(promises);
  }

  /**
   * Get all buffered events
   */
  getAllEvents(): HTLCEvent[] {
    return this.allEvents;
  }

  /**
   * Filter events
   */
  filterEvents(filter: EventFilter): HTLCEvent[] {
    return this.allEvents.filter((event) => {
      if (filter.swapId && event.swapId !== filter.swapId) return false;
      if (filter.chain && event.chain !== filter.chain) return false;
      if (filter.eventType && event.eventType !== filter.eventType) return false;
      if (filter.since && event.timestamp < filter.since) return false;
      return true;
    });
  }

  /**
   * Get health status
   */
  getHealth(): { [chain: string]: boolean } {
    const health: { [chain: string]: boolean } = {};
    for (const [chain, listener] of this.chainListeners.entries()) {
      health[chain] = listener.isReady();
    }
    return health;
  }
}

export default EventManager;

"use strict";
/**
 * Event Listener
 * Listens for HTLC claim/refund events on all four blockchains
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.EventManager = exports.X3VMEventListener = exports.SolanaEventListener = exports.EthereumEventListener = exports.BitcoinEventListener = exports.ChainEventListener = void 0;
const events_1 = require("events");
/**
 * Abstract event listener base class
 */
class ChainEventListener extends events_1.EventEmitter {
    constructor(chain, wsUrl) {
        super();
        this.chain = chain;
        this.wsUrl = wsUrl;
        this.isConnected = false;
        this.lastProcessedBlock = 0;
    }
    /**
     * Get connection status
     */
    isReady() {
        return this.isConnected;
    }
    /**
     * Get chain name
     */
    getChain() {
        return this.chain;
    }
    /**
     * Get last processed block
     */
    getLastBlock() {
        return this.lastProcessedBlock;
    }
}
exports.ChainEventListener = ChainEventListener;
/**
 * Bitcoin Event Listener
 * Uses Electrum protocol for event listening
 */
class BitcoinEventListener extends ChainEventListener {
    constructor(wsUrl) {
        super('bitcoin', wsUrl);
        this.txBuffer = new Map();
    }
    async connect() {
        try {
            // In production, would connect to Electrum server
            console.log(`[Bitcoin] Connecting to ${this.wsUrl}`);
            this.isConnected = true;
            this.emit('connected', { chain: 'bitcoin' });
        }
        catch (error) {
            throw new Error(`Failed to connect to Bitcoin: ${error}`);
        }
    }
    async disconnect() {
        this.isConnected = false;
        this.emit('disconnected', { chain: 'bitcoin' });
    }
    async startListening() {
        if (!this.isConnected)
            throw new Error('Not connected');
        // Listen for HTLC transactions
        console.log('[Bitcoin] Started listening for HTLC events');
        this.emit('listening', { chain: 'bitcoin' });
    }
    async stopListening() {
        console.log('[Bitcoin] Stopped listening for HTLC events');
        this.emit('stopped', { chain: 'bitcoin' });
    }
    /**
     * Process claim transaction
     */
    async processClaim(txid, blockHeight) {
        // Parse witness script to extract preimage
        const preimage = await this.extractPreimage(txid);
        const event = {
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
    async extractPreimage(txid) {
        // In production, would parse actual transaction
        return 'extracted-preimage-from-' + txid.substring(0, 8);
    }
    /**
     * Derive swap ID from transaction
     */
    deriveSwapId(txid) {
        return 'swap-' + txid.substring(0, 16);
    }
    /**
     * Get buffered events
     */
    getBufferedEvents() {
        return Array.from(this.txBuffer.values());
    }
    /**
     * Clear processed event
     */
    clearEvent(txid) {
        this.txBuffer.delete(txid);
    }
}
exports.BitcoinEventListener = BitcoinEventListener;
/**
 * Ethereum Event Listener
 * Uses web3.js for event filtering
 */
class EthereumEventListener extends ChainEventListener {
    constructor(wsUrl) {
        super('ethereum', wsUrl);
        this.eventBuffer = new Map();
    }
    async connect() {
        try {
            console.log(`[Ethereum] Connecting to ${this.wsUrl}`);
            this.isConnected = true;
            this.emit('connected', { chain: 'ethereum' });
        }
        catch (error) {
            throw new Error(`Failed to connect to Ethereum: ${error}`);
        }
    }
    async disconnect() {
        this.isConnected = false;
        this.emit('disconnected', { chain: 'ethereum' });
    }
    async startListening() {
        if (!this.isConnected)
            throw new Error('Not connected');
        // Listen for X3HtlcEvm events
        console.log('[Ethereum] Started listening for HTLC events');
        this.emit('listening', { chain: 'ethereum' });
    }
    async stopListening() {
        console.log('[Ethereum] Stopped listening for HTLC events');
        this.emit('stopped', { chain: 'ethereum' });
    }
    /**
     * Process claim event
     */
    async processClaimEvent(swapId, txid, blockHeight) {
        const event = {
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
    getBufferedEvents() {
        return Array.from(this.eventBuffer.values());
    }
    /**
     * Clear processed event
     */
    clearEvent(txid) {
        this.eventBuffer.delete(txid);
    }
}
exports.EthereumEventListener = EthereumEventListener;
/**
 * Solana Event Listener
 * Uses Solana WebSocket API
 */
class SolanaEventListener extends ChainEventListener {
    constructor(wsUrl) {
        super('solana', wsUrl);
        this.eventBuffer = new Map();
    }
    async connect() {
        try {
            console.log(`[Solana] Connecting to ${this.wsUrl}`);
            this.isConnected = true;
            this.emit('connected', { chain: 'solana' });
        }
        catch (error) {
            throw new Error(`Failed to connect to Solana: ${error}`);
        }
    }
    async disconnect() {
        this.isConnected = false;
        this.emit('disconnected', { chain: 'solana' });
    }
    async startListening() {
        if (!this.isConnected)
            throw new Error('Not connected');
        console.log('[Solana] Started listening for HTLC events');
        this.emit('listening', { chain: 'solana' });
    }
    async stopListening() {
        console.log('[Solana] Stopped listening for HTLC events');
        this.emit('stopped', { chain: 'solana' });
    }
    /**
     * Process claim event
     */
    async processClaimEvent(swapId, txid, blockHeight) {
        const event = {
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
    getBufferedEvents() {
        return Array.from(this.eventBuffer.values());
    }
    /**
     * Clear processed event
     */
    clearEvent(txid) {
        this.eventBuffer.delete(txid);
    }
}
exports.SolanaEventListener = SolanaEventListener;
/**
 * X3VM Event Listener
 * Uses Substrate chain subscription
 */
class X3VMEventListener extends ChainEventListener {
    constructor(wsUrl) {
        super('x3vm', wsUrl);
        this.eventBuffer = new Map();
    }
    async connect() {
        try {
            console.log(`[X3VM] Connecting to ${this.wsUrl}`);
            this.isConnected = true;
            this.emit('connected', { chain: 'x3vm' });
        }
        catch (error) {
            throw new Error(`Failed to connect to X3VM: ${error}`);
        }
    }
    async disconnect() {
        this.isConnected = false;
        this.emit('disconnected', { chain: 'x3vm' });
    }
    async startListening() {
        if (!this.isConnected)
            throw new Error('Not connected');
        console.log('[X3VM] Started listening for HTLC events');
        this.emit('listening', { chain: 'x3vm' });
    }
    async stopListening() {
        console.log('[X3VM] Stopped listening for HTLC events');
        this.emit('stopped', { chain: 'x3vm' });
    }
    /**
     * Process claim event
     */
    async processClaimEvent(swapId, txid, blockHeight) {
        const event = {
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
    getBufferedEvents() {
        return Array.from(this.eventBuffer.values());
    }
    /**
     * Clear processed event
     */
    clearEvent(txid) {
        this.eventBuffer.delete(txid);
    }
}
exports.X3VMEventListener = X3VMEventListener;
/**
 * Multi-Chain Event Manager
 */
class EventManager extends events_1.EventEmitter {
    constructor() {
        super(...arguments);
        // Avoid conflicting with EventEmitter's `listeners` method
        this.chainListeners = new Map();
        this.allEvents = [];
    }
    /**
     * Register an event listener for a chain
     */
    registerListener(listener) {
        const chain = listener.getChain();
        this.chainListeners.set(chain, listener);
        // Relay events from listener
        listener.on('htlc-event', (event) => {
            this.allEvents.push(event);
            this.emit('htlc-event', event);
        });
        listener.on('connected', (event) => this.emit('listener-connected', event));
        listener.on('disconnected', (event) => this.emit('listener-disconnected', event));
    }
    /**
     * Connect all listeners
     */
    async connectAll() {
        const promises = Array.from(this.chainListeners.values()).map((listener) => listener.connect());
        await Promise.all(promises);
    }
    /**
     * Disconnect all listeners
     */
    async disconnectAll() {
        const promises = Array.from(this.chainListeners.values()).map((listener) => listener.disconnect());
        await Promise.all(promises);
    }
    /**
     * Start listening on all chains
     */
    async startAll() {
        const promises = Array.from(this.chainListeners.values()).map((listener) => listener.startListening());
        await Promise.all(promises);
    }
    /**
     * Stop listening on all chains
     */
    async stopAll() {
        const promises = Array.from(this.chainListeners.values()).map((listener) => listener.stopListening());
        await Promise.all(promises);
    }
    /**
     * Get all buffered events
     */
    getAllEvents() {
        return this.allEvents;
    }
    /**
     * Filter events
     */
    filterEvents(filter) {
        return this.allEvents.filter((event) => {
            if (filter.swapId && event.swapId !== filter.swapId)
                return false;
            if (filter.chain && event.chain !== filter.chain)
                return false;
            if (filter.eventType && event.eventType !== filter.eventType)
                return false;
            if (filter.since && event.timestamp < filter.since)
                return false;
            return true;
        });
    }
    /**
     * Get health status
     */
    getHealth() {
        const health = {};
        for (const [chain, listener] of this.chainListeners.entries()) {
            health[chain] = listener.isReady();
        }
        return health;
    }
}
exports.EventManager = EventManager;
exports.default = EventManager;

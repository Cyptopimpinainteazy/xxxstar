"use strict";
/**
 * Atomic Swap Orchestrator
 *
 * Coordinates the full lifecycle of a cross-chain atomic swap:
 *
 * 1. Initiator generates secret + hashLock
 * 2. Initiator creates HTLC on source chain (longer timelock)
 * 3. Counterparty verifies source HTLC, creates HTLC on dest chain (shorter timelock)
 * 4. Initiator claims dest HTLC (reveals secret)
 * 5. Counterparty extracts secret from dest chain, claims source HTLC
 *
 * Supports: EVM ↔ EVM, EVM ↔ Solana, EVM ↔ Bitcoin, EVM ↔ Substrate,
 *           Solana ↔ Bitcoin, Substrate ↔ any
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.SwapOrchestrator = void 0;
const eventemitter3_1 = require("eventemitter3");
const htlc_1 = require("../htlc");
class SwapOrchestrator extends eventemitter3_1.EventEmitter {
    config;
    adapters = new Map();
    swaps = new Map();
    monitorIntervals = new Map();
    /** Monitor polling interval in ms */
    pollIntervalMs;
    constructor(config, pollIntervalMs = 15000) {
        super();
        this.config = config;
        this.pollIntervalMs = pollIntervalMs;
    }
    // ─── Public API ─────────────────────────────────────────────
    /**
     * Initialize a new atomic swap as the initiator.
     *
     * Steps:
     * 1. Generate secret + hashLock
     * 2. Create HTLC on source chain
     * 3. Return swap object for counterparty to verify
     */
    async initiateSwap(params, signerKey) {
        // Generate cryptographic secret
        const { secret, hashLock } = (0, htlc_1.generateSecret)();
        // Calculate time locks
        const timeLockSeconds = params.timeLockSeconds || this.config.defaultTimeLockInitiator;
        const { initiatorTimeLock, counterpartyTimeLock } = (0, htlc_1.calculateTimeLocks)(timeLockSeconds);
        // Get adapter for source chain
        const sourceAdapter = this.getOrCreateAdapter(params.sourceChain);
        // Create HTLC on source chain
        const sourceHtlc = await sourceAdapter.createHTLC({
            chainId: params.sourceChain,
            recipient: params.counterparty,
            tokenAddress: params.sourceToken,
            amount: params.amount,
            hashLock,
            timeLock: initiatorTimeLock,
        }, signerKey);
        // Create swap record
        const swap = {
            id: `swap_${Date.now()}_${hashLock.slice(2, 10)}`,
            initiator: signerKey,
            counterparty: params.counterparty,
            sourceChain: params.sourceChain,
            destChain: params.destChain,
            sourceToken: params.sourceToken,
            destToken: params.destToken,
            sourceAmount: params.amount,
            destAmount: params.amount, // 1:1 for now — price negotiation happens in orderbook
            sourceHtlc,
            hashLock,
            secret,
            status: "htlc-created-source",
            createdAt: Date.now(),
            expiresAt: initiatorTimeLock * 1000,
        };
        this.swaps.set(swap.id, swap);
        this.emit("swap-initiated", swap);
        this.emit("swap-source-funded", swap);
        // Start monitoring
        this.startMonitoring(swap.id);
        return swap;
    }
    /**
     * As counterparty, respond to a swap by creating HTLC on destination chain.
     *
     * The counterparty must verify the source HTLC first, then lock their tokens
     * on the destination chain with the same hashLock but a shorter timelock.
     */
    async respondToSwap(swapId, destAmount, signerKey) {
        const swap = this.swaps.get(swapId);
        if (!swap)
            throw new Error(`Swap ${swapId} not found`);
        if (swap.status !== "htlc-created-source") {
            throw new Error(`Swap ${swapId} is in status ${swap.status}, expected htlc-created-source`);
        }
        // Verify source HTLC is funded
        const sourceAdapter = this.getOrCreateAdapter(swap.sourceChain);
        const isFunded = await sourceAdapter.isHTLCFunded(swap.sourceHtlc.id);
        if (!isFunded) {
            throw new Error(`Source HTLC ${swap.sourceHtlc.id} is not funded`);
        }
        // Get adapter for destination chain
        const destAdapter = this.getOrCreateAdapter(swap.destChain);
        // Create HTLC on destination chain with shorter timelock
        const counterpartyTimeLock = Math.floor(Date.now() / 1000) + this.config.defaultTimeLockCounterparty;
        const destHtlc = await destAdapter.createHTLC({
            chainId: swap.destChain,
            recipient: swap.initiator,
            tokenAddress: swap.destToken,
            amount: destAmount,
            hashLock: swap.hashLock,
            timeLock: counterpartyTimeLock,
        }, signerKey);
        // Update swap
        swap.destHtlc = destHtlc;
        swap.destAmount = destAmount;
        this.updateSwapStatus(swap, "htlc-created-destination");
        this.emit("swap-dest-funded", swap);
        return swap;
    }
    /**
     * As initiator, claim the destination HTLC (reveals secret).
     * After this, the counterparty can extract the secret and claim the source HTLC.
     */
    async claimSwap(swapId, signerKey) {
        const swap = this.swaps.get(swapId);
        if (!swap)
            throw new Error(`Swap ${swapId} not found`);
        if (swap.status !== "htlc-created-destination") {
            throw new Error(`Swap ${swapId} is in status ${swap.status}, expected htlc-created-destination`);
        }
        if (!swap.secret)
            throw new Error("No secret available for this swap");
        if (!swap.destHtlc)
            throw new Error("No destination HTLC for this swap");
        // Claim destination HTLC
        const destAdapter = this.getOrCreateAdapter(swap.destChain);
        await destAdapter.claimHTLC({ htlcId: swap.destHtlc.id, secret: swap.secret }, signerKey);
        this.updateSwapStatus(swap, "claimed");
        this.emit("swap-claimed", swap);
        // Stop monitoring
        this.stopMonitoring(swapId);
        return swap;
    }
    /**
     * As counterparty, extract the revealed secret from the destination chain
     * and claim the source HTLC.
     */
    async counterpartyClaim(swapId, signerKey) {
        const swap = this.swaps.get(swapId);
        if (!swap)
            throw new Error(`Swap ${swapId} not found`);
        if (!swap.destHtlc)
            throw new Error("No destination HTLC");
        if (!swap.sourceHtlc)
            throw new Error("No source HTLC");
        // Extract secret from destination chain
        const destAdapter = this.getOrCreateAdapter(swap.destChain);
        const { claimed, secret } = await destAdapter.isHTLCClaimed(swap.destHtlc.id);
        if (!claimed || !secret) {
            throw new Error("Destination HTLC has not been claimed yet — secret not revealed");
        }
        // Claim source HTLC with the extracted secret
        const sourceAdapter = this.getOrCreateAdapter(swap.sourceChain);
        await sourceAdapter.claimHTLC({ htlcId: swap.sourceHtlc.id, secret }, signerKey);
        swap.secret = secret;
        this.updateSwapStatus(swap, "claimed");
        return swap;
    }
    /**
     * Refund an expired swap (source HTLC).
     */
    async refundSwap(swapId, signerKey) {
        const swap = this.swaps.get(swapId);
        if (!swap)
            throw new Error(`Swap ${swapId} not found`);
        if (!swap.sourceHtlc)
            throw new Error("No source HTLC to refund");
        const sourceAdapter = this.getOrCreateAdapter(swap.sourceChain);
        const isExpired = await sourceAdapter.isHTLCExpired(swap.sourceHtlc.id);
        if (!isExpired) {
            throw new Error("Source HTLC has not expired yet — cannot refund");
        }
        await sourceAdapter.refundHTLC({ htlcId: swap.sourceHtlc.id }, signerKey);
        this.updateSwapStatus(swap, "refunded");
        this.emit("swap-refunded", swap);
        this.stopMonitoring(swapId);
        return swap;
    }
    /**
     * Get swap by ID.
     */
    getSwap(swapId) {
        return this.swaps.get(swapId);
    }
    /**
     * List all swaps.
     */
    listSwaps(filter) {
        const all = Array.from(this.swaps.values());
        if (filter?.status) {
            return all.filter((s) => s.status === filter.status);
        }
        return all;
    }
    /**
     * Stop all monitoring and clean up.
     */
    destroy() {
        for (const [id] of this.monitorIntervals) {
            this.stopMonitoring(id);
        }
        this.removeAllListeners();
    }
    // ─── Adapter Management ───────────────────────────────────────
    getOrCreateAdapter(chainId) {
        let adapter = this.adapters.get(chainId);
        if (!adapter) {
            const endpoint = this.config.chainEndpoints[chainId];
            const htlcContract = this.config.htlcContracts[chainId];
            if (!endpoint) {
                throw new Error(`No RPC endpoint configured for chain ${chainId}`);
            }
            adapter = (0, htlc_1.createHTLCAdapter)({
                chainId,
                rpcEndpoint: endpoint,
                htlcContractAddress: htlcContract,
            });
            this.adapters.set(chainId, adapter);
        }
        return adapter;
    }
    // ─── Monitoring ───────────────────────────────────────────────
    startMonitoring(swapId) {
        if (this.monitorIntervals.has(swapId))
            return;
        const interval = setInterval(async () => {
            try {
                await this.checkSwapProgress(swapId);
            }
            catch (err) {
                const swap = this.swaps.get(swapId);
                if (swap) {
                    this.emit("swap-failed", swap, err.message || "Monitor error");
                }
            }
        }, this.pollIntervalMs);
        this.monitorIntervals.set(swapId, interval);
    }
    stopMonitoring(swapId) {
        const interval = this.monitorIntervals.get(swapId);
        if (interval) {
            clearInterval(interval);
            this.monitorIntervals.delete(swapId);
        }
    }
    async checkSwapProgress(swapId) {
        const swap = this.swaps.get(swapId);
        if (!swap) {
            this.stopMonitoring(swapId);
            return;
        }
        // Check for expiry
        if (Date.now() > swap.expiresAt) {
            this.updateSwapStatus(swap, "expired");
            this.emit("swap-expired", swap);
            this.stopMonitoring(swapId);
            return;
        }
        // Check for claim on destination (if we're waiting for counterparty)
        if (swap.status === "htlc-created-destination" && swap.destHtlc) {
            const destAdapter = this.getOrCreateAdapter(swap.destChain);
            const { claimed, secret } = await destAdapter.isHTLCClaimed(swap.destHtlc.id);
            if (claimed && secret) {
                swap.secret = secret;
                this.updateSwapStatus(swap, "claimed");
                this.emit("swap-claimed", swap);
                this.stopMonitoring(swapId);
            }
        }
        // Check for claim on source (if we need to extract secret)
        if (swap.status === "htlc-created-source" && swap.sourceHtlc) {
            const sourceAdapter = this.getOrCreateAdapter(swap.sourceChain);
            const { claimed, secret } = await sourceAdapter.isHTLCClaimed(swap.sourceHtlc.id);
            if (claimed && secret) {
                swap.secret = secret;
                this.updateSwapStatus(swap, "claimed");
                this.emit("swap-claimed", swap);
                this.stopMonitoring(swapId);
            }
        }
    }
    updateSwapStatus(swap, newStatus) {
        const oldStatus = swap.status;
        swap.status = newStatus;
        this.emit("swap-status-change", swap, oldStatus, newStatus);
    }
}
exports.SwapOrchestrator = SwapOrchestrator;
//# sourceMappingURL=orchestrator.js.map
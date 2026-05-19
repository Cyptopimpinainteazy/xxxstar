"use strict";
/**
 * Settlement Verifier
 * Verifies proofs and tracks settlement completion on target chains
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.SettlementVerifier = void 0;
const events_1 = require("events");
/**
 * Settlement Verifier
 */
class SettlementVerifier extends events_1.EventEmitter {
    constructor(config, chainClients) {
        super();
        this.settlements = new Map();
        this.verifierConfig = config;
        this.chainClients = chainClients;
    }
    /**
     * Register settlement for verification
     */
    registerSettlement(swapId, sourceChain, targetChain, proof, settlementTxid) {
        const record = {
            swapId,
            sourceChain,
            targetChain,
            proof,
            settlementTxid,
            status: 'pending',
            confirmations: 0,
        };
        if (!this.settlements.has(swapId)) {
            this.settlements.set(swapId, []);
        }
        this.settlements.get(swapId).push(record);
        this.emit('settlement-registered', {
            swapId,
            targetChain,
            settlementTxid,
        });
    }
    /**
     * Start verification monitoring
     */
    start() {
        this.verificationInterval = setInterval(() => this.verifyPendingSettlements(), this.verifierConfig.verificationInterval);
        this.emit('verifier-started');
    }
    /**
     * Stop verification monitoring
     */
    stop() {
        if (this.verificationInterval) {
            clearInterval(this.verificationInterval);
        }
        this.emit('verifier-stopped');
    }
    /**
     * Verify pending settlements
     */
    async verifyPendingSettlements() {
        for (const [swapId, records] of this.settlements.entries()) {
            for (const record of records) {
                if (record.status !== 'pending')
                    continue;
                try {
                    await this.verifySettlement(record);
                }
                catch (error) {
                    console.error(`Error verifying settlement ${swapId}/${record.targetChain}:`, error);
                }
            }
        }
    }
    /**
     * Verify single settlement
     */
    async verifySettlement(record) {
        try {
            // Get transaction on target chain
            const client = this.chainClients[record.targetChain];
            if (!client) {
                throw new Error(`No client for chain: ${record.targetChain}`);
            }
            const txInfo = await client.getTransactionInfo(record.settlementTxid);
            if (txInfo.status === 'failed') {
                record.status = 'failed';
                record.completedAt = Math.floor(Date.now() / 1000);
                this.emit('settlement-failed', {
                    swapId: record.swapId,
                    chain: record.targetChain,
                    txid: record.settlementTxid,
                });
                return;
            }
            // Update confirmations
            record.confirmations = txInfo.confirmations;
            if (record.confirmations >= this.verifierConfig.requiredConfirmations) {
                // Verify proof validity
                const verification = await this.verifyProof(record);
                if (verification.valid) {
                    record.status = 'confirmed';
                    record.verifiedAt = Math.floor(Date.now() / 1000);
                    record.completedAt = record.verifiedAt;
                    this.emit('settlement-confirmed', {
                        swapId: record.swapId,
                        chain: record.targetChain,
                        confirmations: record.confirmations,
                    });
                }
                else {
                    record.status = 'failed';
                    record.completedAt = Math.floor(Date.now() / 1000);
                    this.emit('settlement-invalid', {
                        swapId: record.swapId,
                        chain: record.targetChain,
                        errors: verification.errors,
                    });
                }
            }
        }
        catch (error) {
            console.error(`Settlement verification error:`, error);
        }
    }
    /**
     * Verify proof validity
     */
    async verifyProof(record) {
        const errors = [];
        const warnings = [];
        // Verify transaction hash
        if (!record.proof.txid) {
            errors.push('Missing transaction ID');
        }
        // Verify block height
        if (record.proof.blockHeight < 0) {
            errors.push('Invalid block height');
        }
        // Verify confirmations
        if (record.proof.confirmations < 1) {
            errors.push('Transaction not confirmed');
        }
        // Verify block header
        if (!record.proof.blockHeader || !record.proof.blockHeader.merkleRoot) {
            errors.push('Invalid block header');
        }
        // Verify merkle proof
        if (!record.proof.merkleProof || !record.proof.merkleProof.merkleProof) {
            errors.push('Invalid merkle proof');
        }
        // Warning: old transaction
        const now = Math.floor(Date.now() / 1000);
        const age = now - record.proof.timestamp;
        if (age > 24 * 60 * 60) {
            warnings.push('Proof is older than 24 hours');
        }
        return {
            swapId: record.swapId,
            valid: errors.length === 0,
            errors,
            warnings,
        };
    }
    /**
     * Get settlement records
     */
    getSettlements(swapId) {
        if (swapId) {
            return this.settlements.get(swapId) || [];
        }
        const all = [];
        for (const records of this.settlements.values()) {
            all.push(...records);
        }
        return all;
    }
    /**
     * Get settlement status
     */
    getSettlementStatus(swapId, targetChain) {
        const records = this.settlements.get(swapId);
        return records?.find((r) => r.targetChain === targetChain);
    }
    /**
     * Get all confirmed settlements
     */
    getConfirmedSettlements() {
        return this.getSettlements().filter((r) => r.status === 'confirmed');
    }
    /**
     * Get verification statistics
     */
    getStatistics() {
        const allRecords = this.getSettlements();
        const confirmed = allRecords.filter((r) => r.status === 'confirmed').length;
        const pending = allRecords.filter((r) => r.status === 'pending').length;
        const failed = allRecords.filter((r) => r.status === 'failed').length;
        const avgConfirmations = allRecords.length > 0
            ? (allRecords.reduce((sum, r) => sum + r.confirmations, 0) / allRecords.length).toFixed(2)
            : '0';
        return {
            totalSettlements: allRecords.length,
            confirmed,
            pending,
            failed,
            averageConfirmations: avgConfirmations,
            successRate: allRecords.length > 0
                ? ((confirmed / allRecords.length) * 100).toFixed(2) + '%'
                : 'N/A',
        };
    }
    /**
     * Wait for settlement confirmation
     */
    async waitForSettlement(swapId, targetChain, maxWaitTime = 3600000 // 1 hour
    ) {
        const startTime = Date.now();
        while (Date.now() - startTime < maxWaitTime) {
            const settlement = this.getSettlementStatus(swapId, targetChain);
            if (settlement?.status === 'confirmed') {
                return settlement;
            }
            if (settlement?.status === 'failed') {
                throw new Error(`Settlement failed for ${swapId} on ${targetChain}`);
            }
            // Wait 5 seconds before checking again
            await new Promise((resolve) => setTimeout(resolve, 5000));
        }
        throw new Error(`Settlement verification timeout for ${swapId} on ${targetChain}`);
    }
}
exports.SettlementVerifier = SettlementVerifier;
exports.default = SettlementVerifier;

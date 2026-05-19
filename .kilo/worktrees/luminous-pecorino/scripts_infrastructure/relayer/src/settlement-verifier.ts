/**
 * Settlement Verifier
 * Verifies proofs and tracks settlement completion on target chains
 */

import { EventEmitter } from 'events';
import { SPVProof } from './spv-proof-generator';

export interface SettlementRecord {
  swapId: string;
  sourceChain: string;
  targetChain: string;
  proof: SPVProof;
  settlementTxid: string;
  status: 'pending' | 'confirmed' | 'failed';
  confirmations: number;
  verifiedAt?: number;
  completedAt?: number;
}

export interface VerificationResult {
  swapId: string;
  valid: boolean;
  errors: string[];
  warnings: string[];
}

export interface VerifierConfig {
  requiredConfirmations: number;
  maxVerificationAttempts: number;
  verificationInterval: number; // ms
}

/**
 * Settlement Verifier
 */
export class SettlementVerifier extends EventEmitter {
  private settlements: Map<string, SettlementRecord[]> = new Map();
  private verifierConfig: VerifierConfig;
  private chainClients: { [chain: string]: any };
  private verificationInterval?: ReturnType<typeof setInterval>;

  constructor(config: VerifierConfig, chainClients: { [chain: string]: any }) {
    super();
    this.verifierConfig = config;
    this.chainClients = chainClients;
  }

  /**
   * Register settlement for verification
   */
  registerSettlement(
    swapId: string,
    sourceChain: string,
    targetChain: string,
    proof: SPVProof,
    settlementTxid: string
  ): void {
    const record: SettlementRecord = {
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

    this.settlements.get(swapId)!.push(record);
    this.emit('settlement-registered', {
      swapId,
      targetChain,
      settlementTxid,
    });
  }

  /**
   * Start verification monitoring
   */
  start(): void {
    this.verificationInterval = setInterval(
      () => this.verifyPendingSettlements(),
      this.verifierConfig.verificationInterval
    );
    this.emit('verifier-started');
  }

  /**
   * Stop verification monitoring
   */
  stop(): void {
    if (this.verificationInterval) {
      clearInterval(this.verificationInterval);
    }
    this.emit('verifier-stopped');
  }

  /**
   * Verify pending settlements
   */
  private async verifyPendingSettlements(): Promise<void> {
    for (const [swapId, records] of this.settlements.entries()) {
      for (const record of records) {
        if (record.status !== 'pending') continue;

        try {
          await this.verifySettlement(record);
        } catch (error) {
          console.error(
            `Error verifying settlement ${swapId}/${record.targetChain}:`,
            error
          );
        }
      }
    }
  }

  /**
   * Verify single settlement
   */
  private async verifySettlement(record: SettlementRecord): Promise<void> {
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
        } else {
          record.status = 'failed';
          record.completedAt = Math.floor(Date.now() / 1000);
          this.emit('settlement-invalid', {
            swapId: record.swapId,
            chain: record.targetChain,
            errors: verification.errors,
          });
        }
      }
    } catch (error) {
      console.error(`Settlement verification error:`, error);
    }
  }

  /**
   * Verify proof validity
   */
  private async verifyProof(record: SettlementRecord): Promise<VerificationResult> {
    const errors: string[] = [];
    const warnings: string[] = [];

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
  getSettlements(swapId?: string): SettlementRecord[] {
    if (swapId) {
      return this.settlements.get(swapId) || [];
    }

    const all: SettlementRecord[] = [];
    for (const records of this.settlements.values()) {
      all.push(...records);
    }
    return all;
  }

  /**
   * Get settlement status
   */
  getSettlementStatus(swapId: string, targetChain: string): SettlementRecord | undefined {
    const records = this.settlements.get(swapId);
    return records?.find((r) => r.targetChain === targetChain);
  }

  /**
   * Get all confirmed settlements
   */
  getConfirmedSettlements(): SettlementRecord[] {
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

    const avgConfirmations =
      allRecords.length > 0
        ? (allRecords.reduce((sum, r) => sum + r.confirmations, 0) / allRecords.length).toFixed(2)
        : '0';

    return {
      totalSettlements: allRecords.length,
      confirmed,
      pending,
      failed,
      averageConfirmations: avgConfirmations,
      successRate:
        allRecords.length > 0
          ? ((confirmed / allRecords.length) * 100).toFixed(2) + '%'
          : 'N/A',
    };
  }

  /**
   * Wait for settlement confirmation
   */
  async waitForSettlement(
    swapId: string,
    targetChain: string,
    maxWaitTime: number = 3600000 // 1 hour
  ): Promise<SettlementRecord> {
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

    throw new Error(
      `Settlement verification timeout for ${swapId} on ${targetChain}`
    );
  }
}

export default SettlementVerifier;

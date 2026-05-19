/**
 * X3 Validator RPC Client
 * 
 * Client for interacting with X3 Chain's validator-related JSON-RPC endpoints.
 * Provides methods for querying validator status, leaderboard, and metrics.
 */

import { JsonRpcProvider } from './JsonRpcProvider';

/**
 * Validator status enum
 */
export enum ValidatorStatus {
    Online = 'online',
    Syncing = 'syncing',
    Offline = 'offline',
    Inactive = 'inactive',
}

/**
 * Validator information interface
 */
export interface ValidatorInfo {
    accountId: string;
    status: ValidatorStatus;
    score: number;
    blocksProduced: number;
    blocksFinalized: number;
    uptime: number;
    lastSeen: number;
    sessionKey?: string;
}

/**
 * Leaderboard entry interface
 */
export interface LeaderboardEntry {
    rank: number;
    accountId: string;
    score: number;
    blocksProduced: number;
    blocksFinalized: number;
    uptime: number;
    tps: number;
    latencyMs: number;
    gasEfficiency: number;
}

/**
 * Metrics snapshot interface
 */
export interface MetricsSnapshot {
    timestamp: number;
    blockHeight: number;
    validatorCount: number;
    activeValidators: number;
    avgTps: number;
    avgLatencyMs: number;
    totalGasUsed: number;
    gasEfficiencyScore: number;
}

/**
 * Validator RPC client
 */
export class ValidatorRpcClient {
    private provider: JsonRpcProvider;

    constructor(provider: JsonRpcProvider) {
        this.provider = provider;
    }

    /**
     * Get current validator set
     */
    async getValidators(): Promise<ValidatorInfo[]> {
        return this.provider.request<ValidatorInfo[]>('validator_getValidators', []);
    }

    /**
     * Get validator by account ID
     * @param accountId - The account ID of the validator
     */
    async getValidator(accountId: string): Promise<ValidatorInfo> {
        return this.provider.request<ValidatorInfo>('validator_getValidator', [accountId]);
    }

    /**
     * Get leaderboard with optional pagination
     * @param limit - Maximum number of entries to return
     * @param offset - Number of entries to skip
     */
    async getLeaderboard(limit?: number, offset?: number): Promise<LeaderboardEntry[]> {
        return this.provider.request<LeaderboardEntry[]>('validator_getLeaderboard', [limit, offset]);
    }

    /**
     * Get current metrics snapshot
     */
    async getMetrics(): Promise<MetricsSnapshot> {
        return this.provider.request<MetricsSnapshot>('validator_getMetrics', []);
    }

    /**
     * Get metrics for a specific block range
     * @param startBlock - Start block number
     * @param endBlock - End block number
     */
    async getStats(startBlock: number, endBlock: number): Promise<MetricsSnapshot> {
        return this.provider.request<MetricsSnapshot>('validator_getStats', [startBlock, endBlock]);
    }

    /**
     * Subscribe to validator status changes
     * @param callback - Callback function to receive updates
     */
    async subscribeValidators(callback: (validators: ValidatorInfo[]) => void): Promise<string> {
        return this.provider.subscribe('validator_subscribeValidators', [], callback);
    }

    /**
     * Subscribe to leaderboard updates
     * @param callback - Callback function to receive updates
     */
    async subscribeLeaderboard(callback: (leaderboard: LeaderboardEntry[]) => void): Promise<string> {
        return this.provider.subscribe('validator_subscribeLeaderboard', [], callback);
    }

    /**
     * Subscribe to metrics updates
     * @param callback - Callback function to receive updates
     */
    async subscribeMetrics(callback: (metrics: MetricsSnapshot) => void): Promise<string> {
        return this.provider.subscribe('validator_subscribeMetrics', [], callback);
    }
}

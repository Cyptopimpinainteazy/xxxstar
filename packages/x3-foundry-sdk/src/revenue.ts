/**
 * Revenue management module for X3 Foundry.
 *
 * Provides methods for querying revenue statistics, claiming earnings,
 * managing fee configurations, and distributing fees across the platform.
 *
 * @module @x3/foundry-sdk/revenue
 */

import type { ApiClient } from './client';
import type {
  RevenueReport,
  RevenueConfig,
  FeeMode,
  BigNumberish,
} from './types';

// =============================================================================
// RevenueManager
// =============================================================================

/**
 * Manages revenue operations for DApp projects on the X3 Foundry platform.
 *
 * Provides comprehensive revenue tracking, fee configuration, and
 * earnings claiming capabilities for creators and platform operators.
 *
 * @example
 * ```typescript
 * const revenue = new RevenueManager(apiClient);
 *
 * // Get revenue for a specific app
 * const appRevenue = await revenue.getRevenueByApp('project-123');
 *
 * // Claim unclaimed earnings
 * const tx = await revenue.claimRevenue('project-123');
 * ```
 */
export class RevenueManager {
  private readonly client: ApiClient;

  /**
   * Create a new RevenueManager instance.
   *
   * @param client - An authenticated ApiClient instance
   */
  constructor(client: ApiClient) {
    this.client = client;
  }

  /**
   * Get revenue report for a specific DApp project.
   *
   * Returns detailed revenue breakdown including platform fees,
   * creator earnings, treasury contributions, and transaction history
   * for the specified project within the given time period.
   *
   * @param projectId - The unique identifier of the project
   * @param periodStart - ISO timestamp for the start of the reporting period
   * @param periodEnd - ISO timestamp for the end of the reporting period
   * @returns A detailed revenue report for the app
   */
  async getRevenueByApp(
    projectId: string,
    periodStart?: string,
    periodEnd?: string
  ): Promise<RevenueReport> {
    return this.client.get<RevenueReport>(`/revenue/app/${projectId}`, {
      periodStart,
      periodEnd,
    });
  }

  /**
   * Get revenue report for a specific creator.
   *
   * Aggregates revenue across all projects owned by the specified
   * creator address within the given time period.
   *
   * @param creatorAddress - The blockchain address of the creator
   * @param periodStart - ISO timestamp for the start of the reporting period
   * @param periodEnd - ISO timestamp for the end of the reporting period
   * @returns A detailed revenue report for the creator
   */
  async getRevenueByCreator(
    creatorAddress: string,
    periodStart?: string,
    periodEnd?: string
  ): Promise<RevenueReport> {
    return this.client.get<RevenueReport>(
      `/revenue/creator/${creatorAddress}`,
      {
        periodStart,
        periodEnd,
      }
    );
  }

  /**
   * Get revenue report for a specific blockchain.
   *
   * Provides aggregate revenue statistics for all DApps deployed
   * on the specified chain within the given time period.
   *
   * @param chainId - The blockchain chain ID
   * @param periodStart - ISO timestamp for the start of the reporting period
   * @param periodEnd - ISO timestamp for the end of the reporting period
   * @returns A detailed revenue report for the chain
   */
  async getRevenueByChain(
    chainId: number,
    periodStart?: string,
    periodEnd?: string
  ): Promise<RevenueReport> {
    return this.client.get<RevenueReport>(`/revenue/chain/${chainId}`, {
      periodStart,
      periodEnd,
    });
  }

  /**
   * Get platform-wide revenue statistics.
   *
   * Returns aggregated revenue data across all projects, creators,
   * and chains on the Foundry platform.
   *
   * @param periodStart - ISO timestamp for the start of the reporting period
   * @param periodEnd - ISO timestamp for the end of the reporting period
   * @returns A detailed platform-wide revenue report
   */
  async getPlatformRevenue(
    periodStart?: string,
    periodEnd?: string
  ): Promise<RevenueReport> {
    return this.client.get<RevenueReport>('/revenue/platform', {
      periodStart,
      periodEnd,
    });
  }

  /**
   * Get unclaimed revenue for a project or creator.
   *
   * Returns the total amount of revenue that has been earned but
   * not yet claimed by the specified project or creator.
   *
   * @param projectId - Optional project ID to filter by
   * @param creatorAddress - Optional creator address to filter by
   * @returns The unclaimed revenue amount and details
   */
  async getUnclaimedRevenue(
    projectId?: string,
    creatorAddress?: string
  ): Promise<{
    totalUnclaimed: BigNumberish;
    projects: Array<{
      projectId: string;
      projectName: string;
      unclaimedAmount: BigNumberish;
      lastClaimedAt?: string;
    }>;
  }> {
    return this.client.get<{
      totalUnclaimed: BigNumberish;
      projects: Array<{
        projectId: string;
        projectName: string;
        unclaimedAmount: BigNumberish;
        lastClaimedAt?: string;
      }>;
    }>('/revenue/unclaimed', {
      projectId,
      creatorAddress,
    });
  }

  /**
   * Claim accrued revenue for a project.
   *
   * Triggers a payout of all unclaimed earnings for the specified
   * project to the creator's configured payout address.
   *
   * @param projectId - The unique identifier of the project
   * @returns Transaction details of the claim operation
   */
  async claimRevenue(
    projectId: string
  ): Promise<{
    transactionHash: string;
    claimedAmount: BigNumberish;
    claimedAt: string;
    recipientAddress: string;
  }> {
    return this.client.post<{
      transactionHash: string;
      claimedAmount: BigNumberish;
      claimedAt: string;
      recipientAddress: string;
    }>(`/revenue/claim/${projectId}`);
  }

  /**
   * Distribute fees for a project according to its revenue configuration.
   *
   * Manually triggers fee distribution for the specified project,
   * splitting collected fees between the creator, platform treasury,
   * and any configured beneficiaries.
   *
   * @param projectId - The unique identifier of the project
   * @param amount - Optional specific amount to distribute (in wei)
   * @returns Distribution details including amounts sent to each party
   */
  async distributeFees(
    projectId: string,
    amount?: BigNumberish
  ): Promise<{
    distributionId: string;
    totalDistributed: BigNumberish;
    creatorShare: BigNumberish;
    platformShare: BigNumberish;
    treasuryShare: BigNumberish;
    beneficiaryShares: Array<{
      address: string;
      amount: BigNumberish;
    }>;
    transactionHash: string;
  }> {
    return this.client.post<{
      distributionId: string;
      totalDistributed: BigNumberish;
      creatorShare: BigNumberish;
      platformShare: BigNumberish;
      treasuryShare: BigNumberish;
      beneficiaryShares: Array<{
        address: string;
        amount: BigNumberish;
      }>;
      transactionHash: string;
    }>(`/revenue/distribute/${projectId}`, { amount });
  }

  /**
   * Get the treasury split configuration for a project.
   *
   * Returns the current revenue distribution breakdown including
   * platform fee, creator share, treasury reserve, and any
   * beneficiary allocations.
   *
   * @param projectId - The unique identifier of the project
   * @returns The current treasury split configuration
   */
  async getTreasurySplit(
    projectId: string
  ): Promise<{
    feeMode: FeeMode;
    platformFeeBps: number;
    creatorShareBps: number;
    treasuryReserveBps: number;
    beneficiaries: Array<{
      address: string;
      shareBps: number;
    }>;
    minimumFee: BigNumberish;
    maximumFee: BigNumberish;
  }> {
    return this.client.get<{
      feeMode: FeeMode;
      platformFeeBps: number;
      creatorShareBps: number;
      treasuryReserveBps: number;
      beneficiaries: Array<{
        address: string;
        shareBps: number;
      }>;
      minimumFee: BigNumberish;
      maximumFee: BigNumberish;
    }>(`/revenue/treasury-split/${projectId}`);
  }

  /**
   * Get the current fee configuration for a project.
   *
   * Returns the complete RevenueConfig object including fee mode,
   * basis points, fee caps, and dynamic pricing settings.
   *
   * @param projectId - The unique identifier of the project
   * @returns The current fee configuration
   */
  async getFeeConfig(projectId: string): Promise<RevenueConfig> {
    return this.client.get<RevenueConfig>(`/revenue/fee-config/${projectId}`);
  }
}

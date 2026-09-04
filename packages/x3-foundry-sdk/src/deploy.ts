/**
 * Deployment management module for X3 Foundry.
 *
 * Provides methods for deploying DApps to target blockchains,
 * monitoring deployment status, verifying contracts, and
 * retrieving deployment receipts and cost estimates.
 *
 * @module @x3/foundry-sdk/deploy
 */

import type { ApiClient } from './client';
import type {
  DeploymentReceipt,
  DeployDappParams,
  BigNumberish,
} from './types';

// =============================================================================
// DeploymentManager
// =============================================================================

/**
 * Manages DApp deployments on the X3 Foundry platform.
 *
 * Handles the full deployment lifecycle including contract compilation,
 * on-chain deployment, verification, and status monitoring across
 * multiple blockchain networks.
 *
 * @example
 * ```typescript
 * const deploy = new DeploymentManager(apiClient);
 *
 * // Deploy a project
 * const receipt = await deploy.deploy({
 *   projectId: 'project-123',
 *   chainId: 1,
 *   verify: true,
 * });
 *
 * // Check deployment status
 * const status = await deploy.getDeploymentStatus(receipt.deploymentId);
 * ```
 */
export class DeploymentManager {
  private readonly client: ApiClient;

  /**
   * Create a new DeploymentManager instance.
   *
   * @param client - An authenticated ApiClient instance
   */
  constructor(client: ApiClient) {
    this.client = client;
  }

  /**
   * Deploy a DApp project to a target blockchain.
   *
   * Compiles the project source code and deploys it to the specified
   * blockchain network. Supports optional contract verification,
   * custom compiler versions, and configurable gas parameters.
   *
   * @param params - Deployment parameters including project ID and chain ID
   * @returns A deployment receipt with on-chain details
   */
  async deploy(params: DeployDappParams): Promise<DeploymentReceipt> {
    return this.client.post<DeploymentReceipt>('/deploy', params);
  }

  /**
   * Get the current status of a deployment.
   *
   * Returns the deployment's current state in its lifecycle,
   * including any error messages if the deployment failed.
   *
   * @param deploymentId - The unique deployment identifier
   * @returns Current deployment status information
   */
  async getDeploymentStatus(
    deploymentId: string
  ): Promise<{
    deploymentId: string;
    status: 'pending' | 'queued' | 'compiling' | 'deploying' | 'confirming' | 'completed' | 'failed';
    progress: number;
    currentStep: string;
    error?: string;
    startedAt?: string;
    completedAt?: string;
    estimatedTimeRemaining?: number;
  }> {
    return this.client.get<{
      deploymentId: string;
      status: 'pending' | 'queued' | 'compiling' | 'deploying' | 'confirming' | 'completed' | 'failed';
      progress: number;
      currentStep: string;
      error?: string;
      startedAt?: string;
      completedAt?: string;
      estimatedTimeRemaining?: number;
    }>(`/deploy/${deploymentId}/status`);
  }

  /**
   * Get the deployment receipt for a completed deployment.
   *
   * Returns the full deployment receipt including transaction hash,
   * deployed contract addresses, gas costs, and verification status.
   *
   * @param deploymentId - The unique deployment identifier
   * @returns The deployment receipt
   */
  async getDeploymentReceipt(deploymentId: string): Promise<DeploymentReceipt> {
    return this.client.get<DeploymentReceipt>(
      `/deploy/${deploymentId}/receipt`
    );
  }

  /**
   * Verify a deployed contract's source code.
   *
   * Submits the contract source code for on-chain verification,
   * matching the deployed bytecode against the compiled output.
   *
   * @param deploymentId - The unique deployment identifier
   * @param sourceCode - Optional source code override (uses stored source if omitted)
   * @returns Updated deployment receipt with verification status
   */
  async verifyDeployment(
    deploymentId: string,
    sourceCode?: string
  ): Promise<DeploymentReceipt> {
    return this.client.post<DeploymentReceipt>(
      `/deploy/${deploymentId}/verify`,
      { sourceCode }
    );
  }

  /**
   * Get all deployed contracts for a project.
   *
   * Returns a list of all contract addresses and their metadata
   * that were deployed as part of the specified project.
   *
   * @param projectId - The unique project identifier
   * @returns Array of deployed contract details
   */
  async getDeployedContracts(
    projectId: string
  ): Promise<
    Array<{
      contractName: string;
      address: string;
      chainId: number;
      deploymentId: string;
      deployedAt: string;
      verificationStatus: 'unverified' | 'pending' | 'verified' | 'failed';
      compilerVersion: string;
      abi?: Record<string, unknown>[];
    }>
  > {
    return this.client.get<
      Array<{
        contractName: string;
        address: string;
        chainId: number;
        deploymentId: string;
        deployedAt: string;
        verificationStatus: 'unverified' | 'pending' | 'verified' | 'failed';
        compilerVersion: string;
        abi?: Record<string, unknown>[];
      }>
    >(`/deploy/project/${projectId}/contracts`);
  }

  /**
   * Get the estimated deployment cost for a project.
   *
   * Provides a cost estimate for deploying the project to the
   * specified blockchain, including gas estimates and any
   * platform fees.
   *
   * @param projectId - The unique project identifier
   * @param chainId - The target blockchain chain ID
   * @returns Cost estimate details
   */
  async getDeploymentCost(
    projectId: string,
    chainId: number
  ): Promise<{
    estimatedGas: BigNumberish;
    estimatedGasCost: BigNumberish;
    platformFee: BigNumberish;
    totalEstimatedCost: BigNumberish;
    currency: string;
    gasPrice: BigNumberish;
    breakdown: Array<{
      item: string;
      cost: BigNumberish;
    }>;
  }> {
    return this.client.get<{
      estimatedGas: BigNumberish;
      estimatedGasCost: BigNumberish;
      platformFee: BigNumberish;
      totalEstimatedCost: BigNumberish;
      currency: string;
      gasPrice: BigNumberish;
      breakdown: Array<{
        item: string;
        cost: BigNumberish;
      }>;
    }>('/deploy/cost-estimate', {
      projectId,
      chainId,
    });
  }
}

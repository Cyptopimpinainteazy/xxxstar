/**
 * @module @x3/foundry-sdk
 *
 * X3 Foundry SDK - TypeScript SDK for DApp creation, deployment,
 * revenue management, and marketplace operations on the X3 Foundry platform.
 *
 * The Foundry platform enables developers to create, deploy, and monetize
 * decentralized applications across multiple blockchain networks with
 * built-in revenue sharing, template forking, and security auditing.
 *
 * @example
 * ```typescript
 * import { FoundryClient } from '@x3/foundry-sdk';
 *
 * const client = new FoundryClient({
 *   apiUrl: 'https://api.foundry.x3.ai',
 *   chainId: 1,
 *   privateKey: '0x...',
 * });
 *
 * // Create a new project
 * const project = await client.createProject({
 *   name: 'My DApp',
 *   description: 'An amazing decentralized application',
 *   dappType: DAppType.EVM,
 * });
 *
 * // Deploy the project
 * const receipt = await client.deployDapp({
 *   projectId: project.projectId,
 *   chainId: 1,
 * });
 * ```
 */

// =============================================================================
// Core Types
// =============================================================================

export {
  DAppType,
  FeeMode,
  ProjectState,
  ProjectStatus,
} from './types';

export type {
  RevenueConfig,
  SecurityReport,
  SimulationResult,
  DeploymentReceipt,
  MarketplaceListing,
  RevenueReport,
  AppHealthScore,
  ForkLineage,
  PricingTier,
  Template,
  TemplateRegistry,
  FoundryClientConfig,
  CreateProjectParams,
  GenerateDappParams,
  DeployDappParams,
  ForkProjectParams,
  UpdateFeeConfigParams,
  SearchMarketplaceParams,
  PaginationParams,
  PaginatedResponse,
  ApiError,
  ApiResponse,
} from './types';

// =============================================================================
// API Client
// =============================================================================

export { ApiClient } from './client';

export type { ApiClientConfig } from './client';

export {
  ApiClientError,
  AuthenticationError,
  RateLimitError,
  NotFoundError,
  ValidationError,
} from './client';

// =============================================================================
// Revenue Module
// =============================================================================

export { RevenueManager } from './revenue';

// =============================================================================
// Template Module
// =============================================================================

export { TemplateManager } from './templates';

// =============================================================================
// Deployment Module
// =============================================================================

export { DeploymentManager } from './deploy';

// =============================================================================
// FoundryClient
// =============================================================================

import { ApiClient } from './client';
import { RevenueManager } from './revenue';
import { TemplateManager } from './templates';
import { DeploymentManager } from './deploy';

import type {
  FoundryClientConfig,
  CreateProjectParams,
  GenerateDappParams,
  DeployDappParams,
  ForkProjectParams,
  UpdateFeeConfigParams,
  SearchMarketplaceParams,
  ProjectState,
  ProjectStatus,
  DAppType,
  RevenueConfig,
  SecurityReport,
  SimulationResult,
  DeploymentReceipt,
  MarketplaceListing,
  RevenueReport,
  AppHealthScore,
  ForkLineage,
  TemplateRegistry,
  BigNumberish,
} from './types';

/**
 * Main client for the X3 Foundry platform.
 *
 * Provides a unified interface for all Foundry operations including
 * project creation, DApp generation, deployment, revenue management,
 * template forking, and marketplace discovery.
 *
 * @example
 * ```typescript
 * const client = new FoundryClient({
 *   apiUrl: 'https://api.foundry.x3.ai',
 *   chainId: 1,
 * });
 *
 * // List available templates
 * const templates = await client.listTemplates();
 *
 * // Create a project from a template
 * const project = await client.createProject({
 *   name: 'My DeFi App',
 *   description: 'A decentralized exchange',
 *   dappType: DAppType.EVM,
 *   templateId: templates.templates[0].templateId,
 * });
 * ```
 */
export class FoundryClient {
  /** HTTP client for API communication */
  public readonly apiClient: ApiClient;

  /** Revenue management operations */
  public readonly revenue: RevenueManager;

  /** Template registry operations */
  public readonly templates: TemplateManager;

  /** Deployment operations */
  public readonly deploy: DeploymentManager;

  /** The configured chain ID */
  public readonly chainId: number;

  /**
   * Create a new FoundryClient instance.
   *
   * @param config - Configuration options for the client
   */
  constructor(config: FoundryClientConfig) {
    this.chainId = config.chainId;

    this.apiClient = new ApiClient({
      baseUrl: config.apiUrl,
      timeout: config.timeout ?? 30000,
      maxRetries: config.maxRetries ?? 3,
      apiKey: config.privateKey,
      headers: config.headers,
    });

    this.revenue = new RevenueManager(this.apiClient);
    this.templates = new TemplateManager(this.apiClient);
    this.deploy = new DeploymentManager(this.apiClient);
  }

  /**
   * Update the authentication private key/token.
   *
   * @param privateKey - The new private key or auth token
   */
  setAuth(privateKey: string): void {
    this.apiClient.setAuthToken(privateKey);
  }

  // ===========================================================================
  // Project Management
  // ===========================================================================

  /**
   * Create a new DApp project on the Foundry platform.
   *
   * Initializes a new project with the specified name, description,
   * DApp type, and optional revenue configuration. If a templateId
   * is provided, the project will be pre-populated with the template's
   * structure.
   *
   * @param params - Project creation parameters
   * @returns The newly created project details
   */
  async createProject(
    params: CreateProjectParams
  ): Promise<{
    projectId: string;
    name: string;
    description: string;
    dappType: DAppType;
    state: ProjectState;
    status: ProjectStatus;
    revenueConfig: RevenueConfig;
    createdAt: string;
    owner: string;
  }> {
    return this.apiClient.post<{
      projectId: string;
      name: string;
      description: string;
      dappType: DAppType;
      state: ProjectState;
      status: ProjectStatus;
      revenueConfig: RevenueConfig;
      createdAt: string;
      owner: string;
    }>('/projects', params);
  }

  /**
   * Generate a DApp from a template with custom configuration.
   *
   * Uses the specified template to generate a fully-configured DApp
   * project with custom parameters. The generated project can then
   * be deployed directly to the target chains.
   *
   * @param params - DApp generation parameters
   * @returns The generated project details
   */
  async generateDapp(
    params: GenerateDappParams
  ): Promise<{
    projectId: string;
    name: string;
    templateId: string;
    generatedFiles: string[];
    config: Record<string, unknown>;
    targetChains: number[];
    createdAt: string;
  }> {
    return this.apiClient.post<{
      projectId: string;
      name: string;
      templateId: string;
      generatedFiles: string[];
      config: Record<string, unknown>;
      targetChains: number[];
      createdAt: string;
    }>('/projects/generate', params);
  }

  /**
   * Simulate a DApp execution in a sandboxed environment.
   *
   * Runs the project's smart contracts or programs in a simulated
   * blockchain environment, returning execution traces, gas estimates,
   * state changes, and emitted events.
   *
   * @param projectId - The project identifier to simulate
   * @param input - Optional simulation input parameters
   * @returns Simulation results with execution details
   */
  async simulateDapp(
    projectId: string,
    input?: Record<string, unknown>
  ): Promise<SimulationResult> {
    return this.apiClient.post<SimulationResult>(
      `/projects/${projectId}/simulate`,
      input
    );
  }

  /**
   * Run a security audit on a DApp project.
   *
   * Performs comprehensive security analysis including vulnerability
   * scanning, gas optimization suggestions, and compliance checks.
   * Returns a detailed security report with findings and recommendations.
   *
   * @param projectId - The project identifier to audit
   * @returns Security audit report with findings
   */
  async auditDapp(projectId: string): Promise<SecurityReport> {
    return this.apiClient.post<SecurityReport>(
      `/projects/${projectId}/audit`
    );
  }

  /**
   * Deploy a DApp project to a target blockchain.
   *
   * Compiles and deploys the project to the specified blockchain network.
   * Supports optional contract verification and custom gas parameters.
   *
   * @param params - Deployment parameters
   * @returns Deployment receipt with on-chain details
   */
  async deployDapp(params: DeployDappParams): Promise<DeploymentReceipt> {
    return this.deploy.deploy(params);
  }

  // ===========================================================================
  // Revenue & Analytics
  // ===========================================================================

  /**
   * Get revenue statistics for a project or creator.
   *
   * Returns comprehensive revenue data including platform fees,
   * creator earnings, treasury contributions, and transaction history.
   *
   * @param projectId - Optional project ID to filter by
   * @param creatorAddress - Optional creator address to filter by
   * @param periodStart - ISO timestamp for period start
   * @param periodEnd - ISO timestamp for period end
   * @returns Revenue report
   */
  async getRevenueStats(
    projectId?: string,
    creatorAddress?: string,
    periodStart?: string,
    periodEnd?: string
  ): Promise<RevenueReport> {
    if (projectId) {
      return this.revenue.getRevenueByApp(projectId, periodStart, periodEnd);
    }
    if (creatorAddress) {
      return this.revenue.getRevenueByCreator(
        creatorAddress,
        periodStart,
        periodEnd
      );
    }
    return this.revenue.getPlatformRevenue(periodStart, periodEnd);
  }

  /**
   * Get health metrics for a deployed DApp project.
   *
   * Returns operational health data including uptime, gas efficiency,
   * error rates, latency, and any active alerts.
   *
   * @param projectId - The project identifier
   * @returns Health score and metrics
   */
  async getProjectHealth(projectId: string): Promise<AppHealthScore> {
    return this.apiClient.get<AppHealthScore>(
      `/projects/${projectId}/health`
    );
  }

  /**
   * Update the fee configuration for a project.
   *
   * Modifies the revenue sharing configuration including fee mode,
   * platform fee percentage, creator share, and fee caps.
   *
   * @param params - Fee configuration update parameters
   * @returns Updated revenue configuration
   */
  async updateFeeConfig(
    params: UpdateFeeConfigParams
  ): Promise<RevenueConfig> {
    return this.apiClient.put<RevenueConfig>(
      `/projects/${params.projectId}/fees`,
      params
    );
  }

  /**
   * Claim accrued revenue for a project.
   *
   * Triggers a payout of all unclaimed earnings to the creator's
   * configured payout address.
   *
   * @param projectId - The project identifier
   * @returns Transaction details of the claim
   */
  async claimCreatorRevenue(
    projectId: string
  ): Promise<{
    transactionHash: string;
    claimedAmount: BigNumberish;
    claimedAt: string;
    recipientAddress: string;
  }> {
    return this.revenue.claimRevenue(projectId);
  }

  // ===========================================================================
  // Forking & Templates
  // ===========================================================================

  /**
   * Fork an existing project to create a new one.
   *
   * Creates a copy of an existing project with a new name and
   * optional description. The fork lineage is tracked for
   * attribution and version management.
   *
   * @param params - Fork project parameters
   * @returns The newly forked project details
   */
  async forkProject(
    params: ForkProjectParams
  ): Promise<{
    projectId: string;
    name: string;
    sourceProjectId: string;
    forkLineage: ForkLineage;
    createdAt: string;
  }> {
    return this.apiClient.post<{
      projectId: string;
      name: string;
      sourceProjectId: string;
      forkLineage: ForkLineage;
      createdAt: string;
    }>('/projects/fork', params);
  }

  /**
   * List available DApp templates in the registry.
   *
   * Returns a paginated registry of templates with filtering
   * and sorting capabilities.
   *
   * @param params - Optional pagination parameters
   * @returns Paginated template registry
   */
  async listTemplates(
    params?: {
      page?: number;
      pageSize?: number;
      category?: string;
      chainId?: number;
      dappType?: DAppType;
    }
  ): Promise<TemplateRegistry> {
    return this.apiClient.get<TemplateRegistry>('/templates', params);
  }

  /**
   * Search the marketplace for DApps and templates.
   *
   * Searches the Foundry marketplace with full-text query,
   * filters, and sorting options.
   *
   * @param params - Search parameters
   * @returns Paginated marketplace listings
   */
  async searchMarketplace(
    params?: SearchMarketplaceParams
  ): Promise<{
    listings: MarketplaceListing[];
    total: number;
    page: number;
    pageSize: number;
    totalPages: number;
  }> {
    return this.apiClient.get<{
      listings: MarketplaceListing[];
      total: number;
      page: number;
      pageSize: number;
      totalPages: number;
    }>('/marketplace/search', params as Record<string, unknown>);
  }
}

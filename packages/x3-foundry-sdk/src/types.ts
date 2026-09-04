/**
 * Core type definitions for X3 Foundry SDK
 *
 * These types mirror the Rust types defined in the X3 Foundry runtime pallets,
 * providing type-safe interaction with the Foundry API for DApp creation,
 * deployment, revenue management, and marketplace operations.
 *
 * @module @x3/foundry-sdk/types
 */

import type { BigNumberish } from 'ethers';

// Re-exported so consumers of this module can import the shared ethers
// scalar type from a single stable location (`. /types`).
export type { BigNumberish };

// =============================================================================
// Enums
// =============================================================================

/**
 * Supported DApp types on the X3 Foundry platform.
 *
 * Each variant corresponds to a distinct execution environment or framework
 * that can be used to build and deploy decentralized applications.
 */
export enum DAppType {
  /** EVM-compatible smart contract (Solidity, Vyper, etc.) */
  EVM = 'evm',
  /** SVM (Solana Virtual Machine) program */
  SVM = 'svm',
  /** X3 Chain dual-VM Comit (bundled EVM + SVM execution) */
  Comit = 'comit',
  /** Substrate pallet-based runtime module */
  Substrate = 'substrate',
  /** Move language smart contract (Aptos/Sui) */
  Move = 'move',
  /** CosmWasm smart contract (Cosmos ecosystem) */
  CosmWasm = 'cosmwasm',
  /** WebAssembly-based application */
  Wasm = 'wasm',
  /** ZK-proof based application (circom, noir, etc.) */
  ZK = 'zk',
}

/**
 * Fee distribution modes for revenue sharing.
 *
 * Determines how fees collected from DApp usage are split between
 * the creator, platform treasury, and other stakeholders.
 */
export enum FeeMode {
  /** Equal split between creator and platform */
  Equal = 'equal',
  /** Creator receives a larger percentage share */
  CreatorFavored = 'creator_favored',
  /** Platform receives a larger percentage share */
  PlatformFavored = 'platform_favored',
  /** Custom fee split defined by the creator */
  Custom = 'custom',
  /** Dynamic fee split based on usage metrics */
  Dynamic = 'dynamic',
  /** No platform fee — creator keeps 100% */
  ZeroPlatform = 'zero_platform',
}

/**
 * Lifecycle state of a project on the Foundry platform.
 */
export enum ProjectState {
  /** Project has been created but not yet deployed */
  Draft = 'draft',
  /** Project is actively deployed and running */
  Active = 'active',
  /** Project has been paused by the creator or platform */
  Paused = 'paused',
  /** Project has been deprecated and is no longer maintained */
  Deprecated = 'deprecated',
  /** Project has been permanently archived */
  Archived = 'archived',
  /** Project deployment failed and requires intervention */
  Failed = 'failed',
  /** Project is under audit review */
  Auditing = 'auditing',
}

/**
 * Deployment status of a project.
 */
export enum ProjectStatus {
  /** Not yet deployed */
  NotDeployed = 'not_deployed',
  /** Deployment is in progress */
  Deploying = 'deploying',
  /** Successfully deployed */
  Deployed = 'deployed',
  /** Deployment failed */
  Failed = 'failed',
  /** Contract verification is pending */
  Verifying = 'verifying',
  /** Verified and confirmed on-chain */
  Verified = 'verified',
}

// =============================================================================
// Core Interfaces
// =============================================================================

/**
 * Revenue configuration for a DApp project.
 *
 * Defines how fees and revenue are distributed among stakeholders
 * including the creator, platform treasury, and optional third parties.
 */
export interface RevenueConfig {
  /** The fee mode determining distribution logic */
  feeMode: FeeMode;
  /** Platform fee percentage (basis points, e.g. 250 = 2.5%) */
  platformFeeBps: number;
  /** Creator revenue share percentage (basis points) */
  creatorShareBps: number;
  /** Treasury reserve percentage (basis points) */
  treasuryReserveBps: number;
  /** Optional third-party beneficiary address and share */
  beneficiaries?: Array<{
    /** Address of the beneficiary */
    address: string;
    /** Share in basis points */
    shareBps: number;
  }>;
  /** Minimum fee in native token (wei) */
  minimumFee: BigNumberish;
  /** Maximum fee cap in native token (wei), 0 = no cap */
  maximumFee: BigNumberish;
  /** Whether dynamic fee adjustment is enabled */
  dynamicPricing: boolean;
}

/**
 * Security audit report for a DApp project.
 *
 * Contains the results of automated and manual security analysis
 * performed on the project's smart contracts or programs.
 */
export interface SecurityReport {
  /** Unique report identifier */
  reportId: string;
  /** Project identifier this report belongs to */
  projectId: string;
  /** Overall security score (0-100) */
  score: number;
  /** ISO timestamp of the audit */
  timestamp: string;
  /** List of vulnerabilities found, ordered by severity */
  vulnerabilities: Array<{
    /** Severity level */
    severity: 'critical' | 'high' | 'medium' | 'low' | 'informational';
    /** Human-readable title */
    title: string;
    /** Detailed description of the vulnerability */
    description: string;
    /** Affected contract/program file path */
    filePath: string;
    /** Line number range in source code */
    lineRange: [number, number];
    /** Recommended fix description */
    recommendation: string;
    /** CWE identifier if applicable */
    cweId?: string;
  }>;
  /** Gas optimization suggestions */
  gasOptimizations: Array<{
    /** Description of the optimization */
    description: string;
    /** Estimated gas savings */
    estimatedSavings: string;
    /** File path where optimization applies */
    filePath: string;
  }>;
  /** Whether the report passed all critical checks */
  passed: boolean;
  /** Summary of the audit findings */
  summary: string;
}

/**
 * Result of a DApp simulation run.
 *
 * Provides detailed execution traces, gas estimates, and state changes
 * from simulating the DApp in a sandboxed environment.
 */
export interface SimulationResult {
  /** Unique simulation identifier */
  simulationId: string;
  /** Project identifier */
  projectId: string;
  /** Whether the simulation completed successfully */
  success: boolean;
  /** Total gas/compute units consumed */
  gasUsed: BigNumberish;
  /** Estimated cost in native token (wei) */
  estimatedCost: BigNumberish;
  /** Execution trace entries */
  trace: Array<{
    /** Step number in execution */
    step: number;
    /** Opcode or instruction executed */
    opcode: string;
    /** Gas cost of this step */
    gasCost: number;
    /** Stack depth at this step */
    depth: number;
    /** Contract/program address */
    address: string;
    /** Return data if applicable */
    returnData?: string;
  }>;
  /** State changes resulting from execution */
  stateChanges: Array<{
    /** Address of the affected contract/account */
    address: string;
    /** Storage slot key (hex) */
    slot: string;
    /** Previous value (hex) */
    oldValue: string;
    /** New value (hex) */
    newValue: string;
  }>;
  /** Events emitted during simulation */
  events: Array<{
    /** Emitting contract address */
    address: string;
    /** Event signature / topic */
    topics: string[];
    /** Event data payload (hex) */
    data: string;
  }>;
  /** Error message if simulation failed */
  error?: string;
  /** Duration of simulation in milliseconds */
  durationMs: number;
}

/**
 * On-chain deployment receipt.
 *
 * Contains all information needed to verify and interact with
 * a deployed DApp on the target blockchain.
 */
export interface DeploymentReceipt {
  /** Unique deployment identifier */
  deploymentId: string;
  /** Project identifier */
  projectId: string;
  /** Target chain identifier */
  chainId: number;
  /** Deployment transaction hash */
  transactionHash: string;
  /** Block number where deployment was included */
  blockNumber: number;
  /** Block hash */
  blockHash: string;
  /** Deployed contract/program addresses */
  contractAddresses: Record<string, string>;
  /** Gas cost of deployment */
  gasUsed: BigNumberish;
  /** Effective gas price (wei) */
  gasPrice: BigNumberish;
  /** Total deployment cost (wei) */
  totalCost: BigNumberish;
  /** ISO timestamp of deployment */
  deployedAt: string;
  /** Deployer account address */
  deployer: string;
  /** Current verification status */
  verificationStatus: 'unverified' | 'pending' | 'verified' | 'failed';
  /** IPFS CID of the source code metadata */
  sourceCodeCid?: string;
  /** Compiler version used */
  compilerVersion?: string;
  /** Metadata URL for contract ABI/interface */
  metadataUrl?: string;
}

/**
 * Marketplace listing for a DApp template or project.
 *
 * Represents a published item available for discovery, forking,
 * or installation through the Foundry marketplace.
 */
export interface MarketplaceListing {
  /** Unique listing identifier */
  listingId: string;
  /** Title of the listing */
  title: string;
  /** Short description (max 280 characters) */
  description: string;
  /** Detailed markdown description */
  longDescription: string;
  /** DApp type category */
  dappType: DAppType;
  /** Creator/author address */
  author: string;
  /** Current version string (semver) */
  version: string;
  /** Price in native token (wei), 0 = free */
  price: BigNumberish;
  /** Total number of downloads/installs */
  downloads: number;
  /** Average rating (0-5) */
  rating: number;
  /** Number of ratings received */
  ratingCount: number;
  /** Array of tag strings */
  tags: string[];
  /** Array of supported chain IDs */
  supportedChains: number[];
  /** IPFS CID for the source code bundle */
  sourceCid: string;
  /** IPFS CID for documentation */
  docsCid?: string;
  /** URL to project icon/image */
  iconUrl?: string;
  /** ISO timestamp of listing creation */
  createdAt: string;
  /** ISO timestamp of last update */
  updatedAt: string;
  /** Whether the listing is verified by the platform */
  verified: boolean;
  /** License type (e.g., "MIT", "GPL-3.0", "Apache-2.0") */
  license: string;
}

/**
 * Revenue report for a DApp or creator.
 *
 * Provides a comprehensive breakdown of revenue generated,
 * fees collected, and amounts available for claiming.
 */
export interface RevenueReport {
  /** Unique report identifier */
  reportId: string;
  /** Scope of the report (app, creator, chain, platform) */
  scope: 'app' | 'creator' | 'chain' | 'platform';
  /** Identifier matching the scope (app ID, creator address, chain ID) */
  scopeId: string;
  /** ISO timestamp for the start of the reporting period */
  periodStart: string;
  /** ISO timestamp for the end of the reporting period */
  periodEnd: string;
  /** Total revenue generated in the period */
  totalRevenue: BigNumberish;
  /** Total platform fees collected */
  platformFees: BigNumberish;
  /** Total creator earnings */
  creatorEarnings: BigNumberish;
  /** Total treasury contributions */
  treasuryContributions: BigNumberish;
  /** Amount claimed by the creator */
  claimedAmount: BigNumberish;
  /** Amount still available for claiming */
  unclaimedAmount: BigNumberish;
  /** Number of transactions in the period */
  transactionCount: number;
  /** Breakdown by individual transaction */
  transactions?: Array<{
    /** Transaction hash */
    txHash: string;
    /** ISO timestamp */
    timestamp: string;
    /** Revenue amount */
    amount: BigNumberish;
    /** Fee deducted */
    fee: BigNumberish;
    /** Payer address */
    from: string;
  }>;
  /** ISO timestamp when this report was generated */
  generatedAt: string;
}

/**
 * Health score and metrics for a deployed DApp.
 *
 * Provides operational insights including uptime, gas efficiency,
 * error rates, and overall reliability scoring.
 */
export interface AppHealthScore {
  /** Project identifier */
  projectId: string;
  /** Overall health score (0-100) */
  overall: number;
  /** Uptime percentage over the evaluation period */
  uptime: number;
  /** Average gas efficiency score (0-100) */
  gasEfficiency: number;
  /** Error rate as percentage of total transactions */
  errorRate: number;
  /** Average response latency in milliseconds */
  latencyMs: number;
  /** Number of transactions in evaluation period */
  totalTransactions: number;
  /** Number of failed transactions */
  failedTransactions: number;
  /** ISO timestamp of last activity */
  lastActivity: string;
  /** Evaluation period in hours */
  evaluationPeriodHours: number;
  /** Individual metric scores */
  metrics: Record<string, number>;
  /** Any active alerts or warnings */
  alerts: Array<{
    /** Severity level */
    severity: 'info' | 'warning' | 'critical';
    /** Alert message */
    message: string;
    /** ISO timestamp */
    timestamp: string;
  }>;
}

/**
 * Fork lineage tracking for a project.
 *
 * Records the ancestry of forked projects, enabling traceability
 * back to the original source template or project.
 */
export interface ForkLineage {
  /** The fork record identifier */
  forkId: string;
  /** The original project/template that was forked */
  parentId: string;
  /** The new project created from the fork */
  childId: string;
  /** Fork depth from the original root project */
  depth: number;
  /** Root project identifier in the ancestry chain */
  rootId: string;
  /** Address of the account that performed the fork */
  forkedBy: string;
  /** ISO timestamp of the fork operation */
  forkedAt: string;
  /** Commit hash or version of the parent at fork time */
  parentVersion: string;
  /** Any modifications made post-fork */
  changes?: string[];
}

/**
 * Pricing tier for a template or service.
 *
 * Defines the cost structure for using a particular template
 * or platform service, including free and premium options.
 */
export interface PricingTier {
  /** Tier identifier */
  tierId: string;
  /** Human-readable name (e.g., "Free", "Pro", "Enterprise") */
  name: string;
  /** Price in native token (wei), 0 = free */
  price: BigNumberish;
  /** Billing interval */
  interval: 'one_time' | 'monthly' | 'yearly' | 'per_use';
  /** Description of what this tier includes */
  description: string;
  /** Feature flags included in this tier */
  features: string[];
  /** Maximum number of deployments allowed */
  maxDeployments: number;
  /** Maximum transactions per period */
  maxTransactions: number;
  /** Whether this is the default/free tier */
  isDefault: boolean;
  /** Whether this tier is currently active */
  isActive: boolean;
}

/**
 * A DApp template available in the Foundry registry.
 *
 * Templates provide pre-built project structures that can be
 * forked and customized by creators.
 */
export interface Template {
  /** Unique template identifier */
  templateId: string;
  /** Template name */
  name: string;
  /** Short description */
  description: string;
  /** DApp type this template is for */
  dappType: DAppType;
  /** Creator/author address */
  author: string;
  /** Current version (semver) */
  version: string;
  /** Array of supported chain IDs */
  supportedChains: number[];
  /** Category tags */
  categories: string[];
  /** Pricing information */
  pricing: PricingTier[];
  /** IPFS CID for the template source */
  sourceCid: string;
  /** IPFS CID for documentation */
  docsCid?: string;
  /** URL to preview image */
  previewUrl?: string;
  /** Total number of forks */
  forkCount: number;
  /** Average rating (0-5) */
  rating: number;
  /** Number of ratings */
  ratingCount: number;
  /** ISO timestamp of creation */
  createdAt: string;
  /** ISO timestamp of last update */
  updatedAt: string;
  /** Whether template is verified */
  verified: boolean;
  /** Required dependencies (package names) */
  dependencies: string[];
  /** Framework or toolchain required */
  framework?: string;
  /** Minimum compiler version required */
  minCompilerVersion?: string;
}

/**
 * Registry of all available templates.
 *
 * Provides a paginated view of the template marketplace
 * with filtering and sorting metadata.
 */
export interface TemplateRegistry {
  /** Array of templates in the current page */
  templates: Template[];
  /** Total number of templates matching the query */
  total: number;
  /** Current page number (0-indexed) */
  page: number;
  /** Number of items per page */
  pageSize: number;
  /** Total number of pages */
  totalPages: number;
  /** Available category filters */
  availableCategories: string[];
  /** Available chain ID filters */
  availableChains: number[];
  /** Available DApp type filters */
  availableDAppTypes: DAppType[];
}

// =============================================================================
// API Request/Response Types
// =============================================================================

/**
 * Generic pagination parameters for list endpoints.
 */
export interface PaginationParams {
  /** Page number (0-indexed) */
  page?: number;
  /** Number of items per page */
  pageSize?: number;
}

/**
 * Generic paginated API response wrapper.
 */
export interface PaginatedResponse<T> {
  data: T[];
  total: number;
  page: number;
  pageSize: number;
  totalPages: number;
}

/**
 * Standard API error response.
 */
export interface ApiError {
  /** HTTP status code */
  statusCode: number;
  /** Error code string */
  code: string;
  /** Human-readable error message */
  message: string;
  /** Detailed error information (if available) */
  details?: Record<string, unknown>;
  /** Request ID for tracing */
  requestId?: string;
}

/**
 * Standard API response wrapper.
 */
export interface ApiResponse<T> {
  success: boolean;
  data: T;
  requestId?: string;
  timestamp: string;
}

// =============================================================================
// Configuration Types
// =============================================================================

/**
 * Configuration options for the FoundryClient.
 */
export interface FoundryClientConfig {
  /** Base URL for the Foundry API */
  apiUrl: string;
  /** Target blockchain chain ID */
  chainId: number;
  /** Optional private key for authenticated operations */
  privateKey?: string;
  /** Request timeout in milliseconds (default: 30000) */
  timeout?: number;
  /** Maximum number of retry attempts (default: 3) */
  maxRetries?: number;
  /** Additional headers to include in all requests */
  headers?: Record<string, string>;
}

/**
 * Parameters for creating a new project.
 */
export interface CreateProjectParams {
  /** Project name */
  name: string;
  /** Project description */
  description: string;
  /** DApp type */
  dappType: DAppType;
  /** Initial revenue configuration */
  revenueConfig?: RevenueConfig;
  /** Array of tags */
  tags?: string[];
  /** Optional template ID to fork from */
  templateId?: string;
}

/**
 * Parameters for generating a DApp from a template.
 */
export interface GenerateDappParams {
  /** Template identifier */
  templateId: string;
  /** Project name for the generated DApp */
  name: string;
  /** Configuration parameters for generation */
  config: Record<string, unknown>;
  /** Target chain IDs */
  targetChains: number[];
}

/**
 * Parameters for deploying a DApp.
 */
export interface DeployDappParams {
  /** Project identifier */
  projectId: string;
  /** Target chain ID */
  chainId: number;
  /** Compiler version to use */
  compilerVersion?: string;
  /** Constructor arguments (for EVM contracts) */
  constructorArgs?: unknown[];
  /** Whether to verify the contract after deployment */
  verify?: boolean;
  /** Gas limit for deployment */
  gasLimit?: BigNumberish;
  /** Gas price for deployment (wei) */
  gasPrice?: BigNumberish;
}

/**
 * Parameters for forking an existing project.
 */
export interface ForkProjectParams {
  /** Source project ID to fork */
  sourceProjectId: string;
  /** Name for the new forked project */
  newName: string;
  /** Optional description override */
  description?: string;
}

/**
 * Parameters for updating fee configuration.
 */
export interface UpdateFeeConfigParams {
  /** Project identifier */
  projectId: string;
  /** New fee mode */
  feeMode?: FeeMode;
  /** Platform fee in basis points */
  platformFeeBps?: number;
  /** Creator share in basis points */
  creatorShareBps?: number;
  /** Treasury reserve in basis points */
  treasuryReserveBps?: number;
  /** Minimum fee */
  minimumFee?: BigNumberish;
  /** Maximum fee cap */
  maximumFee?: BigNumberish;
}

/**
 * Parameters for searching the marketplace.
 */
export interface SearchMarketplaceParams {
  /** Search query string */
  query?: string;
  /** Filter by DApp type */
  dappType?: DAppType;
  /** Filter by chain ID */
  chainId?: number;
  /** Filter by category/tag */
  category?: string;
  /** Sort field */
  sortBy?: 'rating' | 'downloads' | 'newest' | 'price';
  /** Sort direction */
  sortOrder?: 'asc' | 'desc';
  /** Minimum rating filter (0-5) */
  minRating?: number;
  /** Price range minimum (wei) */
  minPrice?: BigNumberish;
  /** Price range maximum (wei) */
  maxPrice?: BigNumberish;
  /** Page number */
  page?: number;
  /** Page size */
  pageSize?: number;
}

/**
 * Core type definitions for X3 Chain SDK
 *
 * These types mirror the runtime types defined in the X3 Kernel pallet,
 * providing type-safe interaction with the blockchain.
 */

import type { HexString } from '@polkadot/util/types';

// =============================================================================
// Primitive Types
// =============================================================================

/** 32-byte hash (H256) represented as hex string */
export type Hash = HexString;

/** Account identifier (Substrate AccountId32) */
export type AccountId = string;

/** Asset identifier */
export type AssetId = number;

/** Balance in smallest unit (similar to wei for EVM) */
export type Balance = bigint;

/** Block number */
export type BlockNumber = number;

/** Unix timestamp in seconds */
export type Timestamp = number;

/** Nonce for Comit submission */
export type Nonce = bigint;

// =============================================================================
// Comit Types
// =============================================================================

/**
 * A Comit is the atomic unit of cross-VM execution in X3 Chain.
 * It bundles EVM and SVM payloads for simultaneous execution.
 */
export interface Comit {
  /** Unique identifier for this Comit (32-byte hash) */
  comitId: Hash;

  /** Account that submitted the Comit */
  origin: AccountId;

  /** EVM transaction payload (RLP-encoded or raw calldata) */
  evmPayload: Uint8Array;

  /** SVM transaction payload (BPF program or instruction) */
  svmPayload: Uint8Array;

  /** Submitter's nonce at time of submission */
  nonce: Nonce;

  /** Fee paid for execution */
  fee: Balance;

  /** Cryptographic commitment to input parameters */
  prepareRoot: Hash;
}

/**
 * Input parameters for creating a new Comit
 */
export interface ComitInput {
  /** EVM payload (hex string or Uint8Array) */
  evmPayload?: HexString | Uint8Array;

  /** SVM payload (hex string or Uint8Array) */
  svmPayload?: HexString | Uint8Array;

  /** Fee to pay (in smallest unit) */
  fee: Balance;

  /** Optional: override prepare_root (default: computed from inputs) */
  prepareRoot?: Hash;
}

// =============================================================================
// Execution Types
// =============================================================================

/**
 * Log entry emitted during VM execution
 */
export interface ExecutionLog {
  /** Contract/program address that emitted the log */
  address: Uint8Array;

  /** Indexed topics (for EVM events) */
  topics: Hash[];

  /** Log data payload */
  data: Uint8Array;
}

/**
 * State change resulting from VM execution
 */
export interface StateChange {
  /** Account/contract address affected */
  address: Uint8Array;

  /** Storage slot key */
  key: Hash;

  /** New value at the storage slot */
  value: Hash;
}

/**
 * Receipt returned after VM execution
 */
export interface ExecutionReceipt {
  /** Whether execution succeeded */
  success: boolean;

  /** Gas/compute units consumed */
  gasUsed: bigint;

  /** Return data from execution */
  returnData: Uint8Array;

  /** Logs emitted during execution */
  logs: ExecutionLog[];

  /** State changes made during execution */
  stateChanges: StateChange[];
}

/**
 * Combined result from dual-VM execution
 */
export interface ComitResult {
  /** The submitted Comit */
  comit: Comit;

  /** EVM execution receipt (if EVM payload was provided) */
  evmReceipt?: ExecutionReceipt;

  /** SVM execution receipt (if SVM payload was provided) */
  svmReceipt?: ExecutionReceipt;

  /** Combined state after execution */
  sphereState: SphereState;

  /** Block number where Comit was included */
  blockNumber: BlockNumber;

  /** Block hash where Comit was included */
  blockHash: Hash;

  /** Index within the block */
  extrinsicIndex: number;
}

// =============================================================================
// State Types
// =============================================================================

/**
 * Unified state representation for the X3 Chain
 */
export interface SphereState {
  /** Merkle root of the combined state */
  stateRoot: Hash;

  /** Block number when state was computed */
  blockNumber: BlockNumber;

  /** Timestamp of state computation */
  timestamp: Timestamp;
}

/**
 * Asset metadata stored in the canonical ledger
 */
export interface AssetMetadata {
  /** Asset symbol (e.g., "X3", "ETH", "SOL") */
  symbol: string;

  /** Decimal places for display */
  decimals: number;
}

/**
 * Canonical ledger entry for an account's asset balance
 */
export interface LedgerEntry {
  /** Account holding the balance */
  account: AccountId;

  /** Asset identifier */
  assetId: AssetId;

  /** Current balance */
  balance: Balance;
}

// =============================================================================
// Event Types
// =============================================================================

/**
 * Event emitted when a Comit is submitted
 */
export interface ComitSubmittedEvent {
  comitId: Hash;
  origin: AccountId;
  nonce: Nonce;
  fee: Balance;
}

/**
 * Event emitted when Comit execution starts
 */
export interface ComitExecutionStartedEvent {
  comitId: Hash;
  timestamp: Timestamp;
}

/**
 * Event emitted when Comit execution completes
 */
export interface ComitExecutionCompletedEvent {
  comitId: Hash;
  success: boolean;
  gasUsed: bigint;
}

/**
 * Event emitted when a Comit is finalized
 */
export interface ComitFinalizedEvent {
  comitId: Hash;
}

/**
 * Event emitted when a Comit fails
 */
export interface ComitFailedEvent {
  comitId: Hash;
  reason: ComitFailureReason;
}

/**
 * Reasons why a Comit can fail
 */
export type ComitFailureReason =
  | { type: 'InvalidNonce'; expected: Nonce; provided: Nonce }
  | { type: 'InsufficientBalance'; required: Balance; available: Balance }
  | { type: 'EvmExecutionFailed'; gasUsed: bigint; error: string }
  | { type: 'SvmExecutionFailed'; computeUnits: bigint; error: string }
  | { type: 'VerificationFailed'; reason: string }
  | { type: 'Unauthorized' }
  | { type: 'RateLimitExceeded' }
  | { type: 'DuplicateComitId' };

// =============================================================================
// Authority Types
// =============================================================================

/**
 * Authority (validator) in the consensus set
 */
export interface Authority {
  /** Account ID of the authority */
  accountId: AccountId;

  /** Whether currently active */
  isActive: boolean;
}

// =============================================================================
// RPC Response Types
// =============================================================================

/**
 * Response from x3_kernel_getCanonicalBalance RPC
 */
export interface GetCanonicalBalanceResponse {
  balance: Balance;
}

/**
 * Response from x3_kernel_getAssetMetadata RPC
 */
export interface GetAssetMetadataResponse {
  symbol: string;
  decimals: number;
}

/**
 * Response from x3_kernel_isAuthorized RPC
 */
export interface IsAuthorizedResponse {
  authorized: boolean;
}

/**
 * Response from x3_kernel_getAuthorizedAccounts RPC
 */
export interface GetAuthorizedAccountsResponse {
  accounts: AccountId[];
}

/**
 * Response from x3_kernel_getAuthorities RPC
 */
export interface GetAuthoritiesResponse {
  authorities: AccountId[];
}

// =============================================================================
// Subscription Types
// =============================================================================

/**
 * Subscription callback for new blocks
 */
export type BlockSubscriptionCallback = (blockNumber: BlockNumber, blockHash: Hash) => void;

/**
 * Subscription callback for Comit events
 */
export type ComitEventCallback = (event: ComitEvent) => void;

/**
 * Union of all Comit-related events
 */
export type ComitEvent =
  | { type: 'submitted'; data: ComitSubmittedEvent }
  | { type: 'executionStarted'; data: ComitExecutionStartedEvent }
  | { type: 'executionCompleted'; data: ComitExecutionCompletedEvent }
  | { type: 'finalized'; data: ComitFinalizedEvent }
  | { type: 'failed'; data: ComitFailedEvent };

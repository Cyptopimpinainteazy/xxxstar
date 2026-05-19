/**
 * TypeScript interface types for Polkawallet plugin
 */

import type { ApiPromise, SubmittableResult } from '@polkadot/api';
import type { KeyringPair } from '@polkadot/keyring/types';
import type { Signer } from '@polkadot/types/types';

// =============================================================================
// Connection & Config
// =============================================================================

export interface X3ChainConfig {
  /** WebSocket endpoint (e.g. ws://localhost:9944) */
  endpoint: string;
  /** Network name (x3-mainnet, x3-testnet, x3-local) */
  network?: X3Network;
  /** Auto-reconnect on disconnect */
  autoConnect?: boolean;
  /** Custom signer (for Polkawallet mobile) */
  signer?: Signer;
  /** Request timeout in ms */
  timeout?: number;
}

export type X3Network = 'x3-mainnet' | 'x3-testnet' | 'x3-local';

export interface ConnectionState {
  connected: boolean;
  endpoint: string;
  chainName: string;
  genesisHash: string;
  runtimeVersion: number;
  latestBlock: number;
}

// =============================================================================
// Account Types
// =============================================================================

export interface X3Account {
  address: string;
  name?: string;
  isAuthorized: boolean;
  nonce: bigint;
  freeBalance: bigint;
  reservedBalance: bigint;
}

export interface X3Balance {
  assetId: number;
  symbol: string;
  decimals: number;
  free: bigint;
  reserved: bigint;
  frozen: bigint;
}

// =============================================================================
// Comit Types
// =============================================================================

export interface ComitParams {
  comitId: string;
  evmPayload?: Uint8Array | string;
  svmPayload?: Uint8Array | string;
  x3Payload?: Uint8Array | string;
  nonce?: bigint;
  fee: bigint;
  prepareRoot?: string;
}

export interface ComitResult {
  comitId: string;
  blockHash: string;
  blockNumber: number;
  success: boolean;
  gasUsed?: bigint;
  events: ComitEvent[];
}

export interface ComitEvent {
  type: string;
  data: Record<string, unknown>;
}

// =============================================================================
// Settlement Types
// =============================================================================

export interface CreateIntentParams {
  taker: string;
  assetA: AssetSpec;
  assetB: AssetSpec;
  secretHash: string;
  timeoutSeconds?: number;
}

export interface AssetSpec {
  chain: ExternalChainId;
  assetId: string;
  amount: bigint;
}

export type ExternalChainId =
  | 'X3'
  | 'Ethereum'
  | 'Solana'
  | 'Bitcoin'
  | 'Polkadot'
  | 'Kusama'
  | 'Cosmos'
  | 'Near'
  | 'Avalanche'
  | 'Bsc'
  | 'Arbitrum'
  | 'Optimism'
  | 'Base'
  | 'Polygon';

export type IntentState =
  | 'Created'
  | 'Locked'
  | 'ProofSubmitted'
  | 'Claimed'
  | 'Refunded'
  | 'Expired'
  | 'Disputed';

export interface SettlementIntentInfo {
  intentId: string;
  maker: string;
  taker: string;
  assetA: AssetSpec;
  assetB: AssetSpec;
  secretHash: string;
  timeout: number;
  state: IntentState;
  createdAt: number;
}

export interface LockEscrowParams {
  intentId: string;
  legIndex: number;
  chain: ExternalChainId;
  amount: bigint;
  escrowData: Uint8Array | string;
}

export interface BtcProofParams {
  intentId: string;
  btcTxid: string;
  vout: number;
  amountSats: bigint;
  merkleProof: string[];
  blockHeader: BtcBlockHeader;
}

export interface BtcBlockHeader {
  version: number;
  prevBlockHash: string;
  merkleRoot: string;
  timestamp: number;
  bits: number;
  nonce: number;
}

export interface BondParams {
  asset: string;
  amount: bigint;
  bondType: number;
}

// =============================================================================
// Atomic Trade Types
// =============================================================================

export interface CreateTradeBatchParams {
  legs: TradeLegInput[];
  slippageToleranceBps: number;
  deadline: number;
  nonce?: bigint;
}

export interface TradeLegInput {
  ammProtocol: AmmProtocol;
  vmType: VmType;
  assetIn: string;
  assetOut: string;
  amountIn: bigint;
  minAmountOut: bigint;
  routeData?: Uint8Array | string;
}

export type VmType = 'Evm' | 'Svm' | 'X3' | 'CrossVm';

export type AmmProtocol =
  | 'UniswapV2'
  | 'UniswapV3'
  | 'Raydium'
  | 'Orca'
  | 'Jupiter'
  | 'SushiSwap'
  | 'PancakeSwap'
  | 'Curve'
  | 'Balancer'
  | 'AtlasNative';

export type TradeBatchStatus =
  | 'Pending'
  | 'Executing'
  | 'Completed'
  | 'Failed'
  | 'Cancelled'
  | 'RolledBack';

export interface TradeBatchInfo {
  batchId: string;
  creator: string;
  legs: TradeLegInput[];
  slippageToleranceBps: number;
  deadline: number;
  status: TradeBatchStatus;
  createdAt: number;
}

export interface TradeResult {
  batchId: string;
  success: boolean;
  totalInput: bigint;
  totalOutput: bigint;
  gasUsed: bigint;
  legResults: TradeLegResult[];
}

export interface TradeLegResult {
  legIndex: number;
  success: boolean;
  amountOut: bigint;
  error?: string;
}

// =============================================================================
// X3 Domain Types
// =============================================================================

export interface RegisterDomainParams {
  domain: string;
}

export interface SetRecordsParams {
  domain: string;
  records: X3DnsRecord[];
}

export interface X3DnsRecord {
  ttl: number;
  data: X3RecordData;
}

export type X3RecordData =
  | { type: 'A'; value: [number, number, number, number] }
  | { type: 'Aaaa'; value: number[] }
  | { type: 'Cname'; value: string }
  | { type: 'Txt'; value: string };

export interface DomainInfo {
  domain: string;
  owner: string;
  records: X3DnsRecord[];
}

// =============================================================================
// X3 Verifier Types
// =============================================================================

export interface RegisterExecutorParams {
  stake: bigint;
}

export interface SubmitJobParams {
  bytecodeHash: string;
  inputHash: string;
  gasLimit: bigint;
  reward: bigint;
}

export interface SubmitReceiptParams {
  jobId: string;
  inputHash: string;
  outputHash: string;
  stateRootBefore: string;
  stateRootAfter: string;
  gasUsed: bigint;
  timestamp: number;
  outputData: Uint8Array | string;
  stateChanges: Array<[Uint8Array, Uint8Array]>;
  merkleProof: string[];
  signature: Uint8Array | string;
}

export type JobStatus = 'Pending' | 'Assigned' | 'Completed' | 'Failed' | 'Disputed';

export interface JobInfo {
  jobId: string;
  submitter: string;
  bytecodeHash: string;
  inputHash: string;
  gasLimit: bigint;
  reward: bigint;
  executor?: string;
  status: JobStatus;
  createdAt: number;
}

// =============================================================================
// Governance Types
// =============================================================================

export type VoteDirection = 'Aye' | 'Nay' | 'Abstain';

export type ConvictionLevel =
  | 'None'
  | 'Locked1x'
  | 'Locked2x'
  | 'Locked3x'
  | 'Locked4x'
  | 'Locked5x'
  | 'Locked6x';

export interface SubmitProposalParams {
  call: unknown; // RuntimeCall — encoded extrinsic
  title: string;
  description: string;
}

export interface VoteParams {
  proposalId: number;
  direction: VoteDirection;
  balance: bigint;
  conviction: ConvictionLevel;
}

export interface DelegateParams {
  target: string;
  conviction: ConvictionLevel;
}

export interface AIProposalParams {
  proposalType: AIProposalType;
  payload: Uint8Array | string;
  impactAssessment: ImpactAssessment;
  simulationRequirements: SimulationRequirements;
}

export type AIProposalType =
  | 'ParameterTuning'
  | 'FeeAdjustment'
  | 'SecurityPatch'
  | 'PerformanceOptimization'
  | 'ProtocolUpgrade';

export interface ImpactAssessment {
  riskScore: number;
  affectedPallets: string[];
  reversible: boolean;
  estimatedGas: bigint;
}

export interface SimulationRequirements {
  minSimulationBlocks: number;
  requiredCoveragePercent: number;
  maxStateChanges: number;
}

export type KillSwitchLevel =
  | 'Normal'
  | 'Cautious'
  | 'Restricted'
  | 'UpgradeFreeze'
  | 'EmergencyHalt';

// =============================================================================
// Treasury Types
// =============================================================================

export interface TreasuryProposalParams {
  beneficiary: string;
  amount: bigint;
  description: string;
}

export interface RecurringPaymentParams {
  beneficiary: string;
  amount: bigint;
  interval: number;
  totalPayments?: number;
  description: string;
}

export interface YieldStrategyParams {
  agent: string;
  maxAllocation: bigint;
  minExpectedReturn: number; // Percent (0-100)
  riskLevel: RiskLevel;
  description: string;
}

export type RiskLevel = 'Low' | 'Medium' | 'High' | 'Degen';

// =============================================================================
// SVM Types
// =============================================================================

export interface SvmCreateAccountParams {
  pubkey: Uint8Array;
  lamports: bigint;
  space: number;
  owner: Uint8Array;
}

export interface SvmDeployProgramParams {
  programId: Uint8Array;
  bytecode: Uint8Array;
  upgradeAuthority?: Uint8Array;
}

export interface SvmTransferParams {
  from: Uint8Array;
  to: Uint8Array;
  amount: bigint;
}

// =============================================================================
// Subscription Types
// =============================================================================

export type EventCallback<T = unknown> = (event: T) => void;
export type UnsubscribeFn = () => void;

export interface TxStatus {
  status: 'pending' | 'inBlock' | 'finalized' | 'error';
  blockHash?: string;
  blockNumber?: number;
  txHash?: string;
  error?: string;
  events?: ComitEvent[];
}

export type TxStatusCallback = (status: TxStatus) => void;

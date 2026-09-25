import { Signer } from '@polkadot/types/types';
import { KeyringPair } from '@polkadot/keyring/types';

/**
 * TypeScript interface types for Polkawallet plugin
 */

interface X3ChainConfig {
    /** WebSocket endpoint (e.g. ws://localhost:9944) */
    endpoint?: string;
    /** Network name (x3-mainnet, x3-testnet, x3-local) */
    network?: X3Network;
    /** Auto-reconnect on disconnect */
    autoConnect?: boolean;
    /** Custom signer (for Polkawallet mobile) */
    signer?: Signer;
    /** Request timeout in ms */
    timeout?: number;
    /** Enable reconnect attempts after an unexpected disconnect. */
    autoReconnect?: boolean;
    /** Maximum reconnect attempts before reporting a disconnect. */
    reconnectMaxAttempts?: number;
    /** Initial reconnect delay in milliseconds. */
    reconnectDelay?: number;
}
type X3Network = 'x3-mainnet' | 'x3-testnet' | 'x3-local';
interface ConnectionState {
    connected: boolean;
    endpoint: string;
    chainName: string;
    genesisHash: string;
    runtimeVersion: number;
    latestBlock: number;
}
interface X3Account {
    address: string;
    name?: string;
    isAuthorized: boolean;
    nonce: bigint;
    freeBalance: bigint;
    reservedBalance: bigint;
}
interface X3Balance {
    assetId: number;
    symbol: string;
    decimals: number;
    free: bigint;
    reserved: bigint;
    frozen: bigint;
}
interface ComitParams {
    comitId: string;
    evmPayload?: Uint8Array | string;
    svmPayload?: Uint8Array | string;
    x3Payload?: Uint8Array | string;
    nonce?: bigint;
    fee: bigint;
    prepareRoot?: string;
}
interface ComitResult {
    comitId: string;
    blockHash: string;
    blockNumber: number;
    success: boolean;
    gasUsed?: bigint;
    events: ComitEvent[];
}
interface ComitEvent {
    type: string;
    data: Record<string, unknown>;
}
interface CreateIntentParams {
    taker: string;
    assetA: AssetSpec;
    assetB: AssetSpec;
    secretHash: string;
    timeoutSeconds?: number;
}
interface AssetSpec {
    chain: ExternalChainId;
    assetId: string;
    amount: bigint;
}
type ExternalChainId = 'X3' | 'Ethereum' | 'Solana' | 'Bitcoin' | 'Polkadot' | 'Kusama' | 'Cosmos' | 'Near' | 'Avalanche' | 'Bsc' | 'Arbitrum' | 'Optimism' | 'Base' | 'Polygon';
type IntentState = 'Created' | 'Locked' | 'ProofSubmitted' | 'Claimed' | 'Refunded' | 'Expired' | 'Disputed';
interface SettlementIntentInfo {
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
interface LockEscrowParams {
    intentId: string;
    legIndex: number;
    chain: ExternalChainId;
    amount: bigint;
    escrowData: Uint8Array | string;
}
interface BtcProofParams {
    intentId: string;
    btcTxid: string;
    vout: number;
    amountSats: bigint;
    merkleProof: string[];
    blockHeader: BtcBlockHeader;
}
interface BtcBlockHeader {
    version: number;
    prevBlockHash: string;
    merkleRoot: string;
    timestamp: number;
    bits: number;
    nonce: number;
}
interface BondParams {
    asset: string;
    amount: bigint;
    bondType: number;
}
interface CreateTradeBatchParams {
    legs: TradeLegInput[];
    slippageToleranceBps: number;
    deadline: number;
    nonce?: bigint;
}
interface TradeLegInput {
    ammProtocol: AmmProtocol;
    vmType: VmType;
    assetIn: string;
    assetOut: string;
    amountIn: bigint;
    minAmountOut: bigint;
    routeData?: Uint8Array | string;
}
type VmType = 'Evm' | 'Svm' | 'X3' | 'CrossVm';
type AmmProtocol = 'UniswapV2' | 'UniswapV3' | 'Raydium' | 'Orca' | 'Jupiter' | 'SushiSwap' | 'PancakeSwap' | 'Curve' | 'Balancer' | 'AtlasNative';
type TradeBatchStatus = 'Pending' | 'Executing' | 'Completed' | 'Failed' | 'Cancelled' | 'RolledBack';
interface TradeBatchInfo {
    batchId: string;
    creator: string;
    legs: TradeLegInput[];
    slippageToleranceBps: number;
    deadline: number;
    status: TradeBatchStatus;
    createdAt: number;
}
interface TradeResult {
    batchId: string;
    success: boolean;
    totalInput: bigint;
    totalOutput: bigint;
    gasUsed: bigint;
    legResults: TradeLegResult[];
}
interface TradeLegResult {
    legIndex: number;
    success: boolean;
    amountOut: bigint;
    error?: string;
}
interface RegisterDomainParams {
    domain: string;
}
interface SetRecordsParams {
    domain: string;
    records: X3DnsRecord[];
}
interface X3DnsRecord {
    ttl: number;
    data: X3RecordData;
}
type X3RecordData = {
    type: 'A';
    value: [number, number, number, number];
} | {
    type: 'Aaaa';
    value: number[];
} | {
    type: 'Cname';
    value: string;
} | {
    type: 'Txt';
    value: string;
};
interface DomainInfo {
    domain: string;
    owner: string;
    records: X3DnsRecord[];
}
interface RegisterExecutorParams {
    stake: bigint;
}
interface SubmitJobParams {
    bytecodeHash: string;
    inputHash: string;
    gasLimit: bigint;
    reward: bigint;
}
interface SubmitReceiptParams {
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
type JobStatus = 'Pending' | 'Assigned' | 'Completed' | 'Failed' | 'Disputed';
interface JobInfo {
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
type VoteDirection = 'Aye' | 'Nay' | 'Abstain';
type ConvictionLevel = 'None' | 'Locked1x' | 'Locked2x' | 'Locked3x' | 'Locked4x' | 'Locked5x' | 'Locked6x';
interface SubmitProposalParams {
    call: unknown;
    title: string;
    description: string;
}
interface VoteParams {
    proposalId: number;
    direction: VoteDirection;
    balance: bigint;
    conviction: ConvictionLevel;
}
interface DelegateParams {
    target: string;
    conviction: ConvictionLevel;
}
interface AIProposalParams {
    proposalType: AIProposalType;
    payload: Uint8Array | string;
    impactAssessment: ImpactAssessment;
    simulationRequirements: SimulationRequirements;
}
type AIProposalType = 'ParameterTuning' | 'FeeAdjustment' | 'SecurityPatch' | 'PerformanceOptimization' | 'ProtocolUpgrade';
interface ImpactAssessment {
    riskScore: number;
    affectedPallets: string[];
    reversible: boolean;
    estimatedGas: bigint;
}
interface SimulationRequirements {
    minSimulationBlocks: number;
    requiredCoveragePercent: number;
    maxStateChanges: number;
}
type KillSwitchLevel = 'Normal' | 'Cautious' | 'Restricted' | 'UpgradeFreeze' | 'EmergencyHalt';
interface TreasuryProposalParams {
    beneficiary: string;
    amount: bigint;
    description: string;
}
interface RecurringPaymentParams {
    beneficiary: string;
    amount: bigint;
    interval: number;
    totalPayments?: number;
    description: string;
}
interface YieldStrategyParams {
    agent: string;
    maxAllocation: bigint;
    minExpectedReturn: number;
    riskLevel: RiskLevel;
    description: string;
}
type RiskLevel = 'Low' | 'Medium' | 'High' | 'Degen';
interface SvmCreateAccountParams {
    pubkey: Uint8Array;
    lamports: bigint;
    space: number;
    owner: Uint8Array;
}
interface SvmDeployProgramParams {
    programId: Uint8Array;
    bytecode: Uint8Array;
    upgradeAuthority?: Uint8Array;
}
interface SvmTransferParams {
    from: Uint8Array;
    to: Uint8Array;
    amount: bigint;
}
interface TxStatus {
    status: 'pending' | 'inBlock' | 'finalized' | 'error';
    blockHash?: string;
    blockNumber?: number;
    txHash?: string;
    error?: string;
    events?: ComitEvent[];
}
type TxStatusCallback = (status: TxStatus) => void;

/**
 * Transaction Helper — wraps extrinsic signing & submission
 * Works with both KeyringPair (node) and Polkawallet signer (mobile)
 */

type SignerAccount = string | KeyringPair;

export type { AIProposalParams as A, BtcProofParams as B, ConnectionState as C, DomainInfo as D, ExternalChainId as E, IntentState as F, AssetSpec as G, BtcBlockHeader as H, ImpactAssessment as I, JobInfo as J, KillSwitchLevel as K, LockEscrowParams as L, CreateTradeBatchParams as M, TradeResult as N, AmmProtocol as O, TradeBatchInfo as P, TradeBatchStatus as Q, RegisterExecutorParams as R, SignerAccount as S, TxStatusCallback as T, TradeLegInput as U, VoteParams as V, TradeLegResult as W, X3ChainConfig as X, YieldStrategyParams as Y, VmType as Z, X3Network as a, ComitParams as b, ComitResult as c, X3Balance as d, X3Account as e, ComitEvent as f, SubmitJobParams as g, SubmitReceiptParams as h, TreasuryProposalParams as i, RecurringPaymentParams as j, SvmCreateAccountParams as k, SvmDeployProgramParams as l, SvmTransferParams as m, SetRecordsParams as n, RegisterDomainParams as o, X3DnsRecord as p, X3RecordData as q, SubmitProposalParams as r, DelegateParams as s, AIProposalType as t, ConvictionLevel as u, SimulationRequirements as v, VoteDirection as w, CreateIntentParams as x, BondParams as y, SettlementIntentInfo as z };

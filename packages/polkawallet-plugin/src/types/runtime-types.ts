/**
 * X3 Chain x3chain Runtime Type Definitions
 *
 * Complete type registry for Polkawallet integration — mirrors all runtime
 * pallets: x3-kernel, x3-settlement-engine, x3-domain-registry,
 * x3-verifier, atomic-trade-engine, governance, treasury, svm-runtime.
 */

import type { RegistryTypes } from '@polkadot/types/types';

// =============================================================================
// Shared Enums & Structs
// =============================================================================

export const X3ChainCustomTypes: RegistryTypes = {
  // --- x3-kernel ---
  ComitFailureReason: {
    _enum: [
      'EvmExecutionFailed',
      'SvmExecutionFailed',
      'X3ExecutionFailed',
      'InvalidNonce',
      'InsufficientBalance',
      'PayloadDecodeError',
    ],
  },
  AssetMetadata: {
    symbol: 'Vec<u8>',
    decimals: 'u8',
    registered_at: 'BlockNumber',
  },
  AtlasId: {
    account: 'AccountId',
    nonce: 'u64',
    registered_block: 'BlockNumber',
  },

  // --- x3-settlement-engine ---
  SettlementIntent: {
    maker: 'AccountId',
    taker: 'AccountId',
    asset_a: 'AssetSpec',
    asset_b: 'AssetSpec',
    secret_hash: 'H256',
    timeout: 'u64',
    created_at: 'BlockNumber',
  },
  AssetSpec: {
    chain: 'ExternalChainId',
    asset_id: 'Vec<u8>',
    amount: 'u128',
  },
  ExternalChainId: {
    _enum: [
      'X3',
      'Ethereum',
      'Solana',
      'Bitcoin',
      'Polkadot',
      'Kusama',
      'Cosmos',
      'Near',
      'Avalanche',
      'Bsc',
      'Arbitrum',
      'Optimism',
      'Base',
      'Polygon',
    ],
  },
  IntentState: {
    _enum: [
      'Created',
      'Locked',
      'ProofSubmitted',
      'Claimed',
      'Refunded',
      'Expired',
      'Disputed',
    ],
  },
  EscrowLeg: {
    chain: 'ExternalChainId',
    amount: 'u128',
    escrow_address: 'Vec<u8>',
    locked_at: 'Option<BlockNumber>',
    proof: 'Option<SettlementProof>',
  },
  SettlementProof: {
    _enum: {
      SubstrateEvent: '(H256, u32)',
      EvmLog: '(H256, Vec<u8>)',
      SolanaSignature: 'Vec<u8>',
      BtcMerkleProof: '(H256, Vec<H256>)',
      CosmosIbc: 'Vec<u8>',
    },
  },
  BtcBlockHeader: {
    version: 'u32',
    prev_block_hash: 'H256',
    merkle_root: 'H256',
    timestamp: 'u32',
    bits: 'u32',
    nonce: 'u32',
  },
  BtcUtxoState: {
    txid: 'H256',
    vout: 'u32',
    amount_sats: 'u64',
    confirmed: 'bool',
    confirmations: 'u32',
  },
  BondRecord: {
    owner: 'AccountId',
    amount: 'Balance',
    bond_type: 'u8',
    locked: 'bool',
    created_at: 'BlockNumber',
    withdraw_requested_at: 'Option<BlockNumber>',
    slashed: 'bool',
  },
  FinalityConfig: {
    required_confirmations: 'u32',
    finality_delay_blocks: 'u32',
    max_reorg_depth: 'u32',
  },
  InvariantViolationType: {
    _enum: [
      'PartialExecution',
      'DoubleSpend',
      'BalanceMismatch',
      'TimeoutViolation',
      'CrossVmReentrancy',
    ],
  },

  // --- x3-domain-registry ---
  DomainInfo: {
    owner: 'AccountId',
    records: 'Vec<X3DnsRecord>',
  },
  X3DnsRecord: {
    ttl: 'u32',
    data: 'X3RecordData',
  },
  X3RecordData: {
    _enum: {
      A: '[u8; 4]',
      Aaaa: '[u8; 16]',
      Cname: 'Vec<u8>',
      Txt: 'Vec<u8>',
    },
  },

  // --- x3-verifier ---
  ExecutorRecord: {
    account: 'AccountId',
    stake: 'Balance',
    active: 'bool',
    jobs_completed: 'u64',
    jobs_failed: 'u64',
    registered_at: 'BlockNumber',
  },
  JobRecord: {
    submitter: 'AccountId',
    bytecode_hash: 'H256',
    input_hash: 'H256',
    gas_limit: 'u128',
    reward: 'Balance',
    executor: 'Option<AccountId>',
    status: 'JobStatus',
    created_at: 'BlockNumber',
  },
  JobStatus: {
    _enum: ['Pending', 'Assigned', 'Completed', 'Failed', 'Disputed'],
  },
  ExecutionReceiptData: {
    job_id: 'H256',
    executor: 'AccountId',
    input_hash: 'H256',
    output_hash: 'H256',
    state_root_before: 'H256',
    state_root_after: 'H256',
    gas_used: 'u128',
    timestamp: 'u64',
    output_data: 'Vec<u8>',
    state_changes: 'Vec<(Vec<u8>, Vec<u8>)>',
    merkle_proof: 'Vec<H256>',
    signature: 'Vec<u8>',
  },

  // --- atomic-trade-engine ---
  VmType: {
    _enum: ['Evm', 'Svm', 'X3', 'CrossVm'],
  },
  AmmProtocol: {
    _enum: [
      'UniswapV2',
      'UniswapV3',
      'Raydium',
      'Orca',
      'Jupiter',
      'SushiSwap',
      'PancakeSwap',
      'Curve',
      'Balancer',
      'AtlasNative',
    ],
  },
  TradeLegInput: {
    amm_protocol: 'AmmProtocol',
    vm_type: 'VmType',
    asset_in: 'H256',
    asset_out: 'H256',
    amount_in: 'u128',
    min_amount_out: 'u128',
    route_data: 'Vec<u8>',
  },
  TradeBatch: {
    creator: 'AccountId',
    legs: 'Vec<TradeLegInput>',
    slippage_tolerance_bps: 'u32',
    deadline: 'BlockNumber',
    nonce: 'u64',
    status: 'TradeBatchStatus',
    created_at: 'BlockNumber',
  },
  TradeBatchStatus: {
    _enum: [
      'Pending',
      'Executing',
      'Completed',
      'Failed',
      'Cancelled',
      'RolledBack',
    ],
  },
  StateCheckpoint: {
    state_root: 'H256',
    block_number: 'BlockNumber',
    leg_index: 'u32',
  },
  AmmAdapterConfig: {
    vm_type: 'VmType',
    router_address: 'Vec<u8>',
    factory_address: 'Vec<u8>',
    active: 'bool',
  },
  PricePoint: {
    price: 'u128',
    timestamp: 'u64',
    source: 'AmmProtocol',
  },
  TwapData: {
    cumulative_price: 'u256',
    last_update: 'u64',
    observation_count: 'u32',
  },

  // --- governance ---
  VoteDirection: {
    _enum: ['Aye', 'Nay', 'Abstain'],
  },
  Conviction: {
    _enum: [
      'None',
      'Locked1x',
      'Locked2x',
      'Locked3x',
      'Locked4x',
      'Locked5x',
      'Locked6x',
    ],
  },
  ProposalStatus: {
    _enum: ['Voting', 'Approved', 'Rejected', 'Enacted', 'Cancelled'],
  },
  AIProposalType: {
    _enum: [
      'ParameterTuning',
      'FeeAdjustment',
      'SecurityPatch',
      'PerformanceOptimization',
      'ProtocolUpgrade',
    ],
  },
  KillSwitchLevel: {
    _enum: [
      'Normal',
      'Cautious',
      'Restricted',
      'UpgradeFreeze',
      'EmergencyHalt',
    ],
  },
  ImpactAssessment: {
    risk_score: 'u8',
    affected_pallets: 'Vec<Vec<u8>>',
    reversible: 'bool',
    estimated_gas: 'u128',
  },
  SimulationRequirements: {
    min_simulation_blocks: 'u32',
    required_coverage_percent: 'u8',
    max_state_changes: 'u32',
  },

  // --- treasury ---
  SpendTrack: {
    _enum: ['SmallSpend', 'MediumSpend', 'BigSpend', 'CriticalSpend'],
  },
  RiskLevel: {
    _enum: ['Low', 'Medium', 'High', 'Degen'],
  },
  SpendingProposal: {
    proposer: 'AccountId',
    beneficiary: 'AccountId',
    amount: 'Balance',
    description: 'Vec<u8>',
    track: 'SpendTrack',
    status: 'ProposalStatus',
    created_at: 'BlockNumber',
  },
  RecurringPayment: {
    beneficiary: 'AccountId',
    amount: 'Balance',
    interval: 'BlockNumber',
    total_payments: 'Option<u32>',
    payments_made: 'u32',
    last_payment_at: 'BlockNumber',
    active: 'bool',
  },
  YieldStrategy: {
    agent: 'AccountId',
    max_allocation: 'Balance',
    min_expected_return: 'Percent',
    risk_level: 'RiskLevel',
    description: 'Vec<u8>',
    active: 'bool',
    total_deployed: 'Balance',
    total_returned: 'Balance',
  },

  // --- svm-runtime ---
  SvmAccountInfo: {
    lamports: 'u64',
    owner: '[u8; 32]',
    executable: 'bool',
    rent_epoch: 'u64',
    data_len: 'u32',
    created_at: 'BlockNumber',
  },
  SvmProgramInfo: {
    upgrade_authority: 'Option<[u8; 32]>',
    last_deploy_block: 'BlockNumber',
    bytecode_len: 'u32',
    is_frozen: 'bool',
  },
};

/**
 * Runtime RPC methods for x3-chain-specific calls
 */
export const X3ChainRpc = {
  x3: {
    getCanonicalBalance: {
      description: 'Get canonical balance for an account and asset',
      params: [
        { name: 'account', type: 'AccountId' },
        { name: 'asset_id', type: 'AssetId' },
        { name: 'at', type: 'Hash', isOptional: true },
      ],
      type: 'Balance',
    },
    getAssetMetadata: {
      description: 'Get asset metadata for a given asset_id',
      params: [
        { name: 'asset_id', type: 'AssetId' },
        { name: 'at', type: 'Hash', isOptional: true },
      ],
      type: 'Option<AssetMetadata>',
    },
    getNonce: {
      description: 'Get comit nonce for an account',
      params: [
        { name: 'account', type: 'AccountId' },
        { name: 'at', type: 'Hash', isOptional: true },
      ],
      type: 'u64',
    },
    isAuthorized: {
      description: 'Check if account is authorized',
      params: [
        { name: 'account', type: 'AccountId' },
        { name: 'at', type: 'Hash', isOptional: true },
      ],
      type: 'bool',
    },
    getAuthorities: {
      description: 'Get current authority set',
      params: [{ name: 'at', type: 'Hash', isOptional: true }],
      type: 'Vec<AccountId>',
    },
  },
  x3Settlement: {
    getIntent: {
      description: 'Get settlement intent details',
      params: [
        { name: 'intent_id', type: 'H256' },
        { name: 'at', type: 'Hash', isOptional: true },
      ],
      type: 'Option<SettlementIntent>',
    },
    getIntentState: {
      description: 'Get intent state',
      params: [
        { name: 'intent_id', type: 'H256' },
        { name: 'at', type: 'Hash', isOptional: true },
      ],
      type: 'IntentState',
    },
    getBond: {
      description: 'Get bond record',
      params: [
        { name: 'bond_id', type: 'H256' },
        { name: 'at', type: 'Hash', isOptional: true },
      ],
      type: 'Option<BondRecord>',
    },
    getBtcBestHeight: {
      description: 'Get best known BTC block height',
      params: [{ name: 'at', type: 'Hash', isOptional: true }],
      type: 'u64',
    },
  },
  atomicTrade: {
    getBatch: {
      description: 'Get trade batch by ID',
      params: [
        { name: 'batch_id', type: 'H256' },
        { name: 'at', type: 'Hash', isOptional: true },
      ],
      type: 'Option<TradeBatch>',
    },
    getTwap: {
      description: 'Get TWAP price for a pair',
      params: [
        { name: 'token_a', type: 'H256' },
        { name: 'token_b', type: 'H256' },
        { name: 'at', type: 'Hash', isOptional: true },
      ],
      type: 'Option<TwapData>',
    },
    getAmmAdapter: {
      description: 'Get AMM adapter config',
      params: [
        { name: 'protocol', type: 'AmmProtocol' },
        { name: 'at', type: 'Hash', isOptional: true },
      ],
      type: 'Option<AmmAdapterConfig>',
    },
  },
};

/**
 * Signed extensions for the x3chain runtime
 */
export const X3ChainSignedExtensions = {
  ChargeTransactionPayment: {
    extrinsic: { tip: 'Compact<Balance>' },
    payload: {},
  },
};

/**
 * X3 Chain x3chain — Custom type definitions for Polkawallet JS API
 *
 * These types map 1:1 to the X3 Chain runtime pallets:
 *   - x3-kernel (comit v2)
 *   - atomic-trade-engine
 *   - x3-verifier
 *   - x3-settlement-engine
 *   - x3-domain-registry
 *   - governance (+AI proposals, kill switch)
 *   - evolution-core
 *   - agent-accounts
 *   - agent-memory
 *   - treasury
 *   - swarm
 *   - svm-runtime
 */

export const x3chainTypes = {
  /* ─── X3 Kernel ─── */
  ComitId: 'H256',
  PrepareRoot: 'H256',
  CanonicalBalance: 'u128',
  AssetId: 'u32',
  AssetMetadata: {
    name: 'Vec<u8>',
    symbol: 'Vec<u8>',
    decimals: 'u8',
    total_supply: 'u128',
  },
  ComitPayload: {
    evm_payload: 'Option<Vec<u8>>',
    svm_payload: 'Option<Vec<u8>>',
    x3_payload: 'Option<Vec<u8>>',
    fee: 'u128',
    deadline: 'u64',
    metadata: 'Option<Vec<u8>>',
  },

  /* ─── Atomic Trade Engine ─── */
  TradeBatchId: 'u64',
  TradeStatus: {
    _enum: ['Pending', 'Executing', 'Settled', 'Cancelled', 'Failed'],
  },
  TradeBatch: {
    id: 'TradeBatchId',
    creator: 'AccountId',
    legs: 'Vec<TradeLeg>',
    status: 'TradeStatus',
    created_at: 'BlockNumber',
  },
  TradeLeg: {
    asset_in: 'AssetId',
    asset_out: 'AssetId',
    amount_in: 'u128',
    min_amount_out: 'u128',
    chain_target: 'ChainTarget',
  },
  ChainTarget: {
    _enum: ['Native', 'Evm', 'Svm', 'X3'],
  },
  PriceObservation: {
    asset_id: 'AssetId',
    price: 'u128',
    timestamp: 'u64',
    source: 'Vec<u8>',
  },

  /* ─── X3 Verifier ─── */
  ExecutorId: 'AccountId',
  X3JobId: 'u64',
  X3JobStatus: {
    _enum: ['Submitted', 'Assigned', 'Completed', 'Disputed', 'Slashed'],
  },
  X3Job: {
    id: 'X3JobId',
    submitter: 'AccountId',
    executor: 'Option<ExecutorId>',
    bytecode_hash: 'H256',
    gas_limit: 'u64',
    status: 'X3JobStatus',
  },
  X3Receipt: {
    job_id: 'X3JobId',
    executor: 'ExecutorId',
    gas_used: 'u64',
    return_data: 'Vec<u8>',
    state_root: 'H256',
    logs: 'Vec<Vec<u8>>',
  },
  VerificationReport: {
    is_valid: 'bool',
    gas_report: 'Option<Vec<u8>>',
    safety_rules: 'Option<Vec<u8>>',
  },

  /* ─── X3 Settlement Engine ─── */
  IntentId: 'u64',
  SettlementStatus: {
    _enum: ['Created', 'Escrowed', 'ProofSubmitted', 'Claimed', 'Refunded', 'Violated'],
  },
  Intent: {
    id: 'IntentId',
    creator: 'AccountId',
    chain_kind: 'ChainKind',
    amount: 'u128',
    asset: 'AssetId',
    counterparty: 'Option<AccountId>',
    proof_hash: 'Option<H256>',
    status: 'SettlementStatus',
    deadline: 'BlockNumber',
  },
  ChainKind: {
    _enum: {
      Evm: 'u64',
      Svm: 'Null',
      X3: 'Null',
      Bitcoin: 'Null',
    },
  },
  BondState: {
    depositor: 'AccountId',
    amount: 'u128',
    locked_until: 'Option<BlockNumber>',
  },

  /* ─── X3 Domain Registry ─── */
  DomainName: 'Vec<u8>',
  DomainRecord: {
    owner: 'AccountId',
    resolver: 'Option<AccountId>',
    ttl: 'u64',
    records: 'Vec<DnsRecord>',
    registered_at: 'BlockNumber',
    expires_at: 'BlockNumber',
  },
  DnsRecord: {
    record_type: 'DnsRecordType',
    value: 'Vec<u8>',
  },
  DnsRecordType: {
    _enum: ['A', 'AAAA', 'CNAME', 'TXT', 'X3ADDR', 'EVMADDR', 'SVMADDR'],
  },

  /* ─── Governance ─── */
  ProposalId: 'u64',
  ProposalStatus: {
    _enum: ['Active', 'Approved', 'Rejected', 'Executed', 'FastTracked', 'KillSwitched'],
  },
  GovernanceProposal: {
    id: 'ProposalId',
    proposer: 'AccountId',
    call_data: 'Vec<u8>',
    status: 'ProposalStatus',
    votes_for: 'u128',
    votes_against: 'u128',
    is_ai_proposal: 'bool',
  },
  Vote: {
    voter: 'AccountId',
    amount: 'u128',
    direction: 'VoteDirection',
    conviction: 'u8',
  },
  VoteDirection: {
    _enum: ['Aye', 'Nay', 'Abstain'],
  },

  /* ─── Evolution Core ─── */
  MutationId: 'u64',
  MutationStatus: {
    _enum: ['Proposed', 'Approved', 'Applied', 'Rejected', 'RolledBack'],
  },
  Mutation: {
    id: 'MutationId',
    proposer: 'AccountId',
    description: 'Vec<u8>',
    status: 'MutationStatus',
    metrics_before: 'Option<Vec<u8>>',
    metrics_after: 'Option<Vec<u8>>',
  },
  AiAgentRegistration: {
    agent_id: 'AccountId',
    name: 'Vec<u8>',
    capabilities: 'Vec<Vec<u8>>',
    operator: 'AccountId',
  },

  /* ─── Agent Accounts ─── */
  AgentId: 'AccountId',
  AgentPermissions: {
    can_trade: 'bool',
    can_submit_comit: 'bool',
    can_execute_x3: 'bool',
    can_govern: 'bool',
    max_spend_per_block: 'u128',
  },
  AgentReputation: {
    score: 'u64',
    total_operations: 'u64',
    successful_operations: 'u64',
    slashes: 'u32',
  },

  /* ─── Swarm ─── */
  SwarmId: 'u64',
  SwarmConfig: {
    id: 'SwarmId',
    coordinator: 'AccountId',
    agents: 'Vec<AccountId>',
    strategy: 'Vec<u8>',
  },
};

/**
 * Custom RPC methods exposed by X3 Chain node.
 */
export const x3chainRpc = {
  atlasKernel: {
    getCanonicalBalance: {
      description: 'Get the canonical balance of an account for a given asset',
      params: [
        { name: 'account', type: 'AccountId' },
        { name: 'asset_id', type: 'AssetId' },
        { name: 'at', type: 'Hash', isOptional: true },
      ],
      type: 'u128',
    },
    getAssetMetadata: {
      description: 'Get metadata for a registered asset',
      params: [
        { name: 'asset_id', type: 'AssetId' },
        { name: 'at', type: 'Hash', isOptional: true },
      ],
      type: 'AssetMetadata',
    },
    isAuthorized: {
      description: 'Check if an account is authorized',
      params: [
        { name: 'account', type: 'AccountId' },
        { name: 'at', type: 'Hash', isOptional: true },
      ],
      type: 'bool',
    },
    getAuthorities: {
      description: 'Get the list of authority accounts',
      params: [
        { name: 'at', type: 'Hash', isOptional: true },
      ],
      type: 'Vec<AccountId>',
    },
  },
  x3Domains: {
    getRecords: {
      description: 'Get DNS records for a .x3 domain',
      params: [
        { name: 'domain', type: 'Vec<u8>' },
        { name: 'at', type: 'Hash', isOptional: true },
      ],
      type: 'Option<Vec<DnsRecord>>',
    },
    getDomain: {
      description: 'Get full domain info for a .x3 domain',
      params: [
        { name: 'domain', type: 'Vec<u8>' },
        { name: 'at', type: 'Hash', isOptional: true },
      ],
      type: 'Option<DomainRecord>',
    },
    listDomains: {
      description: 'List all registered .x3 domains',
      params: [
        { name: 'at', type: 'Hash', isOptional: true },
      ],
      type: 'Vec<DomainName>',
    },
  },
  atomicTradeEngine: {
    getBatchStatus: {
      description: 'Get the status of a trade batch',
      params: [
        { name: 'batch_id', type: 'TradeBatchId' },
        { name: 'at', type: 'Hash', isOptional: true },
      ],
      type: 'Option<TradeStatus>',
    },
    simulateTrade: {
      description: 'Simulate execution of a trade batch without committing',
      params: [
        { name: 'batch', type: 'TradeBatch' },
        { name: 'at', type: 'Hash', isOptional: true },
      ],
      type: 'Vec<u8>',
    },
    getPrice: {
      description: 'Get latest price observation for an asset',
      params: [
        { name: 'asset_id', type: 'AssetId' },
        { name: 'at', type: 'Hash', isOptional: true },
      ],
      type: 'Option<PriceObservation>',
    },
  },
  x3Verifier: {
    getExecutorInfo: {
      description: 'Get info about a registered X3 executor',
      params: [
        { name: 'executor', type: 'AccountId' },
        { name: 'at', type: 'Hash', isOptional: true },
      ],
      type: 'Option<Vec<u8>>',
    },
    getJobStatus: {
      description: 'Get the status of an X3 job',
      params: [
        { name: 'job_id', type: 'X3JobId' },
        { name: 'at', type: 'Hash', isOptional: true },
      ],
      type: 'Option<X3JobStatus>',
    },
    getReceipt: {
      description: 'Get the execution receipt of an X3 job',
      params: [
        { name: 'job_id', type: 'X3JobId' },
        { name: 'at', type: 'Hash', isOptional: true },
      ],
      type: 'Option<X3Receipt>',
    },
  },
  evolutionCore: {
    getEvolutionStatus: {
      description: 'Get current evolution engine status',
      params: [
        { name: 'at', type: 'Hash', isOptional: true },
      ],
      type: 'Vec<u8>',
    },
    getBlockMetrics: {
      description: 'Get block-level metrics for evolution',
      params: [
        { name: 'block', type: 'BlockNumber' },
        { name: 'at', type: 'Hash', isOptional: true },
      ],
      type: 'Vec<u8>',
    },
  },
};

/**
 * X3 Chain chain constants.
 */
export const X3_CHAIN_ID = 650000;
export const X3_SS58_PREFIX = 42;
export const X3_TOKEN_SYMBOL = 'X3';
export const X3_TOKEN_DECIMALS = 18;
export const X3_BLOCK_TIME_MS = 6000;

/**
 * Default node endpoints.
 */
export const X3_ENDPOINTS = {
  local: 'ws://127.0.0.1:9944',
  testnet: 'wss://testnet.x3chain.io:9944',
  mainnet: 'wss://rpc.x3chain.io:9944',
};

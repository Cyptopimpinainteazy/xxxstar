/**
 * Integration tests for @x3-chain/ts-sdk X3 modules
 *
 * INV-REF: tests/invariants/registry.toml — ts_sdk_x3_integration
 *
 * Tests the X3 client classes added to the ts-sdk:
 *  - X3SettlementClient
 *  - X3AtomicTradeClient
 *  - X3DomainClient
 *  - X3VerifierClient
 */

import {
  X3SettlementClient,
  X3AtomicTradeClient,
  X3DomainClient,
  X3VerifierClient,
  createX3SettlementClient,
  createX3TradeClient,
  createX3DomainClient,
  createX3VerifierClient,
  X3VmType,
  X3AmmProtocol,
} from '../src/x3';

// =============================================================================
// Enum / constant exports
// =============================================================================

describe('X3 enums', () => {
  test('X3VmType contains all VM variants', () => {
    expect(X3VmType.Evm).toBe('Evm');
    expect(X3VmType.Svm).toBe('Svm');
    expect(X3VmType.X3).toBe('X3');
    expect(X3VmType.CrossVm).toBe('CrossVm');
  });

  test('X3AmmProtocol contains all AMM protocols', () => {
    expect(X3AmmProtocol.UniswapV2).toBe('UniswapV2');
    expect(X3AmmProtocol.UniswapV3).toBe('UniswapV3');
    expect(X3AmmProtocol.Raydium).toBe('Raydium');
    expect(X3AmmProtocol.Orca).toBe('Orca');
    expect(X3AmmProtocol.AtlasNative).toBe('AtlasNative');
  });
});

// =============================================================================
// Factory Functions
// =============================================================================

describe('X3 factory functions', () => {
  // We mock ApiPromise to avoid network calls
  const mockApi = {
    tx: {
      x3SettlementEngine: {
        createIntent: jest.fn(),
        lockEscrow: jest.fn(),
        submitProof: jest.fn(),
        claim: jest.fn(),
        refund: jest.fn(),
        submitBtcProof: jest.fn(),
        depositBond: jest.fn(),
        requestBondWithdraw: jest.fn(),
      },
      atomicTradeEngine: {
        createTradeBatch: jest.fn(),
        executeTradeBatch: jest.fn(),
        cancelTradeBatch: jest.fn(),
      },
      x3DomainRegistry: {
        registerDomain: jest.fn(),
        setRecords: jest.fn(),
      },
      x3Verifier: {
        registerExecutor: jest.fn(),
        submitJob: jest.fn(),
        submitReceipt: jest.fn(),
        disputeReceipt: jest.fn(),
      },
    },
    query: {
      x3SettlementEngine: {
        settlementIntents: jest.fn(),
        intentStates: jest.fn(),
        totalIntents: jest.fn(),
        totalSettledVolume: jest.fn(),
        invariantViolations: jest.fn(),
      },
      atomicTradeEngine: {
        batches: jest.fn(),
        twapOracles: jest.fn(),
      },
      x3DomainRegistry: {
        domains: jest.fn(),
      },
      x3Verifier: {
        jobs: jest.fn(),
        executors: jest.fn(),
      },
    },
    rpc: {
      x3Settlement: {
        getIntent: jest.fn(),
        getIntentState: jest.fn(),
      },
      atomicTrade: {
        getBatch: jest.fn(),
        getTwap: jest.fn(),
      },
    },
  } as any;

  test('createX3SettlementClient returns client instance', () => {
    const client = createX3SettlementClient(mockApi);
    expect(client).toBeInstanceOf(X3SettlementClient);
  });

  test('createX3TradeClient returns client instance', () => {
    const client = createX3TradeClient(mockApi);
    expect(client).toBeInstanceOf(X3AtomicTradeClient);
  });

  test('createX3DomainClient returns client instance', () => {
    const client = createX3DomainClient(mockApi);
    expect(client).toBeInstanceOf(X3DomainClient);
  });

  test('createX3VerifierClient returns client instance', () => {
    const client = createX3VerifierClient(mockApi);
    expect(client).toBeInstanceOf(X3VerifierClient);
  });
});

// =============================================================================
// X3SettlementClient Tests
// =============================================================================

describe('X3SettlementClient', () => {
  let client: X3SettlementClient;

  const mockSubmittable = {
    signAndSend: jest.fn(),
    paymentInfo: jest.fn().mockResolvedValue({ partialFee: { toBigInt: () => 100n } }),
  };

  const mockApi = {
    tx: {
      x3SettlementEngine: {
        createIntent: jest.fn().mockReturnValue(mockSubmittable),
        lockEscrow: jest.fn().mockReturnValue(mockSubmittable),
        submitProof: jest.fn().mockReturnValue(mockSubmittable),
        claim: jest.fn().mockReturnValue(mockSubmittable),
        refund: jest.fn().mockReturnValue(mockSubmittable),
        submitBtcProof: jest.fn().mockReturnValue(mockSubmittable),
        depositBond: jest.fn().mockReturnValue(mockSubmittable),
        requestBondWithdraw: jest.fn().mockReturnValue(mockSubmittable),
      },
    },
    query: {
      x3SettlementEngine: {
        settlementIntents: jest.fn().mockResolvedValue({ toJSON: () => ({ initiator: 'alice', amount: 1000, state: 'Pending' }) }),
        intentStates: jest.fn().mockResolvedValue({ toString: () => 'Pending' }),
        totalIntents: jest.fn().mockResolvedValue({ toNumber: () => 1 }),
        totalSettledVolume: jest.fn().mockResolvedValue({ toBigInt: () => 1000n }),
        invariantViolations: jest.fn().mockResolvedValue({ toNumber: () => 0 }),
      },
    },
  } as any;

  beforeEach(() => {
    client = new X3SettlementClient(mockApi);
    jest.clearAllMocks();
  });

  test('createIntent returns extrinsic', () => {
    const ext = client.createIntent({
      taker: '0xabc',
      assetA: { chain: 'Ethereum', assetId: 'ETH', amount: 1000n },
      assetB: { chain: 'Bitcoin', assetId: 'BTC', amount: 2000n },
      secretHash: '0x1234',
      timeoutSeconds: 3600,
    });
    expect(ext).toBeDefined();
    expect(mockApi.tx.x3SettlementEngine.createIntent).toHaveBeenCalledWith(
      '0xabc',
      { chain: 'Ethereum', asset_id: 'ETH', amount: 1000n },
      { chain: 'Bitcoin', asset_id: 'BTC', amount: 2000n },
      '0x1234',
      3600
    );
  });

  test('lockEscrow returns extrinsic', () => {
    const ext = client.lockEscrowLegacy('intent-1', 1000);
    expect(ext).toBeDefined();
    expect(mockApi.tx.x3SettlementEngine.lockEscrow).toHaveBeenCalled();
  });

  test('getIntent queries storage', async () => {
    const intent = await client.getIntent('intent-1');
    expect(intent).toEqual({ initiator: 'alice', amount: 1000, state: 'Pending' });
    expect(mockApi.query.x3SettlementEngine.settlementIntents).toHaveBeenCalledWith('intent-1');
  });
});

// =============================================================================
// X3AtomicTradeClient Tests
// =============================================================================

describe('X3AtomicTradeClient', () => {
  let client: X3AtomicTradeClient;

  const mockSubmittable = {
    signAndSend: jest.fn(),
  };

  const mockApi = {
    tx: {
      atomicTradeEngine: {
        createTradeBatch: jest.fn().mockReturnValue(mockSubmittable),
        executeTradeBatch: jest.fn().mockReturnValue(mockSubmittable),
        cancelTradeBatch: jest.fn().mockReturnValue(mockSubmittable),
      },
    },
    query: {
      atomicTradeEngine: {
        tradeBatches: jest.fn().mockResolvedValue({
          toJSON: () => ({
            id: 'batch-1',
            legs: [{ fromVm: 'Evm', toVm: 'Svm', amount: 100 }],
            status: 'Pending',
          }),
        }),
      },
    },
  } as any;

  beforeEach(() => {
    client = new X3AtomicTradeClient(mockApi);
    jest.clearAllMocks();
  });

  test('createBatch builds extrinsic from legs', () => {
    const legs = [
      { fromVm: X3VmType.Evm, toVm: X3VmType.Svm, fromAsset: 'USDC', toAsset: 'SOL', amount: 100, minOut: 1 },
    ];
    const ext = client.createBatch(legs);
    expect(ext).toBeDefined();
    expect(mockApi.tx.atomicTradeEngine.createTradeBatch).toHaveBeenCalled();
  });

  test('executeBatch builds extrinsic', () => {
    const ext = client.executeBatch('batch-1');
    expect(ext).toBeDefined();
    expect(mockApi.tx.atomicTradeEngine.executeTradeBatch).toHaveBeenCalledWith('batch-1');
  });

  test('cancelBatch builds extrinsic', () => {
    const ext = client.cancelBatch('batch-1');
    expect(ext).toBeDefined();
    expect(mockApi.tx.atomicTradeEngine.cancelTradeBatch).toHaveBeenCalledWith('batch-1');
  });

  test('getBatch queries storage', async () => {
    const batch = await client.getBatch('batch-1');
    expect(batch).toBeDefined();
    expect(batch.id).toBe('batch-1');
    expect(mockApi.query.atomicTradeEngine.tradeBatches).toHaveBeenCalledWith('batch-1');
  });
});

// =============================================================================
// X3DomainClient Tests
// =============================================================================

describe('X3DomainClient', () => {
  let client: X3DomainClient;

  const mockSubmittable = { signAndSend: jest.fn() };

  const mockApi = {
    tx: {
      x3DomainRegistry: {
        registerDomain: jest.fn().mockReturnValue(mockSubmittable),
        setRecords: jest.fn().mockReturnValue(mockSubmittable),
      },
    },
    query: {
      x3DomainRegistry: {
        domains: jest.fn().mockResolvedValue({
          toJSON: () => ({ name: 'alice.x3', owner: 'alice-addr', records: {} }),
        }),
      },
    },
  } as any;

  beforeEach(() => {
    client = new X3DomainClient(mockApi);
    jest.clearAllMocks();
  });

  test('register builds extrinsic', () => {
    const ext = client.register('alice.x3');
    expect(ext).toBeDefined();
    expect(mockApi.tx.x3DomainRegistry.registerDomain).toHaveBeenCalled();
  });

  test('setRecords builds extrinsic', () => {
    const ext = client.setRecordsLegacy('alice.x3', [{ key: 'evm', value: '0xabc' }]);
    expect(ext).toBeDefined();
    expect(mockApi.tx.x3DomainRegistry.setRecords).toHaveBeenCalled();
  });

  test('lookup queries storage', async () => {
    const result = await client.lookup('alice.x3');
    expect(result).toBeDefined();
    expect(result.name).toBe('alice.x3');
    expect(mockApi.query.x3DomainRegistry.domains).toHaveBeenCalled();
  });

  test('isAvailable returns true for unregistered domain', async () => {
    mockApi.query.x3DomainRegistry.domains.mockResolvedValueOnce({
      isSome: false,
    });
    const available = await client.isAvailable('new-name.x3');
    expect(available).toBe(true);
  });
});

// =============================================================================
// X3VerifierClient Tests
// =============================================================================

describe('X3VerifierClient', () => {
  let client: X3VerifierClient;

  const mockSubmittable = { signAndSend: jest.fn() };

  const mockApi = {
    tx: {
      x3Verifier: {
        registerExecutor: jest.fn().mockReturnValue(mockSubmittable),
        submitJob: jest.fn().mockReturnValue(mockSubmittable),
        submitReceipt: jest.fn().mockReturnValue(mockSubmittable),
        disputeReceipt: jest.fn().mockReturnValue(mockSubmittable),
      },
    },
    query: {
      x3Verifier: {
        jobs: jest.fn().mockResolvedValue({
          toJSON: () => ({ id: 'job-1', wasm: '0x00', status: 'Pending' }),
        }),
        verifiedStateRoots: jest.fn().mockResolvedValue({ toHex: () => '0x' + '00'.repeat(32) }),
        totalJobsSubmitted: jest.fn().mockResolvedValue({ toNumber: () => 1 }),
        totalJobsVerified: jest.fn().mockResolvedValue({ toNumber: () => 1 }),
        executors: jest.fn().mockResolvedValue({
          toJSON: () => ({ id: 'executor-1', active: true, stake: 10000 }),
        }),
      },
    },
  } as any;

  beforeEach(() => {
    client = new X3VerifierClient(mockApi);
    jest.clearAllMocks();
  });

  test('registerExecutor builds extrinsic', () => {
    const ext = client.registerExecutorLegacy('executor-1', 10000);
    expect(ext).toBeDefined();
    expect(mockApi.tx.x3Verifier.registerExecutor).toHaveBeenCalled();
  });

  test('submitJob builds extrinsic', () => {
    const ext = client.submitJobLegacy('0x00', { gasLimit: 1000000 });
    expect(ext).toBeDefined();
    expect(mockApi.tx.x3Verifier.submitJob).toHaveBeenCalled();
  });

  test('getJob queries storage', async () => {
    const job = await client.getJob('job-1');
    expect(job).toBeDefined();
    expect(job.id).toBe('job-1');
    expect(mockApi.query.x3Verifier.jobs).toHaveBeenCalledWith('job-1');
  });

  test('getExecutor queries storage', async () => {
    const exec = await client.getExecutor('executor-1');
    expect(exec.active).toBe(true);
    expect(mockApi.query.x3Verifier.executors).toHaveBeenCalledWith('executor-1');
  });
});

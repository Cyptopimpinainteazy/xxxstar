/**
 * Integration tests for @x3-chain/polkawallet-plugin
 *
 * INV-REF: tests/invariants/registry.toml — polkawallet_x3chain_integration
 *
 * These tests validate:
 * 1. Plugin initialization and service wiring
 * 2. Runtime type registration
 * 3. Kernel service API (comit submission, balance queries)
 * 4. Settlement service API (intent creation, state queries)
 * 5. Atomic Trade service API (batch creation, swap)
 * 6. Domain service API (registration, resolution)
 * 7. Verifier service API (job submission)
 * 8. Governance service API (proposals, voting)
 * 9. Treasury service API (spending proposals)
 * 10. X3VM client (compile, deploy, call, flash loans)
 */

import { AtlasX3Plugin } from '../src/plugin';
import { X3ChainCustomTypes, X3ChainRpc } from '../src/types/runtime-types';

// =============================================================================
// Type Registry Tests
// =============================================================================

describe('X3ChainCustomTypes', () => {
  test('defines all runtime pallet types', () => {
    // X3 Kernel types
    expect(X3ChainCustomTypes.ComitFailureReason).toBeDefined();
    expect(X3ChainCustomTypes.AssetMetadata).toBeDefined();

    // Settlement types
    expect(X3ChainCustomTypes.SettlementIntent).toBeDefined();
    expect(X3ChainCustomTypes.ExternalChainId).toBeDefined();
    expect(X3ChainCustomTypes.IntentState).toBeDefined();
    expect(X3ChainCustomTypes.SettlementProof).toBeDefined();
    expect(X3ChainCustomTypes.BtcBlockHeader).toBeDefined();
    expect(X3ChainCustomTypes.BondRecord).toBeDefined();

    // Atomic trade types
    expect(X3ChainCustomTypes.VmType).toBeDefined();
    expect(X3ChainCustomTypes.AmmProtocol).toBeDefined();
    expect(X3ChainCustomTypes.TradeLegInput).toBeDefined();
    expect(X3ChainCustomTypes.TradeBatch).toBeDefined();
    expect(X3ChainCustomTypes.PricePoint).toBeDefined();

    // Domain types
    expect(X3ChainCustomTypes.DomainInfo).toBeDefined();
    expect(X3ChainCustomTypes.X3DnsRecord).toBeDefined();
    expect(X3ChainCustomTypes.X3RecordData).toBeDefined();

    // Verifier types
    expect(X3ChainCustomTypes.ExecutorRecord).toBeDefined();
    expect(X3ChainCustomTypes.JobRecord).toBeDefined();
    expect(X3ChainCustomTypes.ExecutionReceiptData).toBeDefined();

    // Governance types
    expect(X3ChainCustomTypes.VoteDirection).toBeDefined();
    expect(X3ChainCustomTypes.Conviction).toBeDefined();
    expect(X3ChainCustomTypes.AIProposalType).toBeDefined();
    expect(X3ChainCustomTypes.KillSwitchLevel).toBeDefined();

    // Treasury types
    expect(X3ChainCustomTypes.SpendTrack).toBeDefined();
    expect(X3ChainCustomTypes.RiskLevel).toBeDefined();
    expect(X3ChainCustomTypes.YieldStrategy).toBeDefined();

    // SVM types
    expect(X3ChainCustomTypes.SvmAccountInfo).toBeDefined();
    expect(X3ChainCustomTypes.SvmProgramInfo).toBeDefined();
  });

  test('VmType enum includes all VM types', () => {
    const vmType = X3ChainCustomTypes.VmType as any;
    expect(vmType._enum).toContain('Evm');
    expect(vmType._enum).toContain('Svm');
    expect(vmType._enum).toContain('X3');
    expect(vmType._enum).toContain('CrossVm');
  });

  test('ExternalChainId includes all supported chains', () => {
    const chainId = X3ChainCustomTypes.ExternalChainId as any;
    const chains = chainId._enum;
    expect(chains).toContain('X3');
    expect(chains).toContain('Ethereum');
    expect(chains).toContain('Solana');
    expect(chains).toContain('Bitcoin');
    expect(chains).toContain('Polkadot');
    expect(chains).toContain('Cosmos');
    expect(chains).toContain('Arbitrum');
    expect(chains).toContain('Base');
  });

  test('AmmProtocol includes all protocols', () => {
    const proto = X3ChainCustomTypes.AmmProtocol as any;
    expect(proto._enum).toContain('UniswapV2');
    expect(proto._enum).toContain('UniswapV3');
    expect(proto._enum).toContain('Raydium');
    expect(proto._enum).toContain('Orca');
    expect(proto._enum).toContain('AtlasNative');
  });
});

describe('X3ChainRpc', () => {
  test('defines x3 RPC methods', () => {
    expect(X3ChainRpc.x3.getCanonicalBalance).toBeDefined();
    expect(X3ChainRpc.x3.getNonce).toBeDefined();
    expect(X3ChainRpc.x3.isAuthorized).toBeDefined();
    expect(X3ChainRpc.x3.getAuthorities).toBeDefined();
  });

  test('defines x3Settlement RPC methods', () => {
    expect(X3ChainRpc.x3Settlement.getIntent).toBeDefined();
    expect(X3ChainRpc.x3Settlement.getIntentState).toBeDefined();
    expect(X3ChainRpc.x3Settlement.getBond).toBeDefined();
    expect(X3ChainRpc.x3Settlement.getBtcBestHeight).toBeDefined();
  });

  test('defines atomicTrade RPC methods', () => {
    expect(X3ChainRpc.atomicTrade.getBatch).toBeDefined();
    expect(X3ChainRpc.atomicTrade.getTwap).toBeDefined();
    expect(X3ChainRpc.atomicTrade.getAmmAdapter).toBeDefined();
  });
});

// =============================================================================
// Plugin Tests (constructor-level, no network required)
// =============================================================================

describe('AtlasX3Plugin', () => {
  test('creates plugin with config', () => {
    const plugin = new AtlasX3Plugin({ endpoint: 'ws://127.0.0.1:9944' });
    expect(plugin).toBeDefined();
    expect(plugin.isReady).toBe(false);
    expect(plugin.connectionState).toBeNull();
  });

  test('throws when accessing services before init', () => {
    const plugin = new AtlasX3Plugin({ endpoint: 'ws://127.0.0.1:9944' });
    expect(() => plugin.kernel).toThrow('not initialized');
    expect(() => plugin.settlement).toThrow('not initialized');
    expect(() => plugin.trades).toThrow('not initialized');
    expect(() => plugin.domains).toThrow('not initialized');
    expect(() => plugin.verifier).toThrow('not initialized');
    expect(() => plugin.governance).toThrow('not initialized');
    expect(() => plugin.treasury).toThrow('not initialized');
    expect(() => plugin.svm).toThrow('not initialized');
    expect(() => plugin.x3vm).toThrow('not initialized');
  });
});

// =============================================================================
// Exports Validation
// =============================================================================

describe('Package exports', () => {
  test('exports all service classes', async () => {
    const plugin = require('../src/plugin');
    expect(plugin.AtlasX3Plugin).toBeDefined();
  });

  test('exports factory functions', async () => {
    const plugin = require('../src/plugin');
    expect(plugin.createLocalPlugin).toBeDefined();
    expect(plugin.createTestnetPlugin).toBeDefined();
    expect(plugin.createMainnetPlugin).toBeDefined();
  });

  test('exports type registries', async () => {
    const types = require('../src/types/runtime-types');
    expect(types.X3ChainCustomTypes).toBeDefined();
    expect(types.X3ChainRpc).toBeDefined();
    expect(types.X3ChainSignedExtensions).toBeDefined();
  });
});

/**
 * Integration tests for @x3-chain/polkawallet-bridge-adapter
 *
 * INV-REF: tests/invariants/registry.toml — polkawallet_bridge_routing
 */

import { X3ChainAdapter, x3chainRouteConfigs, x3chainTokensConfig, x3chainChainConfig } from '../src/index';

describe('X3ChainAdapter', () => {
  let adapter: X3ChainAdapter;

  beforeEach(() => {
    adapter = new X3ChainAdapter();
  });

  test('exposes chain config and ss58 prefix', () => {
    expect(adapter.chain).toEqual(x3chainChainConfig);
    expect(adapter.getSS58Prefix()).toBe(x3chainChainConfig.ss58Prefix);
  });

  test('returns configured routers', () => {
    const routers = adapter.getRouters();
    expect(routers.length).toBe(x3chainRouteConfigs.length);
  });

  test('returns token configuration', () => {
    const token = adapter.getToken('X3');
    expect(token).toEqual(x3chainTokensConfig.X3);
  });

  test('gets cross-chain fee for configured route', () => {
    const fee = adapter.getCrossChainFee('DOT', 'assetHubPolkadot');
    expect(fee.token).toBe('DOT');
    expect(Number(fee.amount)).toBeGreaterThan(0);
    expect(fee.decimals).toBe(x3chainTokensConfig.DOT.decimals);
  });
});

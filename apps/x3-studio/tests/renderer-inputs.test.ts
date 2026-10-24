import { describe, expect, it } from 'vitest';
import { restoreIconOrder } from '../src/components/Sidebar';
import { getRequestUrl } from '../src/components/panels/NetworkProfilerPanel';
import { syncedConfigs } from '../src/components/panels/ChainSyncPanel';

describe('chain sync response', () => {
  const config = { name: 'local', chainId: 42, rpcUrl: 'http://localhost:9944',
    explorerUrl: '', currency: 'X3', type: 'substrate' };

  it('reads the IPC response envelope', () => {
    expect(syncedConfigs({ configs: [config], error: null })).toEqual([config]);
  });

  it('rejects reported errors even when configs are present', () => {
    expect(() => syncedConfigs({ configs: [config], error: 'RPC unavailable' })).toThrow('RPC unavailable');
  });

  it('rejects malformed entries and unsafe chain identifiers', () => {
    for (const entry of [null, {}, { ...config, type: 'unknown' },
      { ...config, chainId: Number.MAX_SAFE_INTEGER + 1 }]) {
      expect(() => syncedConfigs({ configs: [entry], error: null })).toThrow('Invalid chain config entry');
    }
    expect(() => syncedConfigs({ configs: {}, error: null })).toThrow('Invalid chain config response');
  });
});

describe('saved sidebar order', () => {
  it('rejects stale and malformed IDs while keeping every registered panel', () => {
    const defaults = restoreIconOrder(null).map(entry => entry.id);
    const restored = restoreIconOrder(JSON.stringify([
      'tps-meter', 'unknown-panel', 42, null, 'multi-window', 'tps-meter',
    ])).map(entry => entry.id);
    expect(restored.slice(0, 2)).toEqual(['tps-meter', 'multi-window']);
    expect(new Set(restored)).toEqual(new Set(defaults));
    expect(restored).toHaveLength(defaults.length);
  });

  it('retains newly registered panels when loading an older order', () => {
    const restored = restoreIconOrder('["explorer"]');
    expect(restored[0].id).toBe('explorer');
    expect(restored.some(entry => entry.id === 'multi-window')).toBe(true);
    expect(restored).toHaveLength(restoreIconOrder(null).length);
  });

  it('falls back for invalid JSON and non-array data', () => {
    for (const saved of ['{', '{}', 'null', '"explorer"']) {
      expect(restoreIconOrder(saved)).toEqual(restoreIconOrder(null));
    }
  });
});

describe('network profiler fetch inputs', () => {
  it('preserves string inputs', () => {
    expect(getRequestUrl('/rpc?chain=x3')).toBe('/rpc?chain=x3');
  });

  it('records URL objects with their normalized address', () => {
    const url = new URL('/rpc?chain=x3', 'https://example.org');
    expect(getRequestUrl(url)).toBe(url.href);
  });

  it('records Request objects', () => {
    const request = new Request('https://example.org/rpc', { method: 'POST' });
    expect(getRequestUrl(request)).toBe(request.url);
  });
});

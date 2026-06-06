import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { HealthMonitor } from '../src/connector/health-monitor';

describe('HealthMonitor', () => {
  let originalFetch: any;

  beforeEach(() => {
    originalFetch = globalThis.fetch;
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
    vi.restoreAllMocks();
  });

  it('probes endpoints and marks healthy/unhealthy', async () => {
    globalThis.fetch = vi.fn(async (url: string, opts: any) => {
      if (url.includes('good')) {
        return { ok: true, headers: { get: () => 'application/json' }, json: async () => ({ result: '0x1' }) } as any;
      } else if (url.includes('auth')) {
        return { ok: false, status: 401, headers: { get: () => 'application/json' } } as any;
      }
      throw new Error('fetch failed');
    });

    const monitor = new HealthMonitor({ concurrency: 2, timeoutMs: 200, intervalMs: 1000 });
    const results = await monitor.probeEndpoints(['https://good.example', 'https://auth.example', 'https://down.example'], 2);

    expect(results.find(r => r.endpoint.includes('good'))!.healthy).toBe(true);
    expect(results.find(r => r.endpoint.includes('auth'))!.healthy).toBe(false);
    expect(results.find(r => r.endpoint.includes('down'))!.healthy).toBe(false);
  });
});

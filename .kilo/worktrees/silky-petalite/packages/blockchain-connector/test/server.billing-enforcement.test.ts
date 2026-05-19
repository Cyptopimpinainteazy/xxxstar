import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { mkdtemp, rm } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import { startServer } from '../src/server/index';

describe('Blockchain connector billing enforcement', () => {
  let tempDir = '';
  let server: ReturnType<typeof startServer> | undefined;
  let baseUrl = '';

  const closeServer = async () => {
    if (!server) {
      return;
    }
    await new Promise<void>((resolve, reject) => {
      server!.close((error) => (error ? reject(error) : resolve()));
    });
    server = undefined;
  };

  beforeEach(async () => {
    tempDir = await mkdtemp(join(tmpdir(), 'x3-connector-server-'));
    process.env.BLOCKCHAIN_CONNECTOR_BILLING_DB = join(tempDir, 'billing.json');
    process.env.BLOCKCHAIN_CONNECTOR_BOOTSTRAP_API_KEY = 'sk_x3_test_bootstrap';

    server = startServer({ port: 0 });
    const address = server.address();
    const port = typeof address === 'object' && address ? address.port : 0;
    baseUrl = `http://127.0.0.1:${port}`;
  });

  afterEach(async () => {
    delete process.env.BLOCKCHAIN_CONNECTOR_BILLING_DB;
    delete process.env.BLOCKCHAIN_CONNECTOR_BOOTSTRAP_API_KEY;
    await closeServer();
    if (tempDir) {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it('rejects API routes without API key', async () => {
    const response = await fetch(`${baseUrl}/api/v1/billing/status`);
    expect(response.status).toBe(401);
    const body = await response.json();
    expect(body.error).toContain('missing API key');
  });

  it('serves billing status and decrements request quota with valid API key', async () => {
    const response1 = await fetch(`${baseUrl}/api/v1/billing/status`, {
      headers: { 'X-Api-Key': 'sk_x3_test_bootstrap' },
    });
    expect(response1.status).toBe(200);
    const remaining1 = Number(response1.headers.get('X-RateLimit-Remaining'));
    expect(Number.isFinite(remaining1)).toBe(true);

    const response2 = await fetch(`${baseUrl}/api/v1/billing/status`, {
      headers: { 'X-Api-Key': 'sk_x3_test_bootstrap' },
    });
    expect(response2.status).toBe(200);
    const remaining2 = Number(response2.headers.get('X-RateLimit-Remaining'));
    expect(remaining2).toBe(remaining1 - 1);
  });

  it('accepts apiKey query parameter authentication', async () => {
    const response = await fetch(`${baseUrl}/api/v1/billing/status?apiKey=sk_x3_test_bootstrap`);
    expect(response.status).toBe(200);
    expect(response.headers.get('X-Billing-Tier')).toBe('free');
  });

  it('rejects invalid API key', async () => {
    const response = await fetch(`${baseUrl}/api/v1/billing/status`, {
      headers: { 'X-Api-Key': 'sk_x3_invalid' },
    });
    expect(response.status).toBe(401);
    const body = await response.json();
    expect(body.error).toContain('invalid API key');
  });

  it('returns 429 when request quota is exhausted', async () => {
    await closeServer();

    const quotaExhaustedBilling = {
      initialize: async () => {},
      getPlans: () => ({ free: { maxRequestsPerMonth: 0 } }),
      consumeRequest: async () => {
        throw new Error('QUOTA_EXCEEDED');
      },
    };

    server = startServer({ port: 0, billingRegistry: quotaExhaustedBilling as any });
    const address = server.address();
    const port = typeof address === 'object' && address ? address.port : 0;
    baseUrl = `http://127.0.0.1:${port}`;

    const response = await fetch(`${baseUrl}/api/v1/billing/status`, {
      headers: { 'X-Api-Key': 'sk_x3_any_key' },
    });
    expect(response.status).toBe(429);
    const body = await response.json();
    expect(body.error).toContain('monthly request quota exceeded');
  });

  it('exposes plans endpoint without requiring API key', async () => {
    const response = await fetch(`${baseUrl}/api/v1/billing/plans`);
    expect(response.status).toBe(200);
    const body = await response.json();
    expect(body.plans.free.maxRequestsPerMonth).toBeGreaterThan(0);
    expect(body.plans.enterprise.maxRequestsPerMonth).toBeGreaterThan(body.plans.free.maxRequestsPerMonth);
  });
});
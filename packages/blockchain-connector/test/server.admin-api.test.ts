import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { mkdtemp, rm } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import { startServer } from '../src/server/index';

describe('Blockchain connector admin API', () => {
  let tempDir = '';
  let server: ReturnType<typeof startServer> | undefined;
  let baseUrl = '';

  const closeServer = async () => {
    if (!server) return;
    await new Promise<void>((resolve, reject) => {
      server!.close((error) => (error ? reject(error) : resolve()));
    });
    server = undefined;
  };

  beforeEach(async () => {
    tempDir = await mkdtemp(join(tmpdir(), 'x3-connector-admin-'));
    process.env.BLOCKCHAIN_CONNECTOR_BILLING_DB = join(tempDir, 'billing.json');
    process.env.BLOCKCHAIN_CONNECTOR_BOOTSTRAP_API_KEY = 'sk_x3_bootstrap';
    process.env.BLOCKCHAIN_CONNECTOR_ADMIN_SECRET = 'admin_test_secret';

    server = startServer({ port: 0 });
    const address = server.address();
    const port = typeof address === 'object' && address ? address.port : 0;
    baseUrl = `http://127.0.0.1:${port}`;
  });

  afterEach(async () => {
    delete process.env.BLOCKCHAIN_CONNECTOR_BILLING_DB;
    delete process.env.BLOCKCHAIN_CONNECTOR_BOOTSTRAP_API_KEY;
    delete process.env.BLOCKCHAIN_CONNECTOR_ADMIN_SECRET;
    await closeServer();
    if (tempDir) await rm(tempDir, { recursive: true, force: true });
  });

  it('rejects admin routes with missing secret', async () => {
    const response = await fetch(`${baseUrl}/api/v1/admin/accounts`);
    expect(response.status).toBe(401);
    const body = await response.json();
    expect(body.error).toContain('invalid admin secret');
  });

  it('rejects admin routes with wrong secret', async () => {
    const response = await fetch(`${baseUrl}/api/v1/admin/accounts`, {
      headers: { 'X-Admin-Secret': 'wrong_secret' },
    });
    expect(response.status).toBe(401);
  });

  it('lists all accounts with valid admin secret', async () => {
    const response = await fetch(`${baseUrl}/api/v1/admin/accounts`, {
      headers: { 'X-Admin-Secret': 'admin_test_secret' },
    });
    expect(response.status).toBe(200);
    const body = await response.json();
    expect(Array.isArray(body.accounts)).toBe(true);
    expect(body.accounts.length).toBeGreaterThanOrEqual(1);
    expect(body.accounts[0].apiKey).toBe('sk_x3_bootstrap');
  });

  it('creates a new account with specified tier and returns fresh API key', async () => {
    const response = await fetch(`${baseUrl}/api/v1/admin/accounts`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Admin-Secret': 'admin_test_secret',
      },
      body: JSON.stringify({ tier: 'silver' }),
    });

    expect(response.status).toBe(201);
    const body = await response.json();
    expect(body.apiKey).toMatch(/^sk_x3_/);
    expect(body.account.tier).toBe('silver');
    expect(body.account.quotaRemaining.requests).toBe(1_000_000);
    expect(body.account.quotaRemaining.connectors).toBe(50);

    const listResponse = await fetch(`${baseUrl}/api/v1/admin/accounts`, {
      headers: { 'X-Admin-Secret': 'admin_test_secret' },
    });
    const listBody = await listResponse.json();
    expect(listBody.accounts).toHaveLength(2);
  });

  it('defaults to free tier when no tier is provided', async () => {
    const response = await fetch(`${baseUrl}/api/v1/admin/accounts`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Admin-Secret': 'admin_test_secret',
      },
      body: JSON.stringify({}),
    });

    expect(response.status).toBe(201);
    const body = await response.json();
    expect(body.account.tier).toBe('free');
    expect(body.account.quotaRemaining.connectors).toBe(2);
  });

  it('revokes an account and new API key calls are rejected', async () => {
    const createResponse = await fetch(`${baseUrl}/api/v1/admin/accounts`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Admin-Secret': 'admin_test_secret',
      },
      body: JSON.stringify({ tier: 'bronze' }),
    });
    const { apiKey } = await createResponse.json();

    const validBefore = await fetch(`${baseUrl}/api/v1/billing/status`, {
      headers: { 'X-Api-Key': apiKey },
    });
    expect(validBefore.status).toBe(200);

    const revokeResponse = await fetch(
      `${baseUrl}/api/v1/admin/accounts/${encodeURIComponent(apiKey)}`,
      {
        method: 'DELETE',
        headers: { 'X-Admin-Secret': 'admin_test_secret' },
      },
    );
    expect(revokeResponse.status).toBe(204);

    const invalidAfter = await fetch(`${baseUrl}/api/v1/billing/status`, {
      headers: { 'X-Api-Key': apiKey },
    });
    expect(invalidAfter.status).toBe(401);
  });

  it('returns 404 when revoking a non-existent account', async () => {
    const response = await fetch(`${baseUrl}/api/v1/admin/accounts/sk_x3_nonexistent`, {
      method: 'DELETE',
      headers: { 'X-Admin-Secret': 'admin_test_secret' },
    });
    expect(response.status).toBe(404);
    const body = await response.json();
    expect(body.error).toContain('account not found');
  });

  it('returns 409 when revoking the last account without force=true', async () => {
    const response = await fetch(
      `${baseUrl}/api/v1/admin/accounts/${encodeURIComponent('sk_x3_bootstrap')}`,
      {
        method: 'DELETE',
        headers: { 'X-Admin-Secret': 'admin_test_secret' },
      },
    );

    expect(response.status).toBe(409);
    const body = await response.json();
    expect(body.error).toContain('cannot revoke the last account');

    const stillValid = await fetch(`${baseUrl}/api/v1/billing/status`, {
      headers: { 'X-Api-Key': 'sk_x3_bootstrap' },
    });
    expect(stillValid.status).toBe(200);
  });

  it('allows revoking the last account when force=true is provided', async () => {
    const response = await fetch(
      `${baseUrl}/api/v1/admin/accounts/${encodeURIComponent('sk_x3_bootstrap')}?force=true`,
      {
        method: 'DELETE',
        headers: { 'X-Admin-Secret': 'admin_test_secret' },
      },
    );

    expect(response.status).toBe(204);

    const invalidAfter = await fetch(`${baseUrl}/api/v1/billing/status`, {
      headers: { 'X-Api-Key': 'sk_x3_bootstrap' },
    });
    expect(invalidAfter.status).toBe(401);
  });

  it('returns 503 when admin secret env var is not configured', async () => {
    delete process.env.BLOCKCHAIN_CONNECTOR_ADMIN_SECRET;
    await closeServer();

    server = startServer({ port: 0 });
    const address = server.address();
    const port = typeof address === 'object' && address ? address.port : 0;
    baseUrl = `http://127.0.0.1:${port}`;

    const response = await fetch(`${baseUrl}/api/v1/admin/accounts`, {
      headers: { 'X-Admin-Secret': 'any_secret' },
    });
    expect(response.status).toBe(503);
    const body = await response.json();
    expect(body.error).toContain('admin API not configured');
  });

  it('upgrades account tier and recalculates quotas', async () => {
    const createResponse = await fetch(`${baseUrl}/api/v1/admin/accounts`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Admin-Secret': 'admin_test_secret',
      },
      body: JSON.stringify({ tier: 'free' }),
    });
    const { apiKey } = await createResponse.json();

    const upgradeResponse = await fetch(
      `${baseUrl}/api/v1/admin/accounts/${encodeURIComponent(apiKey)}/tier`,
      {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Admin-Secret': 'admin_test_secret',
        },
        body: JSON.stringify({ tier: 'silver' }),
      },
    );

    expect(upgradeResponse.status).toBe(200);
    const body = await upgradeResponse.json();
    expect(body.account.tier).toBe('silver');
    expect(body.account.quotaRemaining.requests).toBe(1_000_000);
    expect(body.account.quotaRemaining.connectors).toBe(50);
  });

  it('downgrades account tier and recalculates quotas', async () => {
    const createResponse = await fetch(`${baseUrl}/api/v1/admin/accounts`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Admin-Secret': 'admin_test_secret',
      },
      body: JSON.stringify({ tier: 'gold' }),
    });
    const { apiKey } = await createResponse.json();

    const downgradeResponse = await fetch(
      `${baseUrl}/api/v1/admin/accounts/${encodeURIComponent(apiKey)}/tier`,
      {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Admin-Secret': 'admin_test_secret',
        },
        body: JSON.stringify({ tier: 'bronze' }),
      },
    );

    expect(downgradeResponse.status).toBe(200);
    const body = await downgradeResponse.json();
    expect(body.account.tier).toBe('bronze');
    expect(body.account.quotaRemaining.requests).toBe(100_000);
    expect(body.account.quotaRemaining.connectors).toBe(10);
  });

  it('returns 404 when changing tier for non-existent account', async () => {
    const response = await fetch(
      `${baseUrl}/api/v1/admin/accounts/sk_x3_nonexistent/tier`,
      {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Admin-Secret': 'admin_test_secret',
        },
        body: JSON.stringify({ tier: 'silver' }),
      },
    );

    expect(response.status).toBe(404);
    const body = await response.json();
    expect(body.error).toContain('account not found');
  });

  it('defaults to free tier when invalid tier is provided', async () => {
    const createResponse = await fetch(`${baseUrl}/api/v1/admin/accounts`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Admin-Secret': 'admin_test_secret',
      },
      body: JSON.stringify({ tier: 'bronze' }),
    });
    const { apiKey } = await createResponse.json();

    const changeResponse = await fetch(
      `${baseUrl}/api/v1/admin/accounts/${encodeURIComponent(apiKey)}/tier`,
      {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Admin-Secret': 'admin_test_secret',
        },
        body: JSON.stringify({ tier: 'invalid_tier' }),
      },
    );

    expect(changeResponse.status).toBe(200);
    const body = await changeResponse.json();
    expect(body.account.tier).toBe('free');
    expect(body.account.quotaRemaining.connectors).toBe(2);
  });

  it('resets account usage and quotas to current tier defaults', async () => {
    const createResponse = await fetch(`${baseUrl}/api/v1/admin/accounts`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Admin-Secret': 'admin_test_secret',
      },
      body: JSON.stringify({ tier: 'bronze' }),
    });
    const { apiKey } = await createResponse.json();

    await fetch(`${baseUrl}/api/v1/billing/status`, {
      headers: { 'X-Api-Key': apiKey },
    });
    const consumed = await fetch(`${baseUrl}/api/v1/billing/status`, {
      headers: { 'X-Api-Key': apiKey },
    });
    const remainingAfterConsume = Number(consumed.headers.get('X-RateLimit-Remaining'));
    expect(remainingAfterConsume).toBeLessThan(100_000);

    const resetResponse = await fetch(
      `${baseUrl}/api/v1/admin/accounts/${encodeURIComponent(apiKey)}/reset`,
      {
        method: 'POST',
        headers: {
          'X-Admin-Secret': 'admin_test_secret',
        },
      },
    );
    expect(resetResponse.status).toBe(200);
    const resetBody = await resetResponse.json();
    expect(resetBody.account.usage.requestsThisMonth).toBe(0);
    expect(resetBody.account.quotaRemaining.requests).toBe(100_000);

    const statusAfterReset = await fetch(`${baseUrl}/api/v1/billing/status`, {
      headers: { 'X-Api-Key': apiKey },
    });
    expect(statusAfterReset.status).toBe(200);
    const remainingAfterReset = Number(statusAfterReset.headers.get('X-RateLimit-Remaining'));
    expect(remainingAfterReset).toBe(99_999);
  });

  it('returns 404 when resetting usage for non-existent account', async () => {
    const response = await fetch(
      `${baseUrl}/api/v1/admin/accounts/sk_x3_nonexistent/reset`,
      {
        method: 'POST',
        headers: {
          'X-Admin-Secret': 'admin_test_secret',
        },
      },
    );

    expect(response.status).toBe(404);
    const body = await response.json();
    expect(body.error).toContain('account not found');
  });

  it('rotates an API key and returns the new key', async () => {
    const createResponse = await fetch(`${baseUrl}/api/v1/admin/accounts`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', 'X-Admin-Secret': 'admin_test_secret' },
      body: JSON.stringify({ tier: 'bronze' }),
    });
    const { apiKey: oldKey } = await createResponse.json();

    const rotateResponse = await fetch(
      `${baseUrl}/api/v1/admin/accounts/${encodeURIComponent(oldKey)}/rotate`,
      { method: 'POST', headers: { 'X-Admin-Secret': 'admin_test_secret' } },
    );
    expect(rotateResponse.status).toBe(200);
    const rotateBody = await rotateResponse.json();
    expect(rotateBody.newApiKey).toBeTruthy();
    expect(rotateBody.newApiKey).not.toBe(oldKey);
    expect(rotateBody.newApiKey).toMatch(/^sk_x3_/);
    expect(rotateBody.account.tier).toBe('bronze');
  });

  it('old API key is rejected after rotation', async () => {
    const createResponse = await fetch(`${baseUrl}/api/v1/admin/accounts`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', 'X-Admin-Secret': 'admin_test_secret' },
      body: JSON.stringify({ tier: 'free' }),
    });
    const { apiKey: oldKey } = await createResponse.json();

    const rotateResponse = await fetch(
      `${baseUrl}/api/v1/admin/accounts/${encodeURIComponent(oldKey)}/rotate`,
      { method: 'POST', headers: { 'X-Admin-Secret': 'admin_test_secret' } },
    );
    const { newApiKey } = await rotateResponse.json();

    // old key should now be invalid
    const oldKeyStatus = await fetch(`${baseUrl}/api/v1/billing/status`, {
      headers: { 'X-Api-Key': oldKey },
    });
    expect(oldKeyStatus.status).toBe(401);

    // new key should work
    const newKeyStatus = await fetch(`${baseUrl}/api/v1/billing/status`, {
      headers: { 'X-Api-Key': newApiKey },
    });
    expect(newKeyStatus.status).toBe(200);
  });

  it('returns 404 when rotating a non-existent API key', async () => {
    const response = await fetch(
      `${baseUrl}/api/v1/admin/accounts/sk_x3_nonexistent/rotate`,
      { method: 'POST', headers: { 'X-Admin-Secret': 'admin_test_secret' } },
    );
    expect(response.status).toBe(404);
    const body = await response.json();
    expect(body.error).toContain('account not found');
  });

  it('accepts adminSecret query parameter for rotate route', async () => {
    const createResponse = await fetch(
      `${baseUrl}/api/v1/admin/accounts?adminSecret=admin_test_secret`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ tier: 'silver' }),
      },
    );
    expect(createResponse.status).toBe(201);
    const { apiKey: oldKey } = await createResponse.json();

    const rotateResponse = await fetch(
      `${baseUrl}/api/v1/admin/accounts/${encodeURIComponent(oldKey)}/rotate?adminSecret=admin_test_secret`,
      { method: 'POST' },
    );
    expect(rotateResponse.status).toBe(200);
    const rotateBody = await rotateResponse.json();
    expect(rotateBody.newApiKey).toBeTruthy();
    expect(rotateBody.newApiKey).not.toBe(oldKey);
    expect(rotateBody.account.tier).toBe('silver');
  });

  it('accepts adminSecret query parameter for tier-change route', async () => {
    const createResponse = await fetch(
      `${baseUrl}/api/v1/admin/accounts?adminSecret=admin_test_secret`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ tier: 'free' }),
      },
    );
    expect(createResponse.status).toBe(201);
    const { apiKey } = await createResponse.json();

    const tierResponse = await fetch(
      `${baseUrl}/api/v1/admin/accounts/${encodeURIComponent(apiKey)}/tier?adminSecret=admin_test_secret`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ tier: 'silver' }),
      },
    );
    expect(tierResponse.status).toBe(200);
    const tierBody = await tierResponse.json();
    expect(tierBody.account.tier).toBe('silver');
    expect(tierBody.account.quotaRemaining.connectors).toBe(50);
  });

  it('accepts adminSecret query parameter for reset route', async () => {
    const createResponse = await fetch(
      `${baseUrl}/api/v1/admin/accounts?adminSecret=admin_test_secret`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ tier: 'bronze' }),
      },
    );
    expect(createResponse.status).toBe(201);
    const { apiKey } = await createResponse.json();

    await fetch(`${baseUrl}/api/v1/billing/status`, {
      headers: { 'X-Api-Key': apiKey },
    });

    const resetResponse = await fetch(
      `${baseUrl}/api/v1/admin/accounts/${encodeURIComponent(apiKey)}/reset?adminSecret=admin_test_secret`,
      { method: 'POST' },
    );
    expect(resetResponse.status).toBe(200);
    const resetBody = await resetResponse.json();
    expect(resetBody.account.usage.requestsThisMonth).toBe(0);
  });

  it('accepts adminSecret query parameter for revoke route', async () => {
    const createResponse = await fetch(
      `${baseUrl}/api/v1/admin/accounts?adminSecret=admin_test_secret`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ tier: 'bronze' }),
      },
    );
    expect(createResponse.status).toBe(201);
    const { apiKey } = await createResponse.json();

    const revokeResponse = await fetch(
      `${baseUrl}/api/v1/admin/accounts/${encodeURIComponent(apiKey)}?adminSecret=admin_test_secret`,
      { method: 'DELETE' },
    );
    expect(revokeResponse.status).toBe(204);

    const invalidAfter = await fetch(`${baseUrl}/api/v1/billing/status`, {
      headers: { 'X-Api-Key': apiKey },
    });
    expect(invalidAfter.status).toBe(401);
  });

  it('accepts adminSecret as a query parameter', async () => {
    const response = await fetch(
      `${baseUrl}/api/v1/admin/accounts?adminSecret=admin_test_secret`,
    );
    expect(response.status).toBe(200);
    const body = await response.json();
    expect(Array.isArray(body.accounts)).toBe(true);
  });
});

import { mkdtemp, readFile, rm } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { afterEach, describe, expect, it } from 'vitest';
import { BillingRegistry } from './billing';

const tempPaths: string[] = [];

async function createRegistry() {
  const dir = await mkdtemp(join(tmpdir(), 'x3-billing-'));
  tempPaths.push(dir);
  const dbPath = join(dir, 'billing.json');
  const registry = new BillingRegistry(dbPath);
  await registry.initialize();
  return { registry, dbPath };
}

afterEach(async () => {
  await Promise.all(
    tempPaths.splice(0, tempPaths.length).map((path) => rm(path, { recursive: true, force: true })),
  );
});

describe('BillingRegistry', () => {
  it('bootstraps at least one account and persists to disk', async () => {
    const { registry, dbPath } = await createRegistry();

    const raw = await readFile(dbPath, 'utf8');
    const parsed = JSON.parse(raw);
    expect(parsed.accounts.length).toBeGreaterThan(0);

    const account = parsed.accounts[0];
    const status = registry.getAccountStatus(account.apiKey);
    expect(status).not.toBeNull();
    expect(status?.quotaRemaining.requests).toBeGreaterThan(0);
  });

  it('decrements request quota and rejects after exhaustion', async () => {
    const { registry, dbPath } = await createRegistry();
    const raw = await readFile(dbPath, 'utf8');
    const parsed = JSON.parse(raw);
    const apiKey = parsed.accounts[0].apiKey as string;

    const status = registry.getAccountStatus(apiKey);

    expect(status).not.toBeNull();
    if (!status) {
      return;
    }

    status.quotaRemaining.requests = 1;
    const first = await registry.consumeRequest(apiKey);
    expect(first.remaining).toBe(0);

    await expect(registry.consumeRequest(apiKey)).rejects.toThrow('QUOTA_EXCEEDED');
  });

  it('enforces concurrent websocket quotas and releases capacity', async () => {
    const { registry, dbPath } = await createRegistry();
    const raw = await readFile(dbPath, 'utf8');
    const parsed = JSON.parse(raw);
    const apiKey = parsed.accounts[0].apiKey as string;

    const sessions = await Promise.all(
      Array.from({ length: 5 }, () => registry.acquireWsConnection(apiKey)),
    );

    expect(sessions[0].remaining).toBe(4);
    await expect(registry.acquireWsConnection(apiKey)).rejects.toThrow('WS_QUOTA_EXCEEDED');

    await registry.releaseWsConnection(apiKey, sessions[0].connectionId, Date.now() - 61_000);
    const reacquired = await registry.acquireWsConnection(apiKey);
    expect(reacquired.connectionId).toContain('ws_');

    const status = registry.getAccountStatus(apiKey);
    expect(status?.usage.wsMinutesThisMonth).toBeGreaterThan(0);
  });

  it('supports connector slot quotas and release', async () => {
    const { registry, dbPath } = await createRegistry();
    const raw = await readFile(dbPath, 'utf8');
    const parsed = JSON.parse(raw);
    const apiKey = parsed.accounts[0].apiKey as string;

    const first = await registry.acquireConnectorSlot(apiKey);
    expect(first.account.apiKey).toBe(apiKey);
    expect(first.remaining).toBe(1);

    const second = await registry.acquireConnectorSlot(apiKey);
    expect(second.remaining).toBe(0);

    await expect(registry.acquireConnectorSlot(apiKey)).rejects.toThrow('CONNECTOR_QUOTA_EXCEEDED');

    await registry.releaseConnectorSlot(apiKey);
    await expect(registry.acquireConnectorSlot(apiKey)).resolves.toMatchObject({
      account: expect.objectContaining({ apiKey }),
      remaining: 0,
    });
  });

  it('rotates API keys while preserving account state and invalidating old key', async () => {
    const { registry, dbPath } = await createRegistry();
    const raw = await readFile(dbPath, 'utf8');
    const parsed = JSON.parse(raw);
    const oldKey = parsed.accounts[0].apiKey as string;

    await registry.consumeRequest(oldKey);
    await registry.acquireConnectorSlot(oldKey);

    const beforeRotate = registry.getAccountStatus(oldKey);
    expect(beforeRotate).not.toBeNull();

    const rotated = await registry.rotateApiKey(oldKey);
    expect(rotated).not.toBeNull();
    if (!rotated || !beforeRotate) {
      return;
    }

    expect(rotated.newApiKey).toMatch(/^sk_x3_/);
    expect(rotated.newApiKey).not.toBe(oldKey);
    expect(rotated.account.id).toBe(beforeRotate.id);
    expect(rotated.account.tier).toBe(beforeRotate.tier);
    expect(rotated.account.usage.requestsThisMonth).toBe(beforeRotate.usage.requestsThisMonth);
    expect(rotated.account.usage.connectorsActive).toBe(beforeRotate.usage.connectorsActive);

    expect(registry.getAccountStatus(oldKey)).toBeNull();
    await expect(registry.consumeRequest(oldKey)).rejects.toThrow('INVALID_API_KEY');

    const newKeyStatus = registry.getAccountStatus(rotated.newApiKey);
    expect(newKeyStatus).not.toBeNull();
    expect(newKeyStatus?.id).toBe(beforeRotate.id);
  });

  it('returns null when rotating a non-existent API key', async () => {
    const { registry } = await createRegistry();
    const rotated = await registry.rotateApiKey('sk_x3_nonexistent');
    expect(rotated).toBeNull();
  });
});
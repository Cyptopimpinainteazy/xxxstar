import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { startServer } from '../src/server/index';
import { CHAIN_REGISTRY } from '../src/chains/registry';

describe('Blockchain connector connector-route billing enforcement', () => {
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

  beforeEach(() => {
    vi.restoreAllMocks();
  });

  afterEach(async () => {
    await closeServer();
  });

  it('injects authenticated API key into connector creation and filters list by API key', async () => {
    const createConnector = vi.fn(async (options: any) => ({
      id: 'conn_a',
      options,
      chain: {
        id: options.chain,
        name: 'Stub',
        family: 'evm',
        network: options.network,
        nativeCurrency: { name: 'Stub', symbol: 'STB', decimals: 18 },
        chainId: 1,
        defaultRpcUrls: ['http://stub'],
        defaultWsUrls: [],
        available: true,
        avgBlockTimeSeconds: 12,
        consensus: 'pow',
        signatureAlgorithm: 'secp256k1',
        hashAlgorithm: 'keccak256',
        gpuAccelerated: false,
      },
      status: 'connected',
      metrics: {
        blockHeight: 1,
        tps: 0,
        peerCount: 0,
        latencyMs: 0,
        totalRequests: 0,
        totalErrors: 0,
        uptimeSeconds: 0,
        finalityLag: 0,
      },
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    }));

    const removeConnector = vi.fn(async () => undefined);

    const connectorManager = {
      createConnector,
      listConnectors: vi.fn(() => ([
        { id: 'conn_a', options: { auth: { apiKey: 'sk_key_a' } } },
        { id: 'conn_b', options: { auth: { apiKey: 'sk_key_b' } } },
      ])),
      getConnector: vi.fn((id: string) => {
        if (id === 'conn_b') {
          return { id: 'conn_b', options: { auth: { apiKey: 'sk_key_b' } } };
        }
        return undefined;
      }),
      removeConnector,
    } as any;

    const billingRegistry = {
      initialize: async () => undefined,
      getPlans: () => ({ free: { maxRequestsPerMonth: 1000 } }),
      consumeRequest: async () => ({
        account: {
          id: 'acct_1',
          tier: 'free',
          usage: {},
          quotaRemaining: {},
          currentPeriodEnd: new Date(Date.now() + 86_400_000).toISOString(),
        },
        remaining: 999,
      }),
    } as any;

    server = startServer({ port: 0, billingRegistry, connectorManager });
    const address = server.address();
    const port = typeof address === 'object' && address ? address.port : 0;
    baseUrl = `http://127.0.0.1:${port}`;

    const created = await fetch(`${baseUrl}/api/v1/connectors`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Api-Key': 'sk_key_a',
      },
      body: JSON.stringify({
        chain: 'ethereum',
        network: 'mainnet',
        type: 'rpc',
        auth: { apiKey: 'tampered_key' },
      }),
    });

    expect(created.status).toBe(201);
    const createdBody = await created.json();
    expect(createdBody.connector.options.auth.apiKey).toBe('sk_key_a');
    expect(createConnector).toHaveBeenCalledTimes(1);

    const listed = await fetch(`${baseUrl}/api/v1/connectors`, {
      headers: { 'X-Api-Key': 'sk_key_a' },
    });
    expect(listed.status).toBe(200);
    const listedBody = await listed.json();
    expect(listedBody.connectors).toHaveLength(1);
    expect(listedBody.connectors[0].id).toBe('conn_a');

    const forbiddenDelete = await fetch(`${baseUrl}/api/v1/connectors/conn_b`, {
      method: 'DELETE',
      headers: { 'X-Api-Key': 'sk_key_a' },
    });
    expect(forbiddenDelete.status).toBe(403);
    expect(removeConnector).not.toHaveBeenCalled();

    const allowedDelete = await fetch(`${baseUrl}/api/v1/connectors/conn_b`, {
      method: 'DELETE',
      headers: { 'X-Api-Key': 'sk_key_b' },
    });
    expect(allowedDelete.status).toBe(204);
    expect(removeConnector).toHaveBeenCalledWith('conn_b');
  });

  it('returns a single connector by id with ownership check', async () => {
    const stubConnector = {
      id: 'conn_owned',
      options: { auth: { apiKey: 'sk_owner' } },
      chain: { id: 'ethereum' },
      status: 'connected',
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    };

    const connectorManager = {
      createConnector: vi.fn(),
      listConnectors: vi.fn(() => []),
      getConnector: vi.fn((id: string) => id === 'conn_owned' ? stubConnector : undefined),
      removeConnector: vi.fn(),
    } as any;

    const billingRegistry = {
      initialize: async () => undefined,
      getPlans: () => ({ free: { maxRequestsPerMonth: 1000 } }),
      consumeRequest: async () => ({
        account: { id: 'acct_1', tier: 'free', usage: {}, quotaRemaining: {}, currentPeriodEnd: new Date(Date.now() + 86_400_000).toISOString() },
        remaining: 999,
      }),
    } as any;

    server = startServer({ port: 0, billingRegistry, connectorManager });
    const address = server.address();
    const port = typeof address === 'object' && address ? address.port : 0;
    baseUrl = `http://127.0.0.1:${port}`;

    // Owner can fetch their connector
    const owned = await fetch(`${baseUrl}/api/v1/connectors/conn_owned`, {
      headers: { 'X-Api-Key': 'sk_owner' },
    });
    expect(owned.status).toBe(200);
    const ownedBody = await owned.json();
    expect(ownedBody.connector.id).toBe('conn_owned');

    // Wrong key is forbidden
    const forbidden = await fetch(`${baseUrl}/api/v1/connectors/conn_owned`, {
      headers: { 'X-Api-Key': 'sk_other' },
    });
    expect(forbidden.status).toBe(403);

    // Non-existent connector is 404
    const missing = await fetch(`${baseUrl}/api/v1/connectors/conn_missing`, {
      headers: { 'X-Api-Key': 'sk_owner' },
    });
    expect(missing.status).toBe(404);
  });

  it('returns 429 when connector slot quota is exhausted on create', async () => {
    const chainId = CHAIN_REGISTRY[0]?.id ?? 'ethereum';

    const billingRegistry = {
      initialize: async () => undefined,
      getPlans: () => ({ free: { maxRequestsPerMonth: 1000 } }),
      consumeRequest: async () => ({
        account: {
          id: 'acct_1',
          tier: 'free',
          usage: {},
          quotaRemaining: {},
          currentPeriodEnd: new Date(Date.now() + 86_400_000).toISOString(),
        },
        remaining: 999,
      }),
      acquireConnectorSlot: async () => {
        throw new Error('CONNECTOR_QUOTA_EXCEEDED');
      },
      releaseConnectorSlot: async () => undefined,
    } as any;

    server = startServer({ port: 0, billingRegistry });
    const address = server.address();
    const port = typeof address === 'object' && address ? address.port : 0;
    baseUrl = `http://127.0.0.1:${port}`;

    const response = await fetch(`${baseUrl}/api/v1/connectors`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Api-Key': 'sk_key_a',
      },
      body: JSON.stringify({
        chain: chainId,
        network: 'mainnet',
        type: 'rpc',
      }),
    });

    expect(response.status).toBe(429);
    const body = await response.json();
    expect(body.error).toContain('connector quota exceeded');
  });
});

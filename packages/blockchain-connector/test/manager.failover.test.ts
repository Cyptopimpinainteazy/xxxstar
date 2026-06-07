import { describe, it, expect, vi, beforeEach } from 'vitest';
import { ConnectorManager } from '../src/connector/manager';
import type { IChainAdapter } from '../src/adapters/base';

class StubAdapter implements IChainAdapter {
  chain: any;
  endpoint = '';
  connected = false;
  private goodEndpoint: string;
  constructor(chain: any, goodEndpoint: string) {
    this.chain = chain;
    this.goodEndpoint = goodEndpoint;
  }
  async connect(endpoint: string) {
    this.endpoint = endpoint;
    this.connected = true;
    if (endpoint === this.goodEndpoint) return;
    // simulate connect ok but later health check may fail
  }
  async disconnect() { this.connected = false; }
  isConnected() { return this.connected; }
  async getLatestBlock() {
    if (this.endpoint === this.goodEndpoint) return { hash: '0x1', number: 1, parentHash: '0x0', timestamp: new Date().toISOString(), txCount: 0, size: 0 } as any;
    throw new Error('unhealthy');
  }
  async getBlock() { throw new Error('not implemented'); }
  async getTransaction() { throw new Error('not implemented'); }
  async getMetrics() { if (this.endpoint === this.goodEndpoint) return { blockHeight: 1 } as any; throw new Error('unhealthy'); }
}

class StubMonitor {
  private healthy: string | null;
  constructor(healthy: string | null) { this.healthy = healthy; }
  getHealthyEndpoint() { return this.healthy; }
  probeEndpoints() { return Promise.resolve([]); }
}

describe('ConnectorManager failover', () => {
  it('should attempt failover when metrics fail and monitor provides healthy endpoint', async () => {
    const manager = new ConnectorManager();
    // inject monitor
    (manager as any).monitor = new StubMonitor('http://good');

    const chain = { id: 'fake', name: 'Fake', defaultRpcUrls: ['http://bad', 'http://good'], family: 'evm', network: 'mainnet' } as any;
    const connId = 'conn_test';

    const instance = {
      id: connId,
      options: { chain: 'fake', network: 'mainnet', type: 'rpc', endpoint: 'http://bad' },
      chain,
      status: 'connected',
      metrics: { blockHeight: 0 },
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    } as any;

    const adapter = new StubAdapter(chain, 'http://good');
    (manager as any).connectors.set(connId, { instance, adapter });

    // Simulate a refreshMetrics call which will fail and trigger failover
    await expect(manager.refreshMetrics(connId)).resolves.toMatchObject({ blockHeight: 1 });
    const managed = (manager as any).connectors.get(connId);
    expect(managed.instance.options.endpoint).toBe('http://good');
    expect(managed.instance.status).toBe('connected');
  });
});

import { describe, expect, it, vi } from 'vitest';
import { ConnectorManager } from '../src/connector/manager';
import { CHAIN_REGISTRY } from '../src/chains/registry';

describe('ConnectorManager connector quota enforcement', () => {
  it('requires api key when connector quota provider is configured', async () => {
    const manager = new ConnectorManager({
      connectorQuotaProvider: {
        acquireConnectorSlot: vi.fn(async () => ({ remaining: 0 })),
        releaseConnectorSlot: vi.fn(async () => undefined),
      },
    });

    const chainId = CHAIN_REGISTRY[0]?.id;
    expect(chainId).toBeTruthy();

    await expect(
      manager.createConnector({
        chain: chainId!,
        network: 'mainnet',
        type: 'rpc',
      }),
    ).rejects.toThrow('API key required for connector quota enforcement');
  });

  it('surfaces connector quota exceeded errors with stable messaging', async () => {
    const manager = new ConnectorManager({
      connectorQuotaProvider: {
        acquireConnectorSlot: vi.fn(async () => {
          throw new Error('CONNECTOR_QUOTA_EXCEEDED');
        }),
        releaseConnectorSlot: vi.fn(async () => undefined),
      },
    });

    const chainId = CHAIN_REGISTRY[0]?.id;
    expect(chainId).toBeTruthy();

    await expect(
      manager.createConnector({
        chain: chainId!,
        network: 'mainnet',
        type: 'rpc',
        auth: { type: 'api-key', apiKey: 'key-pro' },
      }),
    ).rejects.toThrow('Connector quota exceeded for API key tier');
  });

  it('releases connector slot during connector removal', async () => {
    const releaseConnectorSlot = vi.fn(async () => undefined);
    const manager = new ConnectorManager({
      connectorQuotaProvider: {
        acquireConnectorSlot: vi.fn(async () => ({ remaining: 0 })),
        releaseConnectorSlot,
      },
    });

    const disconnect = vi.fn(async () => undefined);
    (manager as any).connectors.set('conn_test', {
      instance: { id: 'conn_test' },
      adapter: { disconnect },
    });
    (manager as any).connectorQuotaOwners.set('conn_test', 'key-pro');

    await manager.removeConnector('conn_test');

    expect(disconnect).toHaveBeenCalledTimes(1);
    expect(releaseConnectorSlot).toHaveBeenCalledWith('key-pro');
    expect((manager as any).connectors.has('conn_test')).toBe(false);
    expect((manager as any).connectorQuotaOwners.has('conn_test')).toBe(false);
  });
});

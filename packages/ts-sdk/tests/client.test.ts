/**
 * Tests for AtlasSphereClient
 */

import { AtlasSphereClient, ConnectionError } from '../src';

// Mock @polkadot/api
jest.mock('@polkadot/api', () => ({
  ApiPromise: {
    create: jest.fn().mockResolvedValue({
      on: jest.fn(),
      disconnect: jest.fn().mockResolvedValue(undefined),
      rpc: {
        system: {
          chain: jest.fn().mockResolvedValue({ toString: () => 'X3 Chain' }),
          version: jest.fn().mockResolvedValue({ toString: () => '1.0.0' }),
          properties: jest.fn().mockResolvedValue({
            tokenSymbol: { unwrapOr: () => ['X3'] },
            tokenDecimals: { unwrapOr: () => [18] },
            ss58Format: { unwrapOr: () => 42 },
          }),
        },
        chain: {
          getHeader: jest.fn().mockResolvedValue({
            number: { toNumber: () => 100 },
            hash: { toHex: () => '0x1234567890abcdef' },
          }),
        },
      },
      query: {
        system: {
          account: jest.fn().mockResolvedValue({
            data: { free: { toString: () => '1000000000000000000' } },
          }),
        },
        atlasKernel: {
          authorizedAccounts: jest.fn().mockResolvedValue({ isSome: true }),
          comitNonces: jest.fn().mockResolvedValue({ toString: () => '5' }),
        },
      },
    }),
  },
  WsProvider: jest.fn().mockImplementation(() => ({
    disconnect: jest.fn().mockResolvedValue(undefined),
  })),
  HttpProvider: jest.fn(),
}));

describe('AtlasSphereClient', () => {
  let client: AtlasSphereClient;

  beforeEach(() => {
    client = new AtlasSphereClient({ endpoint: 'ws://localhost:9944' });
  });

  afterEach(async () => {
    if (client.isConnected) {
      await client.disconnect();
    }
  });

  describe('connection', () => {
    it('should start disconnected', () => {
      expect(client.status).toBe('disconnected');
      expect(client.isConnected).toBe(false);
    });

    it('should connect successfully', async () => {
      await client.connect();
      expect(client.status).toBe('connected');
      expect(client.isConnected).toBe(true);
    });

    it('should disconnect successfully', async () => {
      await client.connect();
      await client.disconnect();
      expect(client.status).toBe('disconnected');
      expect(client.isConnected).toBe(false);
    });

    it('should throw when accessing API while disconnected', () => {
      expect(() => client.polkadotApi).toThrow(ConnectionError);
    });
  });

  describe('chain info', () => {
    it('should get chain info', async () => {
      await client.connect();
      const info = await client.getChainInfo();

      expect(info.name).toBe('X3 Chain');
      expect(info.version).toBe('1.0.0');
      expect(info.properties.tokenSymbol).toBe('X3');
      expect(info.properties.tokenDecimals).toBe(18);
    });
  });

  describe('balance queries', () => {
    it('should get balance', async () => {
      await client.connect();
      const balance = await client.getBalance('0x' + '00'.repeat(32));

      expect(typeof balance).toBe('bigint');
      expect(balance).toBe(1000000000000000000n);
    });
  });

  describe('authorization queries', () => {
    it('should check authorization', async () => {
      await client.connect();
      const isAuth = await client.isAuthorized('0x' + '00'.repeat(32));

      expect(isAuth).toBe(true);
    });
  });

  describe('nonce queries', () => {
    it('should get nonce', async () => {
      await client.connect();
      const nonce = await client.getNonce('0x' + '00'.repeat(32));

      expect(typeof nonce).toBe('bigint');
      expect(nonce).toBe(5n);
    });
  });
});

/**
 * Tests for X3SubscriptionManager
 */

import { X3SubscriptionManager } from '../src/subscriptions';
import type { BlockNotification } from '../src/subscriptions';

// Mock WsProvider
const mockSubscribe = jest.fn();
const mockSend = jest.fn();
const mockDisconnect = jest.fn().mockResolvedValue(undefined);
const mockOn = jest.fn();

jest.mock('@polkadot/api', () => ({
  WsProvider: jest.fn().mockImplementation(() => ({
    subscribe: mockSubscribe,
    send: mockSend,
    disconnect: mockDisconnect,
    on: mockOn,
  })),
}));

describe('X3SubscriptionManager', () => {
  let manager: X3SubscriptionManager;

  beforeEach(() => {
    jest.clearAllMocks();
    manager = new X3SubscriptionManager('ws://localhost:9944');

    // Auto-resolve connection
    mockOn.mockImplementation((event: string, handler: Function) => {
      if (event === 'connected') {
        setTimeout(() => handler(), 0);
      }
    });
  });

  afterEach(async () => {
    if (manager.connected) {
      await manager.disconnect();
    }
  });

  describe('constructor', () => {
    it('should create with default endpoint', () => {
      const m = new X3SubscriptionManager();
      expect(m.connected).toBe(false);
    });

    it('should create with custom endpoint', () => {
      const m = new X3SubscriptionManager('ws://custom:9944');
      expect(m.connected).toBe(false);
    });
  });

  describe('connect', () => {
    it('should connect to the endpoint', async () => {
      await manager.connect();
      expect(manager.connected).toBe(true);
    });

    it('should not reconnect if already connected', async () => {
      await manager.connect();
      await manager.connect(); // Should be no-op
      expect(manager.connected).toBe(true);
    });
  });

  describe('subscribeNewBlocks', () => {
    it('should subscribe to new block headers', async () => {
      await manager.connect();

      mockSubscribe.mockResolvedValue('sub-1');

      const callback = jest.fn();
      const id = await manager.subscribeNewBlocks(callback);

      expect(id).toMatch(/^newBlocks_/);
      expect(mockSubscribe).toHaveBeenCalledWith(
        'x3_newBlock',
        'x3_subscribeNewBlocks',
        [],
        expect.any(Function)
      );
      expect(manager.activeSubscriptionCount).toBe(1);
    });

    it('should throw if not connected', async () => {
      await expect(manager.subscribeNewBlocks(jest.fn())).rejects.toThrow('Not connected');
    });
  });

  describe('subscribeFinalizedBlocks', () => {
    it('should subscribe to finalized block headers', async () => {
      await manager.connect();

      mockSubscribe.mockResolvedValue('sub-2');

      const callback = jest.fn();
      const id = await manager.subscribeFinalizedBlocks(callback);

      expect(id).toMatch(/^finalizedBlocks_/);
      expect(mockSubscribe).toHaveBeenCalledWith(
        'x3_finalizedBlock',
        'x3_subscribeFinalizedBlocks',
        [],
        expect.any(Function)
      );
    });
  });

  describe('subscribeComits', () => {
    it('should subscribe to comit events', async () => {
      await manager.connect();

      mockSubscribe.mockResolvedValue('sub-3');

      const callback = jest.fn();
      const id = await manager.subscribeComits(callback);

      expect(id).toMatch(/^comits_/);
      expect(mockSubscribe).toHaveBeenCalledWith(
        'x3_newComit',
        'x3_subscribeComits',
        [],
        expect.any(Function)
      );
    });
  });

  describe('unsubscribe', () => {
    it('should unsubscribe from a subscription', async () => {
      await manager.connect();
      mockSubscribe.mockResolvedValue('sub-1');

      const id = await manager.subscribeNewBlocks(jest.fn());
      expect(manager.activeSubscriptionCount).toBe(1);

      const result = await manager.unsubscribe(id);
      expect(result).toBe(true);
      expect(manager.activeSubscriptionCount).toBe(0);
    });

    it('should return false for unknown subscription', async () => {
      const result = await manager.unsubscribe('nonexistent');
      expect(result).toBe(false);
    });
  });

  describe('disconnect', () => {
    it('should clean up all subscriptions on disconnect', async () => {
      await manager.connect();
      mockSubscribe.mockResolvedValue('sub-1');

      await manager.subscribeNewBlocks(jest.fn());
      await manager.subscribeComits(jest.fn());
      expect(manager.activeSubscriptionCount).toBe(2);

      await manager.disconnect();
      expect(manager.connected).toBe(false);
      expect(manager.activeSubscriptionCount).toBe(0);
    });
  });

  describe('setHandlers', () => {
    it('should set event handlers', () => {
      const onError = jest.fn();
      const onConnected = jest.fn();

      manager.setHandlers({ onError, onConnected });
      // Handlers are stored internally - verified by behavior
    });
  });

  describe('callback invocation', () => {
    it('should invoke callback with block data', async () => {
      await manager.connect();

      let subscriberCallback: Function | null = null;
      mockSubscribe.mockImplementation(
        (_sub: string, _method: string, _params: any[], cb: Function) => {
          subscriberCallback = cb;
          return Promise.resolve('sub-1');
        }
      );

      const blocks: BlockNotification[] = [];
      await manager.subscribeNewBlocks((block) => blocks.push(block));

      // Simulate a block notification
      const mockBlock: BlockNotification = {
        number: 42,
        hash: '0xabcdef',
        parentHash: '0x123456',
        stateRoot: '0xaaa',
        extrinsicsRoot: '0xbbb',
      };

      subscriberCallback!(null, mockBlock);

      expect(blocks).toHaveLength(1);
      expect(blocks[0].number).toBe(42);
      expect(blocks[0].hash).toBe('0xabcdef');
    });

    it('should invoke error handler on subscription error', async () => {
      await manager.connect();

      const onError = jest.fn();
      manager.setHandlers({ onError });

      let subscriberCallback: Function | null = null;
      mockSubscribe.mockImplementation(
        (_sub: string, _method: string, _params: any[], cb: Function) => {
          subscriberCallback = cb;
          return Promise.resolve('sub-1');
        }
      );

      await manager.subscribeNewBlocks(jest.fn());

      // Simulate an error
      subscriberCallback!(new Error('stream died'), null);

      expect(onError).toHaveBeenCalledWith(expect.any(Error));
    });
  });
});

/**
 * Live integration tests for @x3-chain/ts-sdk that exercise a running X3 node.
 *
 * These tests are skipped by default. To run against a local/dev node:
 *
 *   RUN_LIVE_INTEGRATION_TESTS=1 npm test --workspace packages/ts-sdk
 *
 * INV-REF: tests/invariants/registry.toml — API-TS-SDK-001
 */

import { AtlasSphereClient } from '../src';

const RUN_LIVE = process.env.RUN_LIVE_INTEGRATION_TESTS === '1';
const WS_ENDPOINT = process.env.X3_WS_ENDPOINT ?? 'ws://127.0.0.1:9944';

(RUN_LIVE ? describe : describe.skip)('X3 TS SDK — live node integration', () => {
  jest.setTimeout(30_000);
  let client: AtlasSphereClient | null = null;

  afterAll(async () => {
    if (client && client.isConnected) {
      await client.disconnect();
    }
  });

  test('connects to live node and queries basic RPCs', async () => {
    client = new AtlasSphereClient({ endpoint: WS_ENDPOINT });
    await client.connect();

    const info = await client.getChainInfo();
    expect(info).toBeDefined();
    expect(typeof info.name).toBe('string');
    expect(typeof info.version).toBe('string');
    expect(typeof info.properties.tokenDecimals).toBe('number');

    const blockNumber = await client.getBlockNumber();
    expect(typeof blockNumber).toBe('number');
    expect(blockNumber).toBeGreaterThanOrEqual(0);

    // Query a generic account (may be zero) and ensure SDK returns a bigint
    const someBalance = await client.getBalance('0x' + '00'.repeat(32));
    expect(typeof someBalance).toBe('bigint');

    // Nonce query should return bigint (may be 0)
    const nonce = await client.getNonce('0x' + '00'.repeat(32));
    expect(typeof nonce).toBe('bigint');

    await client.disconnect();
    client = null;
  });

  /**
   * Live test: send a `balances.transfer` on testnet.
   * - Skipped unless RUN_LIVE_INTEGRATION_TESTS=1 (describe is gated) AND
   *   TESTNET_SIGNER_URI environment variable is provided.
   * - Use TESTNET_RECEIVER (optional) and TESTNET_TRANSFER_AMOUNT (optional).
   * NOTE: the test will perform a small transfer (default 1 unit). Use a test account.
   */
  test('sends balances.transfer on testnet (gated)', async () => {
    const signerUri = process.env.TESTNET_SIGNER_URI;
    if (!signerUri) {
      console.warn('Skipping test: TESTNET_SIGNER_URI not set');
      return;
    }

    const { ApiPromise, WsProvider } = await import('@polkadot/api');
    const { Keyring } = await import('@polkadot/keyring');

    const endpoint = process.env.X3_WS_ENDPOINT ?? 'wss://testnet.atlassphere.io';
    const api = await ApiPromise.create({ provider: new WsProvider(endpoint) });

    const keyring = new Keyring({ type: 'sr25519' });
    const sender = keyring.addFromUri(signerUri);
    const receiver = process.env.TESTNET_RECEIVER ?? sender.address; // self-transfer if not provided
    const amount = process.env.TESTNET_TRANSFER_AMOUNT ?? '1';

    // Submit transfer and wait for finalization
    const result = await new Promise((resolve, reject) => {
      api.tx.balances
        .transfer(receiver, amount)
        .signAndSend(sender, (res: any) => {
          if (res.status.isFinalized) return resolve(res);
          if (res.status.isInvalid || res.status.isDropped) return reject(new Error('tx failed'));
        })
        .catch(reject);
    });

    // Basic assertions
    expect((result as any).status.isFinalized).toBeTruthy();

    await api.disconnect();
  }, 60_000);
});

import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { mkdtemp, rm } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import { WebSocket } from 'ws';
import { startServer } from '../src/server/index';

describe('Blockchain connector websocket billing', () => {
  let tempDir = '';
  let server: ReturnType<typeof startServer> | undefined;
  let baseWsUrl = '';

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
    tempDir = await mkdtemp(join(tmpdir(), 'x3-connector-ws-'));
    process.env.BLOCKCHAIN_CONNECTOR_BILLING_DB = join(tempDir, 'billing.json');
    process.env.BLOCKCHAIN_CONNECTOR_BOOTSTRAP_API_KEY = 'sk_x3_test_bootstrap';

    server = startServer({ port: 0 });
    const address = server.address();
    const port = typeof address === 'object' && address ? address.port : 0;
    baseWsUrl = `ws://127.0.0.1:${port}/ws`;
  });

  afterEach(async () => {
    delete process.env.BLOCKCHAIN_CONNECTOR_BILLING_DB;
    delete process.env.BLOCKCHAIN_CONNECTOR_BOOTSTRAP_API_KEY;
    await closeServer();
    if (tempDir) {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it('accepts a websocket with a valid API key and sends welcome frame', async () => {
    const ws = new WebSocket(`${baseWsUrl}?apiKey=sk_x3_test_bootstrap`);

    const message = await new Promise<string>((resolve, reject) => {
      ws.once('message', (data) => resolve(data.toString()));
      ws.once('error', reject);
    });

    expect(message).toContain('welcome');
    expect(message).toContain('free');

    ws.close();
    await new Promise<void>((resolve) => ws.once('close', () => resolve()));
  });

  it('rejects websocket connections when concurrent quota is exceeded', async () => {
    const sockets = await Promise.all(
      Array.from({ length: 5 }, async () => {
        const ws = new WebSocket(`${baseWsUrl}?apiKey=sk_x3_test_bootstrap`);
        await new Promise<void>((resolve, reject) => {
          ws.once('message', () => resolve());
          ws.once('error', reject);
        });
        return ws;
      }),
    );

    const rejected = await new Promise<{ statusCode: number; body: string }>((resolve, reject) => {
      const ws = new WebSocket(`${baseWsUrl}?apiKey=sk_x3_test_bootstrap`);
      ws.once('unexpected-response', (_request, response) => {
        let body = '';
        response.on('data', (chunk) => {
          body += chunk.toString();
        });
        response.on('end', () => {
          resolve({ statusCode: response.statusCode ?? 0, body });
        });
      });
      ws.once('open', () => reject(new Error('expected websocket quota rejection')));
      ws.once('error', () => undefined);
    });

    expect(rejected.statusCode).toBe(429);
    expect(rejected.body).toContain('concurrent websocket quota exceeded');

    await Promise.all(
      sockets.map(async (ws) => {
        ws.close();
        await new Promise<void>((resolve) => ws.once('close', () => resolve()));
      }),
    );
  });
});
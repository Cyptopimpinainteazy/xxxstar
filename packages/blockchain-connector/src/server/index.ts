import http from 'http';
import type { HealthMonitor } from '../connector/health-monitor';
import { ConnectorManager } from '../connector/manager';
import type { ConnectorOptions } from '../types';
import client from 'prom-client';
import { WebSocketServer, type RawData, type WebSocket } from 'ws';
import { BillingRegistry, extractApiKey } from './billing';

let defaultMetricsInitialized = false;

export function startServer(
  {
    monitor,
    port = 9464,
    billingRegistry,
    connectorManager,
  }: {
    monitor?: HealthMonitor;
    port?: number;
    billingRegistry?: BillingRegistry;
    connectorManager?: ConnectorManager;
  },
) {
  // expose Prometheus metrics
  if (!defaultMetricsInitialized) {
    client.collectDefaultMetrics();
    defaultMetricsInitialized = true;
  }

  const billing = billingRegistry ?? new BillingRegistry();
  const billingReady = billing.initialize();
  const manager = connectorManager ?? new ConnectorManager({ connectorQuotaProvider: billing });
  const wsServer = new WebSocketServer({ noServer: true });

  const readJsonBody = async (req: http.IncomingMessage): Promise<any> => {
    const chunks: Buffer[] = [];
    for await (const chunk of req) {
      chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk));
    }

    if (chunks.length === 0) {
      return {};
    }

    const raw = Buffer.concat(chunks).toString('utf8');
    try {
      return JSON.parse(raw);
    } catch {
      throw new Error('INVALID_JSON_BODY');
    }
  };

  const server = http.createServer(async (req, res) => {
    if (!req.url) return res.end('');
    const requestUrl = new URL(req.url, `http://${req.headers.host ?? 'localhost'}`);
    const path = requestUrl.pathname;

    if (path.startsWith('/metrics')) {
      try {
        const metrics = await client.register.metrics();
        res.writeHead(200, { 'Content-Type': client.register.contentType });
        res.end(metrics);
      } catch (e: any) {
        res.writeHead(500);
        res.end('error');
      }
      return;
    }

    if (path.startsWith('/health')) {
      const body: any = { version: '0.1.0', status: 'unknown' };
      if (monitor) {
        // expose top-level counts
        const map = (monitor as any).statuses as Map<string, any> | undefined;
        if (map) {
          const statuses = Array.from(map.values());
          body.totalEndpoints = statuses.length;
          body.healthy = statuses.filter((s: any) => s.healthy).length;
          body.percentHealthy = ((body.healthy / body.totalEndpoints) * 100) || 0;
        }
        body.status = 'ok';
      }
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify(body));
      return;
    }

    try {
      await billingReady;
    } catch {
      res.writeHead(500, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ error: 'billing subsystem initialization failed' }));
      return;
    }

    if (path === '/api/v1/billing/plans') {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ plans: billing.getPlans() }));
      return;
    }

    if (path.startsWith('/api/v1/admin/')) {
      const adminSecret = process.env.BLOCKCHAIN_CONNECTOR_ADMIN_SECRET;
      if (!adminSecret) {
        res.writeHead(503, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: 'admin API not configured' }));
        return;
      }

      const providedSecret =
        (req.headers['x-admin-secret'] as string | undefined) ??
        requestUrl.searchParams.get('adminSecret') ??
        '';

      if (providedSecret !== adminSecret) {
        res.writeHead(401, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: 'invalid admin secret' }));
        return;
      }

      if (path === '/api/v1/admin/accounts' && req.method === 'GET') {
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ accounts: billing.listAccounts() }));
        return;
      }

      if (path === '/api/v1/admin/accounts' && req.method === 'POST') {
        let payload: { tier?: string } = {};
        try {
          payload = await readJsonBody(req);
        } catch {
          payload = {};
        }

        const tier = (['free', 'bronze', 'silver', 'gold', 'enterprise'].includes(payload.tier ?? '')
          ? payload.tier
          : 'free') as import('../types').BillingTier;

        const { apiKey, account } = await billing.createAccount(tier);
        res.writeHead(201, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ apiKey, account }));
        return;
      }

      if (path.startsWith('/api/v1/admin/accounts/') && req.method === 'DELETE') {
        const targetApiKey = decodeURIComponent(path.slice('/api/v1/admin/accounts/'.length));
        const force = requestUrl.searchParams.get('force') === 'true';
        const result = await billing.revokeAccount(targetApiKey, { force });
        if (result === 'not_found') {
          res.writeHead(404, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ error: 'account not found' }));
          return;
        }
        if (result === 'protected') {
          res.writeHead(409, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ error: 'cannot revoke the last account without force=true' }));
          return;
        }
        res.writeHead(204);
        res.end();
        return;
      }

      if (path.startsWith('/api/v1/admin/accounts/') && path.endsWith('/tier') && req.method === 'POST') {
        const pathBase = '/api/v1/admin/accounts/';
        const pathEnd = '/tier';
        const targetApiKey = decodeURIComponent(path.slice(pathBase.length, -pathEnd.length));
        let payload: { tier?: string } = {};
        try {
          payload = await readJsonBody(req);
        } catch {
          payload = {};
        }

        const tier = (['free', 'bronze', 'silver', 'gold', 'enterprise'].includes(payload.tier ?? '')
          ? payload.tier
          : 'free') as import('../types').BillingTier;

        const updated = await billing.changeTier(targetApiKey, tier);
        if (!updated) {
          res.writeHead(404, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ error: 'account not found' }));
          return;
        }

        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ account: updated }));
        return;
      }

      if (path.startsWith('/api/v1/admin/accounts/') && path.endsWith('/rotate') && req.method === 'POST') {
        const pathBase = '/api/v1/admin/accounts/';
        const pathEnd = '/rotate';
        const targetApiKey = decodeURIComponent(path.slice(pathBase.length, -pathEnd.length));

        const rotated = await billing.rotateApiKey(targetApiKey);
        if (!rotated) {
          res.writeHead(404, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ error: 'account not found' }));
          return;
        }

        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ newApiKey: rotated.newApiKey, account: rotated.account }));
        return;
      }

      if (path.startsWith('/api/v1/admin/accounts/') && path.endsWith('/reset') && req.method === 'POST') {
        const pathBase = '/api/v1/admin/accounts/';
        const pathEnd = '/reset';
        const targetApiKey = decodeURIComponent(path.slice(pathBase.length, -pathEnd.length));

        const reset = await billing.resetAccountUsage(targetApiKey);
        if (!reset) {
          res.writeHead(404, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ error: 'account not found' }));
          return;
        }

        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ account: reset }));
        return;
      }

      res.writeHead(404);
      res.end('not found');
      return;
    }

    if (path.startsWith('/api/')) {
      const apiKey = extractApiKey(req.headers, requestUrl);
      if (!apiKey) {
        res.writeHead(401, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: 'missing API key' }));
        return;
      }

      try {
        const { account, remaining } = await billing.consumeRequest(apiKey);
        res.setHeader('X-Billing-Tier', account.tier);
        res.setHeader('X-RateLimit-Remaining', String(remaining));

        if (path === '/api/v1/billing/status') {
          res.writeHead(200, { 'Content-Type': 'application/json' });
          res.end(
            JSON.stringify({
              accountId: account.id,
              tier: account.tier,
              usage: account.usage,
              quotaRemaining: account.quotaRemaining,
              currentPeriodEnd: account.currentPeriodEnd,
            }),
          );
          return;
        }

        if (path === '/api/v1/connectors' && req.method === 'GET') {
          const connectors = manager
            .listConnectors()
            .filter((connector) => connector.options.auth?.apiKey === apiKey);
          res.writeHead(200, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ connectors }));
          return;
        }

        if (path === '/api/v1/connectors' && req.method === 'POST') {
          let payload: Partial<ConnectorOptions>;
          try {
            payload = await readJsonBody(req);
          } catch (error: any) {
            if (error?.message === 'INVALID_JSON_BODY') {
              res.writeHead(400, { 'Content-Type': 'application/json' });
              res.end(JSON.stringify({ error: 'invalid JSON request body' }));
              return;
            }
            throw error;
          }

          if (!payload.chain || !payload.network || !payload.type) {
            res.writeHead(400, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({ error: 'missing required fields: chain, network, type' }));
            return;
          }

          try {
            const connector = await manager.createConnector({
              ...payload,
              chain: payload.chain,
              network: payload.network,
              type: payload.type,
              auth: {
                ...(payload.auth ?? {}),
                apiKey,
              },
            });
            res.writeHead(201, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({ connector }));
            return;
          } catch (error: any) {
            if (error?.message === 'Connector quota exceeded for API key tier') {
              res.writeHead(429, { 'Content-Type': 'application/json' });
              res.end(JSON.stringify({ error: 'connector quota exceeded for current plan' }));
              return;
            }
            if (error?.message === 'Invalid API key') {
              res.writeHead(401, { 'Content-Type': 'application/json' });
              res.end(JSON.stringify({ error: 'invalid API key' }));
              return;
            }
            res.writeHead(400, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({ error: error?.message ?? 'connector creation failed' }));
            return;
          }
        }

        if (path.startsWith('/api/v1/connectors/') && req.method === 'GET') {
          const connectorId = decodeURIComponent(path.slice('/api/v1/connectors/'.length));
          const connector = manager.getConnector(connectorId);
          if (!connector) {
            res.writeHead(404, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({ error: 'connector not found' }));
            return;
          }

          if (connector.options.auth?.apiKey !== apiKey) {
            res.writeHead(403, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({ error: 'connector does not belong to API key' }));
            return;
          }

          res.writeHead(200, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ connector }));
          return;
        }

        if (path.startsWith('/api/v1/connectors/') && req.method === 'DELETE') {
          const connectorId = decodeURIComponent(path.slice('/api/v1/connectors/'.length));
          const connector = manager.getConnector(connectorId);
          if (!connector) {
            res.writeHead(404, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({ error: 'connector not found' }));
            return;
          }

          if (connector.options.auth?.apiKey !== apiKey) {
            res.writeHead(403, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({ error: 'connector does not belong to API key' }));
            return;
          }

          await manager.removeConnector(connectorId);
          res.writeHead(204);
          res.end();
          return;
        }
      } catch (error: any) {
        if (error?.message === 'INVALID_API_KEY') {
          res.writeHead(401, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ error: 'invalid API key' }));
          return;
        }
        if (error?.message === 'QUOTA_EXCEEDED') {
          res.writeHead(429, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ error: 'monthly request quota exceeded' }));
          return;
        }
        res.writeHead(500, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: 'billing enforcement failure' }));
        return;
      }
    }

    res.writeHead(404);
    res.end('not found');
  });

  const rejectUpgrade = (socket: any, statusCode: number, message: string) => {
    const body = JSON.stringify({ error: message });
    socket.write(
      `HTTP/1.1 ${statusCode} ${statusCode === 401 ? 'Unauthorized' : statusCode === 429 ? 'Too Many Requests' : 'Internal Server Error'}\r\n` +
      'Content-Type: application/json\r\n' +
      'Connection: close\r\n' +
      `Content-Length: ${Buffer.byteLength(body)}\r\n\r\n` +
      body,
    );
    socket.destroy();
  };

  server.on('upgrade', (req, socket, head) => {
    void (async () => {
      if (!req.url) {
        rejectUpgrade(socket, 400, 'missing request URL');
        return;
      }

      const requestUrl = new URL(req.url, `http://${req.headers.host ?? 'localhost'}`);
      if (requestUrl.pathname !== '/ws') {
        socket.destroy();
        return;
      }

      try {
        await billingReady;
      } catch {
        rejectUpgrade(socket, 500, 'billing subsystem initialization failed');
        return;
      }

      const apiKey = extractApiKey(req.headers, requestUrl);
      if (!apiKey) {
        rejectUpgrade(socket, 401, 'missing API key');
        return;
      }

      let session: { account: { tier: string }; connectionId: string; remaining: number };
      try {
        session = await billing.acquireWsConnection(apiKey);
      } catch (error: any) {
        if (error?.message === 'INVALID_API_KEY') {
          rejectUpgrade(socket, 401, 'invalid API key');
          return;
        }
        if (error?.message === 'WS_QUOTA_EXCEEDED') {
          rejectUpgrade(socket, 429, 'concurrent websocket quota exceeded');
          return;
        }
        rejectUpgrade(socket, 500, 'billing enforcement failure');
        return;
      }

      const connectedAtMs = Date.now();

      wsServer.handleUpgrade(req, socket, head, (ws) => {
        wsServer.emit('connection', ws, req, {
          apiKey,
          account: session.account,
          connectionId: session.connectionId,
          remaining: session.remaining,
          connectedAtMs,
        });
      });
    })().catch(() => {
      socket.destroy();
    });
  });

  wsServer.on('connection', (ws: WebSocket, _req: http.IncomingMessage, session: any) => {
    ws.send(JSON.stringify({
      type: 'welcome',
      tier: session.account.tier,
      concurrentRemaining: session.remaining,
    }));

    let released = false;
    const release = async () => {
      if (released) {
        return;
      }
      released = true;
      await billing.releaseWsConnection(session.apiKey, session.connectionId, session.connectedAtMs).catch(() => undefined);
    };

    ws.on('message', (raw: RawData) => {
      let payload: any;
      try {
        payload = JSON.parse(raw.toString());
      } catch {
        ws.send(JSON.stringify({ error: 'invalid json frame' }));
        return;
      }

      if (payload?.action === 'subscribe') {
        ws.send(JSON.stringify({ type: 'subscribed', connectorId: payload.connectorId ?? null, events: payload.events ?? [] }));
        return;
      }

      ws.send(JSON.stringify({ type: 'ack' }));
    });

    ws.on('close', () => { void release(); });
    ws.on('error', () => { void release(); });
  });

  server.listen(port, () => console.log(`blockchain-connector server listening on :${port}`));
  server.on('close', () => {
    wsServer.close();
  });
  return server;
}

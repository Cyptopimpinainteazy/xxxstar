import express from 'express';
import bodyParser from 'body-parser';
import crypto from 'crypto';

export type SettlementPayload = {
  swapId: string;
  chain: string;
  preimage?: string;
  lock?: any;
  type?: string;
};

const app = express();
app.use(bodyParser.json());

// Authentication & integrity middleware
app.use((req, res, next) => {
  const token = process.env.RELAYER_TOKEN;
  const hmacSecret = process.env.RELAYER_HMAC_SECRET;

  // If HMAC secret is configured, verify X-Signature
  if (hmacSecret) {
    const sig = req.headers['x-signature'] as string | undefined;
    if (!sig) {
      res.status(401).json({ error: 'Missing X-Signature header' });
      return;
    }

    // Node's body parser has already run, so we need raw body; use JSON.stringify of body
    const bodyStr = JSON.stringify(req.body || {});
    const expected = crypto.createHmac('sha256', hmacSecret).update(bodyStr).digest('hex');
    if (!crypto.timingSafeEqual(Buffer.from(expected), Buffer.from(sig))) {
      res.status(403).json({ error: 'Invalid signature' });
      return;
    }
  }

  // If token configured, check Bearer token as well
  if (token) {
    const auth = req.headers['authorization'];
    if (!auth || !auth.toString().startsWith('Bearer ')) {
      res.status(401).json({ error: 'Missing Authorization Bearer token' });
      return;
    }
    const provided = auth.toString().slice('Bearer '.length).trim();
    // Use constant-time comparison
    const match = crypto.timingSafeEqual(Buffer.from(provided), Buffer.from(token));
    if (!match) {
      res.status(403).json({ error: 'Invalid token' });
      return;
    }
  }

  next();
});

// Pluggable chain handlers
const handlers: Map<string, (payload: SettlementPayload) => Promise<string>> = new Map();

export function registerHandler(chain: string, fn: (payload: SettlementPayload) => Promise<string>) {
  handlers.set(chain, fn);
}

// Initialize local KMS from environment if present (lazy import to avoid ESM/CJS require cycles)
(async () => {
  try {
    const bootstrap = await import('./kms/bootstrap');
    if (bootstrap && typeof (bootstrap as any).initLocalKmsFromEnv === 'function') {
      (bootstrap as any).initLocalKmsFromEnv();
    }
  } catch (err) {
    // OK if not present or failed to load
  }
})();

// Auto-register built-in handlers when available
try {
  const eth = require('./handlers/ethereum');
  if (eth && typeof eth.ethereumHandler === 'function') {
    registerHandler('ethereum', eth.ethereumHandler);
    console.info('Relayer: registered ethereum handler');
  }
} catch (err) {
  // not available — skip
}

try {
  const btc = require('./handlers/bitcoin');
  if (btc && typeof btc.bitcoinHandler === 'function') {
    registerHandler('bitcoin', btc.bitcoinHandler);
    console.info('Relayer: registered bitcoin handler');
  }
} catch (err) {
  // not available — skip
}

// Default handler returns mock txid
async function defaultHandler(payload: SettlementPayload): Promise<string> {
  return `relayer-mock-tx-${payload.swapId}-${payload.chain}-${Date.now()}`;
}

app.post('/settlement', async (req, res) => {
  const payload = req.body as SettlementPayload;

  if (!payload || !payload.swapId || !payload.chain) {
    res.status(400).json({ error: 'Missing required fields: swapId, chain' });
    return;
  }

  try {
    const handler = handlers.get(payload.chain) ?? defaultHandler;
    const txid = await handler(payload);
    res.json({ txid });
  } catch (err) {
    console.error('Relayer handler failed:', (err as Error).message);
    res.status(500).json({ error: (err as Error).message });
  }
});

app.get('/health', (_req, res) => {
  res.json({ status: 'ok', tls: !!(process.env.RELAYER_TLS_CERT && process.env.RELAYER_TLS_KEY), auth: !!process.env.RELAYER_TOKEN });
});

export default app;

if (require.main === module) {
  const useTls = !!(process.env.RELAYER_TLS_CERT && process.env.RELAYER_TLS_KEY);
  const port = process.env.PORT ? parseInt(process.env.PORT) : 9090;

  if (useTls) {
    // Lazy import to avoid adding heavy deps when not used
    const https = require('https');
    const fs = require('fs');
    const cert = fs.readFileSync(process.env.RELAYER_TLS_CERT);
    const key = fs.readFileSync(process.env.RELAYER_TLS_KEY);
    https.createServer({ cert, key }, app).listen(port, () => {
      console.log(`Relayer stub listening (TLS) on :${port}`);
    });
  } else {
    app.listen(port, () => {
      console.log(`Relayer stub listening on :${port}`);
    });
  }
}

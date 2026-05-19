#!/usr/bin/env node
/**
 * Mock Substrate JSON-RPC Server for local development
 * Simulates X3 Chain node on ports 9933 (HTTP) and 9944 (WebSocket)
 *
 * This allows testing x3-live-data.js without building the full Substrate node.
 */

import http from 'http';
import { WebSocketServer } from 'ws';

const HTTP_PORT = 9933;
const WS_PORT = 9944;
const OPEN = 1; // WebSocket.OPEN

// Simulated chain state
let state = {
  blockHeight: 12847,
  validators: 847,
  totalIssuance: 1_000_000_000_000n, // 1M X3S in Planck units
  totalTx: 2841204,
  tps: 4218,
  timestamp: Date.now(),
};

// Track active WebSocket subscriptions
const subscriptions = new Map();

/**
 * Convert number to hex with 0x prefix
 */
function toHex(n) {
  if (typeof n === 'bigint') {
    return '0x' + n.toString(16);
  }
  return '0x' + n.toString(16);
}

/**
 * Generate mock block extrinsics (for TPS calculation)
 */
function generateExtrinsics(count) {
  return Array(count)
    .fill(null)
    .map((_, i) => '0x' + i.toString(16));
}

/**
 * Simulate block production every 6 seconds
 */
function startBlockProduction() {
  setInterval(() => {
    state.blockHeight++;
    state.totalTx += Math.floor(Math.random() * 20) + 5;
    state.tps = Math.floor(Math.random() * 2000) + 2000;
    state.timestamp = Date.now();

    // Broadcast new block to all WebSocket subscribers
    const header = {
      number: toHex(state.blockHeight),
      parentHash: '0x' + 'a'.repeat(64),
      stateRoot: '0x' + 'b'.repeat(64),
      extrinsicsRoot: '0x' + 'c'.repeat(64),
      digest: { logs: [] },
    };

    subscriptions.forEach((subId, wsClient) => {
      if (wsClient.readyState === OPEN) {
        wsClient.send(
          JSON.stringify({
            jsonrpc: '2.0',
            method: 'chain_newHead',
            params: { subscription: subId, result: header },
          })
        );
      }
    });

    console.log(`📦 Block #${state.blockHeight} produced (TPS: ${state.tps})`);
  }, 6000);
}

/**
 * HTTP RPC Server (port 9933)
 */
const httpServer = http.createServer((req, res) => {
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'POST, GET, OPTIONS');
  res.setHeader('Content-Type', 'application/json');

  if (req.method === 'OPTIONS') {
    res.writeHead(200);
    res.end();
    return;
  }

  if (req.method !== 'POST') {
    res.writeHead(404);
    res.end(JSON.stringify({ error: 'Not found' }));
    return;
  }

  let body = '';
  req.on('data', chunk => {
    body += chunk;
  });

  req.on('end', () => {
    try {
      const { id, jsonrpc, method, params } = JSON.parse(body);

      let result = null;

      if (method === 'chain_getHeader') {
        result = {
          number: toHex(state.blockHeight),
          parentHash: '0x' + 'a'.repeat(64),
          stateRoot: '0x' + 'b'.repeat(64),
          extrinsicsRoot: '0x' + 'c'.repeat(64),
          digest: { logs: [] },
        };
      } else if (method === 'chain_getBlock') {
        const exCount = Math.floor(Math.random() * 20) + 5;
        result = {
          block: {
            header: {
              number: toHex(state.blockHeight),
              parentHash: '0x' + 'a'.repeat(64),
              stateRoot: '0x' + 'b'.repeat(64),
              extrinsicsRoot: '0x' + 'c'.repeat(64),
              digest: { logs: [] },
            },
            extrinsics: generateExtrinsics(exCount),
          },
        };
      } else if (method === 'session_validators') {
        result = Array(state.validators)
          .fill(null)
          .map((_, i) => '0x' + i.toString(16).padStart(64, '0'));
      } else if (method === 'balances_totalIssuance' || method === 'state_getStorage') {
        result = toHex(state.totalIssuance);
      } else if (method === 'system_health') {
        result = {
          peers: Math.floor(Math.random() * 50) + 10,
          isSyncing: false,
          shouldHavePeers: true,
        };
      } else {
        result = { error: `Method ${method} not implemented in mock` };
      }

      res.writeHead(200);
      res.end(
        JSON.stringify({
          jsonrpc,
          id,
          result: result.error ? undefined : result,
          error: result.error,
        })
      );
    } catch (e) {
      res.writeHead(400);
      res.end(JSON.stringify({ error: 'Invalid JSON' }));
    }
  });
});

/**
 * WebSocket Server (port 9944)
 */
const wss = new WebSocketServer({ port: WS_PORT });

wss.on('connection', ws => {
  console.log(`🔗 WebSocket client connected`);

  ws.on('message', msg => {
    try {
      const { id, jsonrpc, method, params } = JSON.parse(msg);

      if (method === 'chain_subscribeNewHeads') {
        const subId = 'sub_' + Math.random().toString(36).slice(2);
        subscriptions.set(ws, subId);

        ws.send(
          JSON.stringify({
            jsonrpc,
            id,
            result: subId,
          })
        );

        // Send initial block
        ws.send(
          JSON.stringify({
            jsonrpc: '2.0',
            method: 'chain_newHead',
            params: {
              subscription: subId,
              result: {
                number: toHex(state.blockHeight),
                parentHash: '0x' + 'a'.repeat(64),
                stateRoot: '0x' + 'b'.repeat(64),
                extrinsicsRoot: '0x' + 'c'.repeat(64),
                digest: { logs: [] },
              },
            },
          })
        );

        console.log(`📡 Subscribed to chain_newHead: ${subId}`);
      } else {
        ws.send(
          JSON.stringify({
            jsonrpc,
            id,
            error: `Method ${method} not implemented for WebSocket`,
          })
        );
      }
    } catch (e) {
      ws.send(JSON.stringify({ error: 'Invalid message' }));
    }
  });

  ws.on('close', () => {
    subscriptions.delete(ws);
    console.log(`🔌 WebSocket client disconnected`);
  });
});

// Start servers
httpServer.listen(HTTP_PORT, () => {
  console.log(`🚀 Mock Substrate RPC HTTP Server listening on port ${HTTP_PORT}`);
  console.log(`   Endpoint: http://localhost:${HTTP_PORT}/rpc`);
});

wss.on('listening', () => {
  console.log(`🌐 Mock Substrate RPC WebSocket Server listening on port ${WS_PORT}`);
  console.log(`   Endpoint: ws://localhost:${WS_PORT}`);
});

// Start block production simulation
startBlockProduction();

console.log(`\n📊 Simulated chain state:`);
console.log(`   Block Height: ${state.blockHeight}`);
console.log(`   Validators: ${state.validators}`);
console.log(`   Total TPS: ${state.tps}`);
console.log(`   Total Tx: ${state.totalTx}\n`);

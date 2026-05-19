#!/usr/bin/env node
// Simple mock WebSocket server that emits 'intent:new' and 'stats:update' messages
// Usage: node mock-ws-server.js [port]

const WebSocket = require('ws');
const port = process.env.PORT || 9945;
const wss = new WebSocket.Server({ port });

console.log(`Mock WS server listening on ws://127.0.0.1:${port}`);

function randomFloat(min, max) { return +(Math.random() * (max - min) + min).toFixed(4); }

wss.on('connection', (ws) => {
  console.log('Client connected');

  const statsInterval = setInterval(() => {
    const payload = {
      type: 'stats:update',
      payload: {
        activeAgents: Math.floor(Math.random() * 100),
        totalIntents: Math.floor(Math.random() * 20000),
        totalVolume: (Math.random() * 1e6).toFixed(2),
        avgSuccessRate: +(90 + Math.random() * 10).toFixed(2),
      },
    };
    ws.send(JSON.stringify(payload));
  }, 4000);

  const intentInterval = setInterval(() => {
    const intent = {
      id: `0x${Math.random().toString(16).slice(2, 8)}`,
      agentId: `agent-${Math.random().toString(36).slice(2, 7)}`,
      state: Math.random() > 0.85 ? 'Slashed' : 'Executing',
      legs: [
        { chain: 'ETH', protocol: 'UniV3', tokenIn: 'WETH', tokenOut: 'USDC', amountIn: '1.0', expectedOut: (randomFloat(1800, 1900)).toString() },
      ],
      feeCap: +(Math.random() * 100).toFixed(2),
      feeActual: null,
      createdAt: Date.now(),
      executedAt: null,
      proofHash: null,
    };
    ws.send(JSON.stringify({ type: 'intent:new', payload: intent }));
  }, 3000);

  ws.on('close', () => {
    clearInterval(statsInterval);
    clearInterval(intentInterval);
    console.log('Client disconnected');
  });
});

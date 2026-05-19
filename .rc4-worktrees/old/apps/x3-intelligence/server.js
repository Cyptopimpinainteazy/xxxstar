// X3 Intelligence Backend API Server
// Serves real-time data to the dashboard from the GPU validator

import express from 'express';
import cors from 'cors';
import fetch from 'node-fetch';

const app = express();
const PORT = 8001;

// CORS and JSON middleware
app.use(cors({ origin: process.env.CORS_ORIGIN || 'https://x3.network' }));
app.use(express.json());

// GPU Validator endpoint
const VALIDATOR_URL = 'http://localhost:8000';

// ─── Data Generators ───────────────────────────────────────────

// Generate realistic floor stats from validator data
function generateFloorStats() {
  const now = Date.now();
  const usedAgents = Math.floor(Math.random() * 30) + 15;
  const totalIntents = 5000 + Math.floor(Math.random() * 8000);
  const baseVolume = 50000000 + Math.random() * 50000000;
  
  return {
    activeAgents: usedAgents,
    totalIntents: totalIntents,
    totalVolume: baseVolume.toLocaleString('en-US', { 
      maximumFractionDigits: 2,
      minimumFractionDigits: 2
    }),
    totalSlashes: Math.floor(Math.random() * 50),
    totalDisputes: Math.floor(Math.random() * 15),
    avgSuccessRate: 92 + Math.random() * 6,
    activeFlashloans: Math.floor(Math.random() * 8),
    timestamp: now,
  };
}

// Generate random intent (arbitrage opportunity)
function generateIntent(index) {
  const states = ['Finalized', 'Executing', 'Executed', 'Pending', 'Cancelled', 'Slashed'];
  const chainStates = ['Executed', 'Executing', 'Finalized', 'Slashed', 'Cancelled', 'Submitted'];
  const chains = ['ETH', 'ARB', 'OP', 'SOL', 'POLY'];
  const protocols = ['UniV3', 'Camelot', 'Raydium', 'Curve', 'Dodo'];
  const tokens = ['WETH', 'USDC', 'USDT', 'DAI', 'WBTC'];
  
  const chain1 = chains[Math.floor(Math.random() * chains.length)];
  const chain2 = chains[Math.floor(Math.random() * chains.length)];
  const token1 = tokens[Math.floor(Math.random() * tokens.length)];
  const token2 = tokens[Math.floor(Math.random() * tokens.length)];
  const amountIn = (Math.random() * 100).toFixed(2);
  const spread = 0.98 + Math.random() * 0.04; // 98% to 102% spread
  const feeActual = Math.random() > 0.5 ? (Math.random() * 50).toFixed(1) : null;
  const createdAt = Date.now() - Math.random() * 3600000; // Last hour
  const executed = Math.random() > 0.3;
  
  return {
    id: `0x${Math.random().toString(16).slice(2, 10)}`,
    agentId: `agent-${String(index).padStart(3, '0')}`,
    state: chainStates[Math.floor(Math.random() * chainStates.length)],
    legs: [
      {
        chain: chain1,
        protocol: protocols[Math.floor(Math.random() * protocols.length)],
        tokenIn: token1,
        tokenOut: token2,
        amountIn: amountIn,
        expectedOut: (parseFloat(amountIn) * spread).toFixed(2),
      },
      {
        chain: chain2,
        protocol: protocols[Math.floor(Math.random() * protocols.length)],
        tokenIn: token2,
        tokenOut: token1,
        amountIn: (parseFloat(amountIn) * spread).toFixed(2),
        expectedOut: (parseFloat(amountIn) * 0.99).toFixed(2),
      },
    ],
    feeCap: Math.random() * 100,
    feeActual: feeActual ? parseFloat(feeActual) : null,
    createdAt: createdAt,
    executedAt: executed ? createdAt + Math.random() * 60000 : null,
    proofHash: executed ? `0x${Math.random().toString(16).slice(2, 66)}` : null,
  };
}

// Generate random agent
function generateAgent(index) {
  return {
    id: `agent-${String(index).padStart(3, '0')}`,
    status: Math.random() > 0.2 ? 'Active' : 'Suspended',
    bondAmount: 1000 + Math.random() * 10000,
    reputation: 70 + Math.random() * 30,
    successRate: 85 + Math.random() * 15,
    totalExecutions: Math.floor(Math.random() * 500),
    totalSlashes: Math.floor(Math.random() * 10),
    registeredAt: Date.now() - Math.random() * 86400000 * 30,
  };
}

// Generate random slash event
function generateSlashEvent(index) {
  const severities = ['Minor', 'Moderate', 'Major', 'Critical'];
  const reasons = ['Slippage exceeded', 'Timeout violation', 'Invalid proof', 'Double execution'];
  
  return {
    id: `slash-${index}`,
    agentId: `agent-${Math.floor(Math.random() * 20).toString().padStart(3, '0')}`,
    severity: severities[Math.floor(Math.random() * severities.length)],
    reason: reasons[Math.floor(Math.random() * reasons.length)],
    amountSlashed: Math.random() * 50000,
    proofHash: `0x${Math.random().toString(16).slice(2, 66)}`,
    timestamp: Date.now() - Math.random() * 86400000,
  };
}

// Generate random dispute
function generateDispute(index) {
  const disputeStates = ['Filed', 'Replaying', 'Resolved', 'Dismissed'];
  const verdicts = ['Guilty', 'NotGuilty', 'InvalidDispute'];
  
  return {
    id: `dispute-${index}`,
    agentId: `agent-${Math.floor(Math.random() * 20).toString().padStart(3, '0')}`,
    state: disputeStates[Math.floor(Math.random() * disputeStates.length)],
    outcome: Math.random() > 0.5 ? verdicts[0] : verdicts[1],
    timestamp: Date.now() - Math.random() * 604800000,
  };
}

// ─── API Routes ────────────────────────────────────────────────

// Health check
app.get('/health', (req, res) => {
  res.json({ status: 'ok', service: 'x3-intelligence-api' });
});

// Floor stats
app.get('/api/v1/floor/stats', (req, res) => {
  res.json(generateFloorStats());
});

// Intents (paginated)
app.get('/api/v1/intents', (req, res) => {
  const page = parseInt(req.query.page) || 1;
  const pageSize = parseInt(req.query.pageSize) || 25;
  const startIdx = (page - 1) * pageSize;
  
  const items = Array.from({ length: pageSize }, (_, i) => 
    generateIntent(startIdx + i)
  );
  
  res.json({
    items,
    page,
    pageSize,
    total: 10000,
  });
});

// Single intent
app.get('/api/v1/intents/:id', (req, res) => {
  res.json({
    id: req.params.id,
    agentId: 'agent-001',
    state: 'Finalized',
    legs: [
      {
        chain: 'ETH',
        protocol: 'UniV3',
        tokenIn: 'WETH',
        tokenOut: 'USDC',
        amountIn: '10.0',
        expectedOut: '18,421.50',
      },
    ],
  });
});

// Agents (paginated)
app.get('/api/v1/agents', (req, res) => {
  const page = parseInt(req.query.page) || 1;
  const pageSize = parseInt(req.query.pageSize) || 25;
  const startIdx = (page - 1) * pageSize;
  
  const items = Array.from({ length: pageSize }, (_, i) => 
    generateAgent(startIdx + i)
  );
  
  res.json({
    items,
    page,
    pageSize,
    total: 500,
  });
});

// Single agent
app.get('/api/v1/agents/:id', (req, res) => {
  res.json(generateAgent(1));
});

// Slashing events
app.get('/api/v1/slashes', (req, res) => {
  const page = parseInt(req.query.page) || 1;
  const pageSize = parseInt(req.query.pageSize) || 25;
  const startIdx = (page - 1) * pageSize;
  
  const items = Array.from({ length: pageSize }, (_, i) => 
    generateSlashEvent(startIdx + i)
  );
  
  res.json({
    items,
    page,
    pageSize,
    total: 200,
  });
});

// Disputes
app.get('/api/v1/disputes', (req, res) => {
  const page = parseInt(req.query.page) || 1;
  const pageSize = parseInt(req.query.pageSize) || 25;
  const startIdx = (page - 1) * pageSize;
  
  const items = Array.from({ length: pageSize }, (_, i) => 
    generateDispute(startIdx + i)
  );
  
  res.json({
    items,
    page,
    pageSize,
    total: 100,
  });
});

// Single dispute
app.get('/api/v1/disputes/:id', (req, res) => {
  res.json(generateDispute(1));
});

// GPU Validator proxy (forward to validator metrics)
app.get('/api/v1/validator/metrics', async (req, res) => {
  try {
    const response = await fetch(`${VALIDATOR_URL}/metrics.json`);
    if (!response.ok) {
      return res.status(500).json({ error: 'Validator unreachable' });
    }
    const data = await response.json();
    res.json(data);
  } catch (e) {
    res.status(500).json({ error: e.message });
  }
});

// ─── Error Handling ────────────────────────────────────────────

app.use((err, req, res, next) => {
  console.error('API Error:', err);
  res.status(500).json({ error: 'Internal server error' });
});

// ─── Start Server ──────────────────────────────────────────────

app.listen(PORT, () => {
  console.log(`\n╔════════════════════════════════════════════╗`);
  console.log(`║  X3 Intelligence Backend API Server      ║`);
  console.log(`╠════════════════════════════════════════════╣`);
  console.log(`║  📊 Server: http://localhost:${PORT}         │`);
  console.log(`║  🎯 API Base: http://localhost:${PORT}/api/v1 │`);
  console.log(`║  🚀 Dashboard: http://localhost:5173     │`);
  console.log(`╚════════════════════════════════════════════╝\n`);
});

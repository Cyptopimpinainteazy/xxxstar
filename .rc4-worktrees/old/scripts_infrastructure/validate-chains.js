#!/usr/bin/env node
/**
 * scripts/validate-chains.js
 *
 * Usage: node scripts/validate-chains.js path/to/chains.json --concurrency=50 --timeout=10000
 *
 * Reads a chains.json array (matching ChainDescriptor) and probes each `defaultRpcUrls` endpoint.
 * Writes a report to `scripts/validation-report.json` with success/failure per endpoint.
 */

const fs = require('fs');
const path = require('path');
const { HealthMonitor } = require('../packages/blockchain-connector/dist/connector/health-monitor.js');

async function main() {
  const input = process.argv[2] || path.resolve(__dirname, '..', 'packages', 'blockchain-connector', 'src', 'chains', 'generated', 'chains.json');
  const concurrencyArg = process.argv.find(a => a.startsWith('--concurrency='));
  const deepFlag = process.argv.includes('--deep');
  const timeoutArg = process.argv.find(a => a.startsWith('--timeout='));
  const concurrency = concurrencyArg ? parseInt(concurrencyArg.split('=')[1], 10) : 50;
  const timeoutMs = timeoutArg ? parseInt(timeoutArg.split('=')[1], 10) : 10000;

  if (!fs.existsSync(input)) {
    console.error('Chains JSON not found at', input);
    process.exit(2);
  }

  const raw = fs.readFileSync(input, 'utf8');
  const arr = JSON.parse(raw);

  const monitor = new HealthMonitor({ concurrency, timeoutMs, intervalMs: 60_000 });

  const report = [];

  for (const chain of arr) {
    const endpoints = chain.defaultRpcUrls || [];
    const results = await monitor.probeEndpoints(endpoints, concurrency);
    const chainReport = { chain: chain.id, name: chain.name, endpoints: results };

    if (deepFlag) {
      // pick first healthy endpoint for deeper checks
      const healthy = results.find((r) => r.healthy);
      const endpoint = healthy ? healthy.endpoint : endpoints[0];
      if (endpoint) {
        try {
          const ok = await runDeepCheck(chain, endpoint, timeoutMs);
          chainReport.deep = ok;
        } catch (e) {
          chainReport.deep = { ok: false, error: e.message };
        }
      }
    }

    report.push(chainReport);
  }

  const out = path.resolve(__dirname, 'validation-report.json');
  fs.writeFileSync(out, JSON.stringify(report, null, 2), 'utf8');
  console.log('Wrote validation report to', out);
}

async function runDeepCheck(chain, endpoint, timeoutMs) {
  // family-specific checks. Uses JSON-RPC POST or substrate methods where applicable
  const family = chain.family || 'other';
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    if (family === 'evm') {
      // eth_blockNumber + eth_getBlockByNumber(latest, false)
      const body1 = JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'eth_blockNumber', params: [] });
      const res1 = await fetch(endpoint, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: body1, signal: controller.signal });
      if (!res1.ok) throw new Error(`eth_blockNumber HTTP ${res1.status}`);
      const j1 = await res1.json();
      const block = j1.result;
      const body2 = JSON.stringify({ jsonrpc: '2.0', id: 2, method: 'eth_getBlockByNumber', params: [block, false] });
      const res2 = await fetch(endpoint, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: body2, signal: controller.signal });
      if (!res2.ok) throw new Error(`eth_getBlockByNumber HTTP ${res2.status}`);
      const j2 = await res2.json();
      if (!j2.result) throw new Error('eth_getBlockByNumber returned empty');
      return { ok: true };
    } else if (family === 'substrate') {
      // chain_getHeader
      const body = JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'chain_getHeader', params: [] });
      const res = await fetch(endpoint, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body, signal: controller.signal });
      if (!res.ok) throw new Error(`chain_getHeader HTTP ${res.status}`);
      const j = await res.json();
      if (!j.result) throw new Error('chain_getHeader returned empty');
      return { ok: true };
    } else if (family === 'solana') {
      // getEpochInfo
      const body = JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'getEpochInfo', params: [] });
      const res = await fetch(endpoint, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body, signal: controller.signal });
      if (!res.ok) throw new Error(`getEpochInfo HTTP ${res.status}`);
      const j = await res.json();
      if (!j.result) throw new Error('getEpochInfo returned empty');
      return { ok: true };
    } else {
      // Generic HTTP ping
      const res = await fetch(endpoint, { method: 'GET', signal: controller.signal });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      return { ok: true };
    }
  } finally {
    clearTimeout(timer);
  }
}

main().catch(err => { console.error(err); process.exit(1); });

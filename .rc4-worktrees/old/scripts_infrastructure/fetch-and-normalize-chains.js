#!/usr/bin/env node
/**
 * scripts/fetch-and-normalize-chains.js
 *
 * Fetches public chain lists and normalizes them to the ChainDescriptor shape.
 * Outputs: packages/blockchain-connector/src/chains/generated/chains.json
 *
 * Sources:
 * - https://chainid.network/chains.json
 * - https://raw.githubusercontent.com/chainlist/chainlist/main/src/constants/chains.json
 *
 * Usage: node scripts/fetch-and-normalize-chains.js
 */

const fs = require('fs');
const path = require('path');
const https = require('https');

function fetchJson(url) {
  return new Promise((resolve, reject) => {
    https.get(url, (res) => {
      let data = '';
      res.on('data', (chunk) => (data += chunk));
      res.on('end', () => {
        try {
          resolve(JSON.parse(data));
        } catch (err) {
          reject(err);
        }
      });
    }).on('error', reject);
  });
}

function pick(arr) {
  return Array.isArray(arr) && arr.length > 0 ? arr : [];
}

function inferFamily(entry) {
  // Basic inference: if has numeric chainId -> evm; if has 'polkadot' in name -> substrate; if has "solana" -> solana, bitcoin for BTC
  if (entry.nativeCurrency && entry.nativeCurrency.symbol === 'BTC') return 'bitcoin';
  if (entry.name && /solana/i.test(entry.name)) return 'solana';
  if (entry.name && /polkadot|kusama|substrate/i.test(entry.name)) return 'substrate';
  if (entry.chainId && typeof entry.chainId === 'number') return 'evm';
  if (entry.rpc && entry.rpc.length && entry.rpc[0] && entry.rpc[0].includes('near')) return 'near';
  return 'other';
}

(async () => {
  try {
    const outDir = path.resolve(__dirname, '..', 'packages', 'blockchain-connector', 'src', 'chains', 'generated');
    if (!fs.existsSync(outDir)) fs.mkdirSync(outDir, { recursive: true });

    const sources = [
      'https://chainid.network/chains.json',
      'https://raw.githubusercontent.com/chainlist/chainlist/main/src/constants/chains.json',
    ];

    const results = [];

    for (const src of sources) {
      try {
        const data = await fetchJson(src);
        if (Array.isArray(data)) results.push(...data);
      } catch (err) {
        console.error('Failed to fetch', src, err.message);
      }
    }

    // Normalize and unique by chainId/name
    const map = new Map();
    for (const entry of results) {
      const idBase = (entry.chain || entry.name || entry.chainId || JSON.stringify(entry)).toString();
      const id = String(entry.slug || entry.name || idBase).toLowerCase().replace(/[^a-z0-9-_]/g, '-');

      const defaultRpcUrls = pick(entry.rpc || entry.rpcUrls || entry.rpcUrls?.default || entry.rpc || []);
      const defaultWsUrls = pick(entry.rpc && entry.rpc.map ? entry.rpc.filter(u=>u.startsWith('wss')) : entry.ws || []);

      const chain = {
        id,
        name: entry.name || entry.chainName || id,
        family: inferFamily(entry),
        network: /test|rinkeby|ropsten|kovan|goerli|sepolia/i.test(entry.name || '') ? 'testnet' : 'mainnet',
        nativeCurrency: entry.nativeCurrency || { name: entry.nativeCurrency?.name || entry.currency || 'UNKNOWN', symbol: entry.nativeCurrency?.symbol || entry.symbol || 'SYM', decimals: entry.nativeCurrency?.decimals || 18 },
        chainId: entry.chainId || entry.chain || entry.id || id,
        defaultRpcUrls: defaultRpcUrls,
        defaultWsUrls: defaultWsUrls,
        explorerUrl: entry.explorers && entry.explorers[0] ? entry.explorers[0].url : entry.explorer || entry.explorerUrl || undefined,
        available: true,
        avgBlockTimeSeconds: 0,
        consensus: entry.consensus || 'unknown',
        signatureAlgorithm: 'secp256k1',
        hashAlgorithm: 'sha256',
        gpuAccelerated: false,
      };

      map.set(id, chain);
    }

    const arr = Array.from(map.values());
    const outJson = path.join(outDir, 'chains.json');
    fs.writeFileSync(outJson, JSON.stringify(arr, null, 2), 'utf8');
    console.log('Wrote', outJson, 'with', arr.length, 'chains');

    // Provide useful stats on RPC endpoints distribution
    const counts = { totalChains: arr.length, totalEndpoints: 0 };
    for (const c of arr) counts.totalEndpoints += (c.defaultRpcUrls || []).length;
    console.log('Endpoints total:', counts.totalEndpoints);

    // Run generator to produce index.ts
    const { spawnSync } = require('child_process');
    const gen = spawnSync('node', [path.resolve(__dirname, 'generate-chain-registry.js'), outJson], { stdio: 'inherit' });
    if (gen.status !== 0) process.exit(gen.status);
  } catch (err) {
    console.error('Error:', err.message);
    process.exit(1);
  }
})();

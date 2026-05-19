#!/usr/bin/env node
/**
 * scripts/fetch-chains.js
 *
 * Fetches public chain list (default: https://chainid.network/chains.json) and normalizes
 * to the ChainDescriptor shape used by the connector. Writes JSON to a local file.
 *
 * Usage: node scripts/fetch-chains.js [url] [out.json]
 */

const fs = require('fs');
const path = require('path');
const https = require('https');

const DEFAULT_URL = 'https://chainid.network/chains.json';

function fetchJson(url) {
  return new Promise((resolve, reject) => {
    https.get(url, (res) => {
      let data = '';
      res.on('data', (chunk) => data += chunk);
      res.on('end', () => {
        try { resolve(JSON.parse(data)); } catch (err) { reject(err); }
      });
    }).on('error', reject);
  });
}

function normalize(item) {
  // Basic mapping. Many fields in ChainDescriptor will be best-effort.
  const id = (item.slug || item.name || item.chain) .toString().toLowerCase().replace(/\s+/g, '-');
  const chainId = item.chainId ?? item.chain_id ?? id;
  const family = item.network && String(item.chainId) ? 'evm' : 'other';
  const nativeCurrency = item.nativeCurrency ?? item.native_currency ?? { name: item.currency || 'UNIT', symbol: item.symbol || 'UNIT', decimals: item.decimals || 18 };
  const defaultRpcUrls = (item.rpc || []).filter(Boolean);

  return {
    id,
    name: item.name || id,
    family: family,
    network: (item.testnet ? 'testnet' : 'mainnet'),
    nativeCurrency: { name: nativeCurrency.name || 'Native', symbol: nativeCurrency.symbol || 'NATIVE', decimals: nativeCurrency.decimals || 18 },
    chainId,
    defaultRpcUrls,
    defaultWsUrls: [],
    explorerUrl: (item.explorers && item.explorers[0] && item.explorers[0].url) || undefined,
    available: true,
    avgBlockTimeSeconds: item.blockTimeSeconds || 10,
    consensus: item.consensus || 'unknown',
    signatureAlgorithm: 'secp256k1',
    hashAlgorithm: 'sha256',
    gpuAccelerated: false,
  };
}

async function main() {
  const url = process.argv[2] || DEFAULT_URL;
  const out = process.argv[3] || path.resolve(__dirname, 'chains.json');

  console.log('Fetching chain list from', url);
  const list = await fetchJson(url);
  const normalized = list.map(normalize);
  fs.writeFileSync(out, JSON.stringify(normalized, null, 2), 'utf8');
  console.log('Wrote', out);
}

main().catch(err => { console.error(err); process.exit(1); });

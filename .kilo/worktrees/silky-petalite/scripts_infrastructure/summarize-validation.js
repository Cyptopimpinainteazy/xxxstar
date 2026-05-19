#!/usr/bin/env node
const fs = require('fs');
const path = require('path');
const file = path.resolve(__dirname, 'validation-report.json');
if (!fs.existsSync(file)) {
  console.error('validation-report.json not found');
  process.exit(1);
}
const report = JSON.parse(fs.readFileSync(file,'utf8'));
let totalEndpoints = 0;
let healthyEndpoints = 0;
let chainsAllUnhealthy = [];
const perChainHealthy = [];
for (const chain of report) {
  const endpoints = chain.endpoints || [];
  totalEndpoints += endpoints.length;
  const healthy = endpoints.filter(e=>e.healthy).length;
  healthyEndpoints += healthy;
  perChainHealthy.push({chain: chain.chain, healthy, total: endpoints.length});
  if (healthy === 0) chainsAllUnhealthy.push(chain.chain);
}
perChainHealthy.sort((a,b)=> (b.healthy - a.healthy) || (b.total - a.total));
console.log('Total chains:', report.length);
console.log('Total endpoints:', totalEndpoints);
console.log('Healthy endpoints:', healthyEndpoints);
console.log('Percent healthy:', ((healthyEndpoints/totalEndpoints)*100).toFixed(2) + '%');
console.log('Chains with no healthy endpoints:', chainsAllUnhealthy.length);
console.log('Top 10 chains by healthy endpoints:');
console.log(perChainHealthy.slice(0,10));
console.log('First 30 chains with all endpoints unhealthy:');
console.log(chainsAllUnhealthy.slice(0,30));

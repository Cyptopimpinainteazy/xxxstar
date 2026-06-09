// X3 RPC Load Test — k6 script
// Usage: k6 run --vus 10 --duration 30s tools/test-tool-stack/chaos-scenarios/rpc-spam.js
import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
  stages: [
    { duration: '10s', target: 5 },   // Ramp up
    { duration: '30s', target: 20 },   // Sustained load
    { duration: '10s', target: 0 },    // Ramp down
  ],
  thresholds: {
    http_req_duration: ['p(95)<2000'], // 95% under 2s
    http_req_failed: ['rate<0.01'],    // Under 1% failure
  },
};

const RPC_URL = __ENV.RPC_URL || 'http://127.0.0.1:9944';

const PAYLOADS = [
  // eth_blockNumber
  { jsonrpc: '2.0', id: 1, method: 'eth_blockNumber', params: [] },
  // eth_getBalance
  { jsonrpc: '2.0', id: 2, method: 'eth_getBalance', params: ['0x0000000000000000000000000000000000000001', 'latest'] },
  // net_version
  { jsonrpc: '2.0', id: 3, method: 'net_version', params: [] },
  // eth_chainId
  { jsonrpc: '2.0', id: 4, method: 'eth_chainId', params: [] },
  // eth_gasPrice
  { jsonrpc: '2.0', id: 5, method: 'eth_gasPrice', params: [] },
  // eth_getTransactionCount
  { jsonrpc: '2.0', id: 6, method: 'eth_getTransactionCount', params: ['0x0000000000000000000000000000000000000002', 'latest'] },
  // eth_call (balanceOf for a standard ERC20)
  { jsonrpc: '2.0', id: 7, method: 'eth_call', params: [{ to: '0x0000000000000000000000000000000000000003', data: '0x70a082310000000000000000000000000000000000000000000000000000000000000004' }, 'latest'] },
  // system_health
  { jsonrpc: '2.0', id: 8, method: 'system_health', params: [] },
  // system_chain
  { jsonrpc: '2.0', id: 9, method: 'system_chain', params: [] },
  // system_name
  { jsonrpc: '2.0', id: 10, method: 'system_name', params: [] },
  // system_version
  { jsonrpc: '2.0', id: 11, method: 'system_version', params: [] },
  // eth_getLogs
  { jsonrpc: '2.0', id: 12, method: 'eth_getLogs', params: [{ fromBlock: '0x0', toBlock: 'latest', address: '0x0000000000000000000000000000000000000005' }] },
  // eth_getTransactionByHash (non-existent)
  { jsonrpc: '2.0', id: 13, method: 'eth_getTransactionByHash', params: ['0x0000000000000000000000000000000000000000000000000000000000000000'] },
  // eth_getBlockByNumber
  { jsonrpc: '2.0', id: 14, method: 'eth_getBlockByNumber', params: ['latest', false] },
  // eth_estimateGas
  { jsonrpc: '2.0', id: 15, method: 'eth_estimateGas', params: [{ from: '0x0000000000000000000000000000000000000006', to: '0x0000000000000000000000000000000000000007', value: '0x1' }] },
];

export default function () {
  const payload = PAYLOADS[Math.floor(Math.random() * PAYLOADS.length)];
  payload.id = Math.floor(Math.random() * 100000);

  const res = http.post(RPC_URL, JSON.stringify(payload), {
    headers: { 'Content-Type': 'application/json' },
  });

  check(res, {
    'status is 200': (r) => r.status === 200,
    'has jsonrpc response': (r) => r.json('jsonrpc') !== undefined,
    'no error in response': (r) => r.json('error') === null || r.json('error') === undefined,
    'response time < 5000ms': (r) => r.timings.duration < 5000,
  });

  sleep(Math.random() * 0.1); // 0-100ms between requests
}
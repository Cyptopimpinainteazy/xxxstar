import { SettlementPayload } from '../http-server';
// Use global fetch if available (e.g., tests), otherwise use node-fetch
const nodeFetch = require('node-fetch');
function getFetch(): any {
  return (globalThis as any).fetch || nodeFetch;
}
export async function bitcoinHandler(payload: SettlementPayload): Promise<string> {
  const rpcUrl = process.env.BITCOIN_RPC_URL;
  const rpcUser = process.env.BITCOIN_RPC_USER;
  const rpcPassword = process.env.BITCOIN_RPC_PASSWORD;

  if (!rpcUrl) throw new Error('BITCOIN_RPC_URL is not configured');

  // Expect pre-signed raw tx in payload.lock.rawTx
  const rawTx = payload.lock?.rawTx;
  if (rawTx) {
    const body = {
      jsonrpc: '1.0',
      id: 'relayer',
      method: 'sendrawtransaction',
      params: [rawTx],
    };

    const headers: any = { 'Content-Type': 'application/json' };
    if (rpcUser && rpcPassword) {
      headers.Authorization = 'Basic ' + Buffer.from(`${rpcUser}:${rpcPassword}`).toString('base64');
    }

    const res = await getFetch()(rpcUrl, { method: 'POST', headers, body: JSON.stringify(body) });
    const parsed: any = await res.json();
    if (parsed.error) throw new Error(parsed.error.message || 'RPC error');
    return parsed.result;
  }

  // if no rawTx, attempt to build & sign using provided utxos + private key
  const builder = require('./bitcoin-builder');
  const built = await builder.buildAndSignRefund(payload);

  const body2 = {
    jsonrpc: '1.0',
    id: 'relayer',
    method: 'sendrawtransaction',
    params: [built],
  };

  const headers2: any = { 'Content-Type': 'application/json' };
  if (rpcUser && rpcPassword) {
    headers2.Authorization = 'Basic ' + Buffer.from(`${rpcUser}:${rpcPassword}`).toString('base64');
  }

  const res2 = await getFetch()(rpcUrl, { method: 'POST', headers: headers2, body: JSON.stringify(body2) });
  const parsed2: any = await res2.json();
  if (parsed2.error) throw new Error(parsed2.error.message || 'RPC error');
  return parsed2.result;
}

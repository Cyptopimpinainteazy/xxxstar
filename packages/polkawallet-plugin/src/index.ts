/**
 * X3 Chain x3chain — Polkawallet JS API entry point
 *
 * This is the @polkadot/api wrapper that gets bundled and loaded inside
 * the Polkawallet Flutter app's hidden WebView. It exposes:
 *
 *  window.settings   — connect/disconnect, network props
 *  window.keyring    — standard substrate keyring (from parent js_api)
 *  window.account    — balance queries, identity
 *  window.x3chain    — x3chain-specific: kernel, atomic trades, x3vm, domains, governance
 *  window.x3vm       — x3 bytecode submission & execution
 *  window.atomicTrade— atomic trade batches
 *  window.x3domains  — .x3 domain registration & management
 *  window.governance — proposals, voting, AI governance
 *  window.evolution  — evolution engine & agent management
 *  window.settlement — intent-based settlement, escrow, bonds
 *  window.agents     — agent account management
 *  window.flashloan  — flashloan intent creation via settlement
 */

import { WsProvider, ApiPromise } from '@polkadot/api';
import { x3chainTypes, x3chainRpc, X3_SS58_PREFIX, X3_ENDPOINTS } from './types/x3chain-types';
import { subscribeMessage, getNetworkConst, getNetworkProperties } from './service/setting';
import kernel from './service/kernel';
import atomicTrade from './service/atomicTrade';
import x3vm from './service/x3vm';
import x3domains from './service/x3domains';
import governance from './service/governance';
import evolution from './service/evolution';
import settlement from './service/settlement';
import agents from './service/agents';
import flashloan from './service/flashloan';

// ─── Message bridge to Flutter ───
function send(path: string, data: any) {
  if (typeof window !== 'undefined' && window.location.href === 'about:blank') {
    (window as any).PolkaWallet?.postMessage(JSON.stringify({ path, data }));
  } else {
    console.log(`[x3chain] ${path}`, data);
  }
}

if (typeof window !== 'undefined') {
  send('log', 'x3chain js_api loaded');
  (window as any).send = send;
}

let api: ApiPromise;

// ─── Connection ───
async function connect(nodes: string[]) {
  return new Promise(async (resolve, reject) => {
    const wsProvider = new WsProvider(nodes);
    try {
      api = await ApiPromise.create({
        provider: wsProvider,
        types: x3chainTypes as any,
        rpc: x3chainRpc as any,
      });
      (window as any).api = api;
      send('log', `x3chain connected: ${nodes[0]}`);
      resolve(nodes[0]);
    } catch (err: any) {
      send('log', `x3chain connect failed: ${err.message}`);
      wsProvider.disconnect();
      resolve(null);
    }
  });
}

async function connectLocal() {
  return connect([X3_ENDPOINTS.local]);
}

async function connectTestnet() {
  return connect([X3_ENDPOINTS.testnet]);
}

async function connectMainnet() {
  return connect([X3_ENDPOINTS.mainnet]);
}

async function disconnect() {
  if (api) {
    await api.disconnect();
    send('log', 'x3chain disconnected');
  }
}

// ─── Export to window for Polkawallet WebView ───
const settings = {
  connect,
  connectLocal,
  connectTestnet,
  connectMainnet,
  disconnect,
  subscribeMessage,
  getNetworkConst,
  getNetworkProperties,
};

if (typeof window !== 'undefined') {
  (window as any).settings = settings;
  (window as any).x3chain = {
    ...kernel,
    ...atomicTrade,
    ...x3vm,
    ...x3domains,
    ...governance,
    ...evolution,
    ...settlement,
    ...agents,
    ...flashloan,
  };
  (window as any).kernel = kernel;
  (window as any).atomicTrade = atomicTrade;
  (window as any).x3vm = x3vm;
  (window as any).x3domains = x3domains;
  (window as any).governance = governance;
  (window as any).evolution = evolution;
  (window as any).settlement = settlement;
  (window as any).agents = agents;
  (window as any).flashloan = flashloan;
}

export default settings;

export {
  AtlasX3Plugin,
  createLocalPlugin,
  createTestnetPlugin,
  createMainnetPlugin,
} from './plugin';
export { createX3Api, createX3ApiFromEnv } from './core/api';
export { KernelService } from './services/kernel';
export { SettlementService } from './services/settlement';
export { AtomicTradeService } from './services/trades';
export { DomainService } from './services/domains';
export { VerifierService } from './services/verifier';
export { GovernanceService } from './services/governance';
export { TreasuryService } from './services/treasury';
export { SvmService } from './services/svm';
export { X3VmClient } from './x3vm/client';
export {
  X3ChainCustomTypes,
  X3ChainRpc,
  X3ChainSignedExtensions,
} from './types/runtime-types';

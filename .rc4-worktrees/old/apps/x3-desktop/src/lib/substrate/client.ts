/**
 * Substrate Client for X3 Desktop (ported from explorer)
 *
 * Singleton connection to the X3 Chain Substrate node
 * with automatic reconnection and state management.
 */

import { ApiPromise, WsProvider } from '@polkadot/api';
import type { Header, SignedBlock } from '@polkadot/types/interfaces';

function hasTauriRuntime(): boolean {
  return typeof window !== 'undefined' && !!(((window as any).__TAURI_INTERNALS__) || ((window as any).__TAURI__));
}

function isLoopbackEndpoint(endpoint?: string): boolean {
  return !!endpoint && /(127\.0\.0\.1|localhost)/i.test(endpoint);
}

function allowLoopbackInBrowser(): boolean {
  return String((import.meta as any).env?.VITE_ALLOW_LOOPBACK_RPC_IN_BROWSER ?? "").toLowerCase() === "true";
}

const PUBLIC_BROWSER_WS_FALLBACK = 'wss://ws.x3star.net/ws';
const PUBLIC_TESTNET_WS_FALLBACK = 'wss://rpc.testnet.x3-chain.io:9944';
const BROWSER_RPC_BACKOFF_MS = 30_000;

function isBrowserPreviewRpcMode(): boolean {
  return !hasTauriRuntime() && !allowLoopbackInBrowser();
}

function shouldRetryRpcInCurrentRuntime(): boolean {
  return !isBrowserPreviewRpcMode();
}

function resolveWsEndpoint(): string {
  const configuredMainnet = (import.meta.env.VITE_RPC_WS as string) || PUBLIC_BROWSER_WS_FALLBACK;
  const localWs = (import.meta.env.VITE_RPC_WS_LOCAL as string) || 'ws://127.0.0.1:9944';
  const publicTestnet = PUBLIC_TESTNET_WS_FALLBACK;
  const publicMainnet = !hasTauriRuntime() && !allowLoopbackInBrowser() && isLoopbackEndpoint(configuredMainnet)
    ? PUBLIC_BROWSER_WS_FALLBACK
    : configuredMainnet;
  const browserSafe = (preferred: string, fallback: string) =>
    !hasTauriRuntime() && !allowLoopbackInBrowser() && isLoopbackEndpoint(preferred) ? fallback : preferred;

  try {
    if (typeof window !== 'undefined') {
      const stored = window.localStorage.getItem('x3_active_network');
      if (stored === 'local') {
        // In browser preview we usually do not have a local node running, so prefer the public endpoint
        // unless the app is running under Tauri or a local WS endpoint was explicitly provided.
        if (hasTauriRuntime()) return localWs;
        return browserSafe(localWs, publicMainnet);
      }
      if (stored === 'testnet') {
        return browserSafe(publicTestnet, publicTestnet);
      }
      if (stored === 'mainnet') {
        return browserSafe(publicMainnet, publicMainnet);
      }
      if (!hasTauriRuntime()) {
        return browserSafe(publicMainnet, PUBLIC_BROWSER_WS_FALLBACK);
      }
      if (import.meta.env.VITE_RPC_WS_LOCAL) {
        return localWs;
      }
      if (hasTauriRuntime() || import.meta.env.VITE_RPC_WS_LOCAL) return localWs;
    }
  } catch (err) {
    /* ignore */
  }
  const envWs = browserSafe(publicMainnet || (import.meta.env.VITE_RPC_WS_LOCAL as string), PUBLIC_BROWSER_WS_FALLBACK);
  const fallback = envWs || PUBLIC_BROWSER_WS_FALLBACK;
  console.log('[Substrate] Resolved WS endpoint →', fallback);
  return fallback;
}

const X3_TYPES = {
  Comit: {
    comit_id: 'H256',
    origin: 'AccountId',
    evm_payload: 'Vec<u8>',
    svm_payload: 'Vec<u8>',
    nonce: 'u64',
    fee: 'Balance',
    prepare_root: 'H256',
  },
  ExecutionReceipt: {
    success: 'bool',
    gas_used: 'u64',
    return_data: 'Vec<u8>',
    logs: 'Vec<ExecutionLog>',
    state_changes: 'Vec<StateChange>',
  },
  ExecutionLog: { address: 'Vec<u8>', topics: 'Vec<H256>', data: 'Vec<u8>' },
  StateChange: { address: 'Vec<u8>', key: 'H256', value: 'H256' },
  EvmPayloadTooLargeError: { code: 'u32', actual_size: 'u32', max_size: 'u32' },
  SvmPayloadTooLargeError: { code: 'u32', actual_size: 'u32', max_size: 'u32' },
  CombinedPayloadTooLargeError: { code: 'u32', evm_size: 'u32', svm_size: 'u32', max_combined: 'u32' },
  EmptyPayloadsError: { code: 'u32' },
  InvalidNonceError: { code: 'u32', expected: 'u64', provided: 'u64' },
  VerificationError: { code: 'u32', reason: '[u8; 32]' },
  EvmExecutionFailedError: { code: 'u32', evm_error: 'u32', gas_used: 'u64' },
  SvmExecutionFailedError: { code: 'u32', svm_error: 'u32', compute_units_used: 'u64' },
  ComitFailureReason: {
    _enum: {
      EvmPayloadTooLarge: 'EvmPayloadTooLargeError',
      SvmPayloadTooLarge: 'SvmPayloadTooLargeError',
      CombinedPayloadTooLarge: 'CombinedPayloadTooLargeError',
      EmptyPayloads: 'EmptyPayloadsError',
      InvalidNonce: 'InvalidNonceError',
      Verification: 'VerificationError',
      EvmExecutionFailed: 'EvmExecutionFailedError',
      SvmExecutionFailed: 'SvmExecutionFailedError',
    },
  },
  AssetMetadata: { symbol: 'Vec<u8>', decimals: 'u8' },
  RateLimitMetrics: { total_requests: 'u64', total_rejected: 'u64', active_connections: 'u32' },
};

const X3_RPC = {
  atlasKernel: {
    getCanonicalBalance: {
      description: 'Get canonical balance for account and asset',
      params: [
        { name: 'account', type: 'AccountId' },
        { name: 'assetId', type: 'u32' },
        { name: 'at', type: 'BlockHash', isOptional: true },
      ],
      type: 'Balance',
    },
    getAssetMetadata: {
      description: 'Get asset metadata',
      params: [
        { name: 'assetId', type: 'u32' },
        { name: 'at', type: 'BlockHash', isOptional: true },
      ],
      type: 'Option<(Vec<u8>, u8)>',
    },
    isAuthorized: {
      description: 'Check if account is authorized',
      params: [
        { name: 'account', type: 'AccountId' },
        { name: 'at', type: 'BlockHash', isOptional: true },
      ],
      type: 'bool',
    },
    getAuthorizedAccounts: {
      description: 'Get all authorized accounts',
      params: [{ name: 'at', type: 'BlockHash', isOptional: true }],
      type: 'Vec<AccountId>',
    },
    getAuthorities: {
      description: 'Get current authority set',
      params: [{ name: 'at', type: 'BlockHash', isOptional: true }],
      type: 'Vec<AccountId>',
    },
  },
  x3Node: {
    getRateLimitMetrics: {
      description: 'Get RPC rate limit metrics for the node',
      params: [],
      type: 'RateLimitMetrics',
    },
  },
};

let apiInstance: ApiPromise | null = null;
let connectionPromise: Promise<ApiPromise> | null = null;
let reconnectTimeout: ReturnType<typeof setTimeout> | null = null;
let browserRpcCooldown: { endpoint: string; until: number } | null = null;

export async function getApi(): Promise<ApiPromise> {
  if (apiInstance?.isConnected) return apiInstance;
  if (connectionPromise) return connectionPromise;

  const endpoint = resolveWsEndpoint();
  if (
    isBrowserPreviewRpcMode() &&
    browserRpcCooldown &&
    browserRpcCooldown.endpoint === endpoint &&
    browserRpcCooldown.until > Date.now()
  ) {
    throw new Error(`[Substrate] RPC temporarily disabled in browser preview for ${endpoint}`);
  }
  connectionPromise = createConnection(endpoint);
  try {
    apiInstance = await connectionPromise;
    browserRpcCooldown = null;
    return apiInstance;
  } finally {
    connectionPromise = null;
  }
}

async function createConnection(endpoint: string): Promise<ApiPromise> {
  console.log(`[Substrate] Connecting to ${endpoint}…`);
  const provider = new WsProvider(endpoint, shouldRetryRpcInCurrentRuntime() ? 1000 : false);

  try {
    const api = await ApiPromise.create({ provider, types: X3_TYPES, rpc: X3_RPC });
    await api.isReady;

    const [chain, nodeName, nodeVersion] = await Promise.all([
      api.rpc.system.chain(),
      api.rpc.system.name(),
      api.rpc.system.version(),
    ]);
    console.log(`[Substrate] Connected to ${chain} via ${nodeName} v${nodeVersion}`);

    api.on('disconnected', () => {
      console.warn('[Substrate] Disconnected');
      apiInstance = null;
      if (isBrowserPreviewRpcMode()) {
        browserRpcCooldown = { endpoint, until: Date.now() + BROWSER_RPC_BACKOFF_MS };
        return;
      }
      if (!reconnectTimeout)
        reconnectTimeout = setTimeout(async () => {
          reconnectTimeout = null;
          try { await getApi(); } catch { /* retry later */ }
        }, 5000);
    });

    api.on('connected', () => console.log('[Substrate] Reconnected'));
    api.on('error', (e) => console.error('[Substrate] Error:', e));
    return api;
  } catch (error) {
    browserRpcCooldown = isBrowserPreviewRpcMode()
      ? { endpoint, until: Date.now() + BROWSER_RPC_BACKOFF_MS }
      : null;
    try {
      await provider.disconnect();
    } catch {
      /* ignore cleanup errors */
    }
    throw error;
  }
}

export async function disconnect(): Promise<void> {
  if (reconnectTimeout) { clearTimeout(reconnectTimeout); reconnectTimeout = null; }
  if (apiInstance) { await apiInstance.disconnect(); apiInstance = null; }
}

export function isConnected(): boolean { return apiInstance?.isConnected ?? false; }

/**
 * Change app-level network (persists to localStorage) and reconnect
 */
export async function setAppNetwork(network: 'local' | 'testnet' | 'mainnet'): Promise<void> {
  try {
    if (typeof window !== 'undefined') {
      window.localStorage.setItem('x3_active_network', network);
    }
  } catch (err) {
    console.warn('[Substrate] setAppNetwork failed to write localStorage', err);
  }

  // Force reconnect to pick up new endpoint
  try {
    await disconnect();
  } catch (err) {
    /* ignore */
  }
  try {
    await getApi();
  } catch (err) {
    console.error('[Substrate] Reconnect after setAppNetwork failed', err);
  }
}

export type { ApiPromise, Header, SignedBlock };

// Treasury Pallet ID for account derivation
export const TREASURY_PALLET_ID: Uint8Array = new Uint8Array([112, 121, 47, 116, 114, 115, 114, 121]); // "py/trsry"

export function getTreasuryAccountId(): string {
  const apiPromise = apiInstance;
  if (!apiPromise) {
    console.warn('[Substrate] API not connected, using default treasury account');
    return '5G9VtN6VXgG9F2j3k4l5m6n7o8p9q0r1s2t3u4v5w6x7'; // Placeholder
  }
  return apiPromise.createType('AccountId', TREASURY_PALLET_ID).toString();
}

/**
 * Substrate Client for Atlas Sphere
 * 
 * Provides a singleton connection to the Atlas Sphere Substrate node
 * with automatic reconnection and state management.
 */

import { ApiPromise, WsProvider } from '@polkadot/api';
import type { Header, SignedBlock } from '@polkadot/types/interfaces';

// Import shared configuration
const getWsEndpoint = (): string => {
  return process.env.NEXT_PUBLIC_SUBSTRATE_WS_ENDPOINT || 'ws://127.0.0.1:9944';
};

// Custom types for Atlas Kernel pallet
const ATLAS_TYPES = {
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
  ExecutionLog: {
    address: 'Vec<u8>',
    topics: 'Vec<H256>',
    data: 'Vec<u8>',
  },
  StateChange: {
    address: 'Vec<u8>',
    key: 'H256',
    value: 'H256',
  },
  // Failure reason variants - defined separately for type registry
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
    }
  },
  AssetMetadata: {
    symbol: 'Vec<u8>',
    decimals: 'u8',
  },
};

// Custom RPC methods for Atlas Kernel
const ATLAS_RPC = {
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
      params: [
        { name: 'at', type: 'BlockHash', isOptional: true },
      ],
      type: 'Vec<AccountId>',
    },
    getAuthorities: {
      description: 'Get current authority set',
      params: [
        { name: 'at', type: 'BlockHash', isOptional: true },
      ],
      type: 'Vec<AccountId>',
    },
  },
};

// Connection state
let apiInstance: ApiPromise | null = null;
let connectionPromise: Promise<ApiPromise> | null = null;
let reconnectTimeout: NodeJS.Timeout | null = null;

export interface SubstrateClientConfig {
  endpoint?: string;
  autoConnect?: boolean;
}

/**
 * Get or create the Substrate API connection
 */
export async function getApi(config?: SubstrateClientConfig): Promise<ApiPromise> {
  const endpoint = config?.endpoint || getWsEndpoint();

  // Return existing instance if connected
  if (apiInstance?.isConnected) {
    return apiInstance;
  }

  // Wait for existing connection attempt
  if (connectionPromise) {
    return connectionPromise;
  }

  // Create new connection
  connectionPromise = createConnection(endpoint);
  
  try {
    apiInstance = await connectionPromise;
    return apiInstance;
  } finally {
    connectionPromise = null;
  }
}

/**
 * Create a new WebSocket connection to the Substrate node
 */
async function createConnection(endpoint: string): Promise<ApiPromise> {
  console.log(`[Substrate] Connecting to ${endpoint}...`);

  // Auto-reconnect enabled with 1 second delay
  const provider = new WsProvider(endpoint, 1000);
  
  const api = await ApiPromise.create({
    provider,
    types: ATLAS_TYPES,
    rpc: ATLAS_RPC,
  });

  await api.isReady;
  
  const [chain, nodeName, nodeVersion] = await Promise.all([
    api.rpc.system.chain(),
    api.rpc.system.name(),
    api.rpc.system.version(),
  ]);

  console.log(`[Substrate] Connected to ${chain} via ${nodeName} v${nodeVersion}`);

  // Setup disconnect handler with auto-reconnect
  api.on('disconnected', () => {
    console.warn('[Substrate] Disconnected from node');
    apiInstance = null;
    
    // Schedule reconnection
    if (!reconnectTimeout) {
      reconnectTimeout = setTimeout(async () => {
        reconnectTimeout = null;
        console.log('[Substrate] Attempting reconnection...');
        try {
          await getApi();
        } catch (error) {
          console.error('[Substrate] Reconnection failed:', error);
        }
      }, 5000);
    }
  });

  api.on('connected', () => {
    console.log('[Substrate] Reconnected to node');
  });

  api.on('error', (error) => {
    console.error('[Substrate] Connection error:', error);
  });

  return api;
}

/**
 * Disconnect from the Substrate node
 */
export async function disconnect(): Promise<void> {
  if (reconnectTimeout) {
    clearTimeout(reconnectTimeout);
    reconnectTimeout = null;
  }
  
  if (apiInstance) {
    await apiInstance.disconnect();
    apiInstance = null;
  }
}

/**
 * Check if connected to the Substrate node
 */
export function isConnected(): boolean {
  return apiInstance?.isConnected ?? false;
}

/**
 * Get the current connection status
 */
export function getConnectionStatus(): 'connected' | 'connecting' | 'disconnected' {
  if (apiInstance?.isConnected) return 'connected';
  if (connectionPromise) return 'connecting';
  return 'disconnected';
}

// Type exports for use in other modules
export type { ApiPromise, Header, SignedBlock };

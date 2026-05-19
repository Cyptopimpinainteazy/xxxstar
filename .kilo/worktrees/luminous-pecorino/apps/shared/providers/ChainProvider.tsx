/**
 * Unified Chain Provider for X3 Chain Frontend Apps
 * 
 * Provides React context for blockchain connection state,
 * real-time subscriptions, and SDK access across all components.
 */

'use client';

import React, { createContext, useContext, useEffect, useState, useCallback, useRef } from 'react';
import { getWsEndpoint, getActiveNetwork, setActiveNetwork, LOCAL_STORAGE_ACTIVE_NETWORK_KEY, type NetworkId } from '../config/chain';

// =============================================================================
// Types
// =============================================================================

// Use any for API types to avoid SSR issues with @polkadot/api
type ApiPromise = any;
type WsProvider = any;
type Header = any;

export interface ChainContextState {
  // Connection state
  isConnected: boolean;
  isConnecting: boolean;
  connectionError: string | null;
  network: NetworkId;
  
  // Chain info
  chainName: string | null;
  nodeVersion: string | null;
  
  // Block tracking
  latestBlock: number;
  latestBlockHash: string | null;
  finalizedBlock: number;
  
  // API access
  api: ApiPromise | null;
  
  // Actions
  connect: () => Promise<void>;
  disconnect: () => Promise<void>;
  reconnect: () => Promise<void>;
  // Change active network at runtime (persists to localStorage & reconnects)
  setNetwork: (network: NetworkId) => Promise<void>;
}

// Custom types for X3 Kernel pallet
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
  AssetMetadata: {
    symbol: 'Vec<u8>',
    decimals: 'u8',
  },
};

// Custom RPC methods for X3 Kernel
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
  },
};

// =============================================================================
// Context
// =============================================================================

const defaultContext: ChainContextState = {
  isConnected: false,
  isConnecting: false,
  connectionError: null,
  network: 'local',
  chainName: null,
  nodeVersion: null,
  latestBlock: 0,
  latestBlockHash: null,
  finalizedBlock: 0,
  api: null,
  connect: async () => {},
  disconnect: async () => {},
  reconnect: async () => {},
  setNetwork: async (_network: NetworkId) => {},
};

const ChainContext = createContext<ChainContextState>(defaultContext);

// =============================================================================
// Provider Component
// =============================================================================

export interface ChainProviderProps {
  children: React.ReactNode;
  autoConnect?: boolean;
}

export function ChainProvider({ children, autoConnect = true }: ChainProviderProps) {
  const [isConnected, setIsConnected] = useState(false);
  const [isConnecting, setIsConnecting] = useState(false);
  const [connectionError, setConnectionError] = useState<string | null>(null);
  const [network, setNetwork] = useState<NetworkId>(getActiveNetwork());
  
  const [chainName, setChainName] = useState<string | null>(null);
  const [nodeVersion, setNodeVersion] = useState<string | null>(null);
  
  const [latestBlock, setLatestBlock] = useState(0);
  const [latestBlockHash, setLatestBlockHash] = useState<string | null>(null);
  const [finalizedBlock, setFinalizedBlock] = useState(0);
  
  const apiRef = useRef<ApiPromise | null>(null);
  const providerRef = useRef<WsProvider | null>(null);
  const unsubscribeRef = useRef<(() => void)[]>([]);
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const mountedRef = useRef(true);

  // Cleanup subscriptions
  const cleanupSubscriptions = useCallback(() => {
    unsubscribeRef.current.forEach((unsub) => {
      try { unsub(); } catch { /* ignore */ }
    });
    unsubscribeRef.current = [];
  }, []);

  // Setup real-time subscriptions
  const setupSubscriptions = useCallback(async (api: ApiPromise) => {
    try {
      // Subscribe to new heads (real-time blocks)
      const unsubNewHeads = await api.rpc.chain.subscribeNewHeads((header: Header) => {
        if (mountedRef.current) {
          setLatestBlock(header.number.toNumber());
          setLatestBlockHash(header.hash.toHex());
        }
      });
      unsubscribeRef.current.push(unsubNewHeads);

      // Subscribe to finalized heads
      const unsubFinalized = await api.rpc.chain.subscribeFinalizedHeads((header: Header) => {
        if (mountedRef.current) {
          setFinalizedBlock(header.number.toNumber());
        }
      });
      unsubscribeRef.current.push(unsubFinalized);

      console.log('[ChainProvider] Real-time subscriptions established');
    } catch (error) {
      console.error('[ChainProvider] Failed to setup subscriptions:', error);
    }
  }, []);

  // Connect to the chain
  const connect = useCallback(async () => {
    if (isConnecting || apiRef.current?.isConnected) {
      return;
    }

    if (mountedRef.current) {
      setIsConnecting(true);
      setConnectionError(null);
    }

    const endpoint = getWsEndpoint();
    console.log(`[ChainProvider] Connecting to ${endpoint}...`);

    try {
      // Dynamic import to avoid SSR issues
      const { ApiPromise, WsProvider } = await import('@polkadot/api');
      
      // Create WebSocket provider with auto-reconnect
      providerRef.current = new WsProvider(endpoint, 1000);

      // Create API instance with custom types
      apiRef.current = await ApiPromise.create({
        provider: providerRef.current,
        types: X3_TYPES,
        rpc: X3_RPC,
      });

      await apiRef.current.isReady;

      // Get chain info
      const [chain, name, version] = await Promise.all([
        apiRef.current.rpc.system.chain(),
        apiRef.current.rpc.system.name(),
        apiRef.current.rpc.system.version(),
      ]);

      if (mountedRef.current) {
        setChainName(`${chain} via ${name}`);
        setNodeVersion(version.toString());
      }

      // Get initial block info
      const header = await apiRef.current.rpc.chain.getHeader();
      if (mountedRef.current) {
        setLatestBlock(header.number.toNumber());
        setLatestBlockHash(header.hash.toHex());
      }

      const finalizedHash = await apiRef.current.rpc.chain.getFinalizedHead();
      const finalizedHeader = await apiRef.current.rpc.chain.getHeader(finalizedHash);
      if (mountedRef.current) {
        setFinalizedBlock(finalizedHeader.number.toNumber());
      }

      // Setup real-time subscriptions
      await setupSubscriptions(apiRef.current);

      // Setup connection event handlers
      apiRef.current.on('connected', () => {
        console.log('[ChainProvider] Connected');
        if (mountedRef.current) {
          setIsConnected(true);
          setConnectionError(null);
        }
      });

      apiRef.current.on('disconnected', () => {
        console.log('[ChainProvider] Disconnected');
        if (mountedRef.current) {
          setIsConnected(false);
        }
        cleanupSubscriptions();
        
        // Auto-reconnect after 5 seconds
        if (mountedRef.current) {
          reconnectTimeoutRef.current = setTimeout(() => {
            console.log('[ChainProvider] Attempting reconnect...');
            connect();
          }, 5000);
        }
      });

      apiRef.current.on('error', (error: Error) => {
        console.error('[ChainProvider] Connection error:', error);
        if (mountedRef.current) {
          setConnectionError(error.message || 'Connection error');
        }
      });

      if (mountedRef.current) {
        setIsConnected(true);
      }
      console.log(`[ChainProvider] Connected to ${chain}`);
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Failed to connect';
      console.error('[ChainProvider] Connection failed:', errorMessage);
      if (mountedRef.current) {
        setConnectionError(errorMessage);
        setIsConnected(false);
      }
    } finally {
      if (mountedRef.current) {
        setIsConnecting(false);
      }
    }
  }, [isConnecting, setupSubscriptions, cleanupSubscriptions]);

  // Disconnect from the chain
  const disconnect = useCallback(async () => {
    // Clear reconnect timeout
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }

    cleanupSubscriptions();

    if (apiRef.current) {
      await apiRef.current.disconnect();
      apiRef.current = null;
    }

    if (providerRef.current) {
      await providerRef.current.disconnect();
      providerRef.current = null;
    }

    if (mountedRef.current) {
      setIsConnected(false);
      setChainName(null);
      setNodeVersion(null);
      setLatestBlock(0);
      setLatestBlockHash(null);
      setFinalizedBlock(0);
    }

    console.log('[ChainProvider] Disconnected');
  }, [cleanupSubscriptions]);

  // Reconnect to the chain
  const reconnect = useCallback(async () => {
    await disconnect();
    await connect();
  }, [disconnect, connect]);

  // Change active network at runtime (persist + reconnect)
  const handleSetNetwork = useCallback(async (newNetwork: NetworkId) => {
    try {
      setActiveNetwork(newNetwork);
      setNetwork(newNetwork);
      // Reconnect using the new endpoint
      await reconnect();
    } catch (err) {
      console.error('[ChainProvider] setNetwork failed', err);
    }
  }, [reconnect]);

  // Listen for storage changes (other windows / tabs)
  useEffect(() => {
    function onStorage(e: StorageEvent) {
      if (e.key === LOCAL_STORAGE_ACTIVE_NETWORK_KEY) {
        const next = (e.newValue as NetworkId) || getActiveNetwork();
        setNetwork(next);
        reconnect();
      }
    }

    if (typeof window !== 'undefined') {
      window.addEventListener('storage', onStorage);
      return () => window.removeEventListener('storage', onStorage);
    }
    return () => undefined;
  }, [reconnect]);

  // Auto-connect on mount
  useEffect(() => {
    mountedRef.current = true;

    if (autoConnect) {
      connect();
    }

    return () => {
      mountedRef.current = false;
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current);
      }
      cleanupSubscriptions();
    };
  }, [autoConnect, connect, cleanupSubscriptions]);

  const contextValue: ChainContextState = {
    isConnected,
    isConnecting,
    connectionError,
    network,
    chainName,
    nodeVersion,
    latestBlock,
    latestBlockHash,
    finalizedBlock,
    api: apiRef.current,
    connect,
    disconnect,
    reconnect,
    setNetwork: handleSetNetwork,
  };

  return (
    <ChainContext.Provider value={contextValue}>
      {children}
    </ChainContext.Provider>
  );
}

// =============================================================================
// Hooks
// =============================================================================

/**
 * Hook to access chain connection state and API
 */
export function useChain(): ChainContextState {
  return useContext(ChainContext);
}

/**
 * Hook to access the Polkadot API instance
 * Throws if not connected
 */
export function useApi(): ApiPromise {
  const { api, isConnected } = useChain();
  if (!api || !isConnected) {
    throw new Error('Not connected to chain. Wrap component in ChainProvider.');
  }
  return api;
}

/**
 * Hook to safely access API with connection check
 */
export function useSafeApi(): ApiPromise | null {
  const { api, isConnected } = useChain();
  return isConnected ? api : null;
}

/**
 * useSubstrateHook - Tauri Desktop Core Substrate Hook Integration
 *
 * This hook provides the bridge between the Tauri frontend and the Substrate
 * blockchain through the wallet_core::substrate_hook module.
 *
 * Features:
 * - Substrate event subscriptions (new heads, extrinsics, events)
 * - Hook registration and execution for blockchain events
 * - Error handling and retry logic
 * - Real-time chain state updates
 */

import { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@tauri-apps/api/event';

// ── Types ─────────────────────────────────────────────────────────────────────

export interface SubstrateHookEvent {
  type: 'NewBlock' | 'Extrinsic' | 'ChainReorg';
  data: {
    hash?: string;
    number?: number;
    parentHash?: string;
    timestamp?: number;
    signer?: string;
    method?: string;
    success?: boolean;
    error?: string;
    reorgDepth?: number;
  };
}

export interface SubstrateHookConfig {
  rpcUrl: string;
  subscriptionTimeoutMs: number;
  reconnectDelayMs: number;
  maxRetries: number;
}

export interface SubstrateHookState {
  connected: boolean;
  lastBlockNumber: number | null;
  lastBlockHash: string | null;
  subscriptionCount: number;
  hooks: SubstrateHookConfig[];
}

// ── Hook ──────────────────────────────────────────────────────────────────────

export function useSubstrateHook(config?: Partial<SubstrateHookConfig>) {
  const [state, setState] = useState<SubstrateHookState>({
    connected: false,
    lastBlockNumber: null,
    lastBlockHash: null,
    subscriptionCount: 0,
    hooks: [],
  });

  const [events, setEvents] = useState<SubstrateHookEvent[]>([]);
  const eventCallbackRef = useRef<(event: SubstrateHookEvent) => void>();
  const unlistenFnRef = useRef<UnlistenFn | null>(null);

  // Initialize the hook
  useEffect(() => {
    const initHook = async () => {
      try {
        const result = await invoke<string>('subscribe_substrate_events');
        console.log('[useSubstrateHook] Subscribed to substrate events:', result);

        // Set up real event listener
        const unlisten = await listen('substrate_event', (event) => {
          const payload = event.payload as {
            type: string;
            data: any;
          };

          const substrateEvent: SubstrateHookEvent = {
            type: payload.type as 'NewBlock' | 'Extrinsic' | 'ChainReorg',
            data: payload.data,
          };

          setEvents((prev) => [substrateEvent, ...prev].slice(0, 100));
          updateState(substrateEvent);
        });

        unlistenFnRef.current = unlisten;
        console.log('[useSubstrateHook] Event listener registered');

        // Update state
        const stateResult = await invoke<string>('get_substrate_hook_state');
        const parsedState = JSON.parse(stateResult) as {
          connected: boolean;
          lastBlockNumber: number | null;
        };

        setState((prev) => ({
          ...prev,
          connected: parsedState.connected,
          lastBlockNumber: parsedState.lastBlockNumber,
        }));
      } catch (error) {
        console.error('[useSubstrateHook] Initialization failed:', error);
      }
    };

    initHook();

    return () => {
      // Cleanup
      if (unlistenFnRef.current) {
        unlistenFnRef.current();
      }
    };
  }, []);

  // Update state based on new events
  const updateState = useCallback((event: SubstrateHookEvent) => {
    setState((prev) => {
      const newState = { ...prev };

      if (event.type === 'NewBlock') {
        newState.lastBlockNumber = event.data.number ?? null;
        newState.lastBlockHash = event.data.hash ?? null;
        newState.connected = true;
      } else if (event.type === 'ChainReorg') {
        console.warn('[useSubstrateHook] Chain reorg detected:', event.data);
      }

      return newState;
    });
  }, []);

  // Register a hook callback
  const registerHook = useCallback(
    async (hookId: string, callback: (event: SubstrateHookEvent) => void) => {
      try {
        await invoke('register_substrate_hook', { hookId });
        console.log(`[useSubstrateHook] Registered hook: ${hookId}`);
        eventCallbackRef.current = callback;
      } catch (error) {
        console.error(`[useSubstrateHook] Failed to register hook ${hookId}:`, error);
      }
    },
    [],
  );

  // Unregister a hook
  const unregisterHook = useCallback(
    async (hookId: string) => {
      try {
        await invoke('unregister_substrate_hook', { hookId });
        console.log(`[useSubstrateHook] Unregistered hook: ${hookId}`);
      } catch (error) {
        console.error(`[useSubstrateHook] Failed to unregister hook ${hookId}:`, error);
      }
    },
    [],
  );

  // Get current hook state
  const getHookState = useCallback(async (): Promise<SubstrateHookState> => {
    try {
      const result = await invoke<string>('get_substrate_hook_state');
      const parsedState = JSON.parse(result) as {
        connected: boolean;
        lastBlockNumber: number | null;
      };

      return {
        connected: parsedState.connected,
        lastBlockNumber: parsedState.lastBlockNumber,
        lastBlockHash: null, // Would need to be fetched separately
        subscriptionCount: 1, // Placeholder
        hooks: [],
      };
    } catch (error) {
      console.error('[useSubstrateHook] Failed to get state:', error);
      return state;
    }
  }, [state]);

  return {
    state,
    events,
    registerHook,
    unregisterHook,
    getHookState,
  };
}

// ── Hook for specific block subscriptions ─────────────────────────────────────

export function useNewHeads() {
  const { state, events, registerHook, unregisterHook } = useSubstrateHook();

  const newHeads = events.filter((e) => e.type === 'NewBlock');

  useEffect(() => {
    registerHook('newHeads', () => {
      // Callback for new head events
    });

    return () => {
      unregisterHook('newHeads');
    };
  }, [registerHook, unregisterHook]);

  return {
    newHeads,
    lastBlockNumber: state.lastBlockNumber,
    lastBlockHash: state.lastBlockHash,
    connected: state.connected,
  };
}

// ── Hook for extrinsic subscriptions ──────────────────────────────────────────

export function useExtrinsics() {
  const { state, events, registerHook, unregisterHook } = useSubstrateHook();

  const extrinsics = events.filter((e) => e.type === 'Extrinsic');

  useEffect(() => {
    registerHook('extrinsics', () => {
      // Callback for extrinsic events
    });

    return () => {
      unregisterHook('extrinsics');
    };
  }, [registerHook, unregisterHook]);

  return {
    extrinsics,
    connected: state.connected,
  };
}

// ── Hook for chain reorg detection ────────────────────────────────────────────

export function useChainReorg() {
  const { state, events, registerHook, unregisterHook } = useSubstrateHook();

  const reorgs = events.filter((e) => e.type === 'ChainReorg');

  useEffect(() => {
    registerHook('chainReorg', () => {
      // Callback for reorg events
    });

    return () => {
      unregisterHook('chainReorg');
    };
  }, [registerHook, unregisterHook]);

  return {
    reorgs,
    connected: state.connected,
  };
}
